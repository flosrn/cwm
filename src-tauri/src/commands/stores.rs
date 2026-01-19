use serde_json::Value;

use crate::commands::git::{git_auto_commit, git_auto_commit_managed, git_current_branch, git_has_changes, git_is_repo, git_switch_branch_ref};
use crate::commands::utils::{get_app_config_dir, get_home_dir, get_stores_file, read_stores, write_stores};
use crate::commands::workspace::{
    clear_claude_dir_for_switch, copy_claude_to_workspace, copy_workspace_to_claude,
    count_workspace_items, sync_workspace_content,
};
use crate::commands::updates::unlock_cc_ext;

// ============================================================================
// TYPES
// ============================================================================

/// Workspace type enum
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceType {
    SettingsOnly,   // Original CC Mate behavior: only merge settings.json
    FullDirectory,  // New: switch complete ~/.claude directory
}

impl Default for WorkspaceType {
    fn default() -> Self {
        WorkspaceType::SettingsOnly
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ConfigStore {
    pub id: String,
    pub title: String,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    pub settings: Value,
    pub using: bool,

    // Workspace support
    #[serde(rename = "workspaceType", default)]
    pub workspace_type: WorkspaceType,
    #[serde(rename = "workspacePath", skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    #[serde(rename = "includeScripts", default)]
    pub include_scripts: bool,

    // Metadata for full directory workspaces
    #[serde(rename = "skillsCount", skip_serializing_if = "Option::is_none")]
    pub skills_count: Option<u32>,
    #[serde(rename = "commandsCount", skip_serializing_if = "Option::is_none")]
    pub commands_count: Option<u32>,
    #[serde(rename = "agentsCount", skip_serializing_if = "Option::is_none")]
    pub agents_count: Option<u32>,
    #[serde(rename = "pluginsCount", skip_serializing_if = "Option::is_none")]
    pub plugins_count: Option<u32>,
    #[serde(rename = "lastSynced", skip_serializing_if = "Option::is_none")]
    pub last_synced: Option<u64>,

    // Git import source tracking
    #[serde(rename = "sourceUrl", skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct NotificationSettings {
    pub enable: bool,
    pub enabled_hooks: Vec<String>,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        NotificationSettings {
            enable: true,
            enabled_hooks: vec!["Notification".to_string()],
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct StoresData {
    pub configs: Vec<ConfigStore>,
    pub distinct_id: Option<String>,
    pub notification: Option<NotificationSettings>,
}

impl Default for StoresData {
    fn default() -> Self {
        StoresData {
            configs: vec![],
            distinct_id: None,
            notification: Some(NotificationSettings::default()),
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Sanitize a title for use as a git branch name
fn sanitize_branch_name(title: &str) -> String {
    title
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else if c == ' ' {
                '-'
            } else {
                '_'
            }
        })
        .collect::<String>()
        // Remove consecutive dashes/underscores
        .split(|c| c == '-' || c == '_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

#[tauri::command]
pub async fn get_stores() -> Result<Vec<ConfigStore>, String> {
    let stores_file = get_stores_file()?;

    if !stores_file.exists() {
        return Ok(vec![]);
    }

    let mut stores_data = read_stores()?;

    // Add default notification settings if they don't exist
    let mut needs_save = false;
    if stores_data.notification.is_none() {
        stores_data.notification = Some(NotificationSettings::default());
        needs_save = true;
        println!("Added default notification settings to existing stores.json");
    }

    // Auto-refresh counts for FullDirectory workspaces missing any count (migration)
    for store in stores_data.configs.iter_mut() {
        let needs_refresh = store.workspace_type == WorkspaceType::FullDirectory
            && store.workspace_path.is_some()
            && (store.skills_count.is_none()
                || store.commands_count.is_none()
                || store.agents_count.is_none()
                || store.plugins_count.is_none());

        if needs_refresh {
            if let Some(ref workspace_path) = store.workspace_path {
                if let Ok((skills, commands, agents, plugins)) = count_workspace_items(workspace_path) {
                    store.skills_count = skills;
                    store.commands_count = commands;
                    store.agents_count = agents;
                    store.plugins_count = plugins;
                    needs_save = true;
                    println!(
                        "Auto-refreshed counts for workspace: {} (skills: {:?}, cmds: {:?}, agents: {:?}, plugins: {:?})",
                        store.id, skills, commands, agents, plugins
                    );
                }
            }
        }
    }

    // Write back if any changes were made
    if needs_save {
        write_stores(&stores_data)?;
    }

    let mut stores_vec = stores_data.configs;
    // Sort by createdAt in ascending order (oldest first)
    stores_vec.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    Ok(stores_vec)
}

#[tauri::command]
pub async fn get_store(store_id: String) -> Result<ConfigStore, String> {
    let stores = get_stores().await?;
    stores
        .into_iter()
        .find(|store| store.id == store_id)
        .ok_or_else(|| format!("Store with id '{}' not found", store_id))
}

#[tauri::command]
pub async fn get_current_store() -> Result<Option<ConfigStore>, String> {
    let stores = get_stores().await?;
    Ok(stores.into_iter().find(|store| store.using))
}

#[tauri::command]
pub async fn create_config(
    id: String,
    title: String,
    settings: Value,
    workspace_type: Option<String>,
    include_scripts: Option<bool>,
) -> Result<ConfigStore, String> {
    let home_dir = get_home_dir()?;
    let app_config_path = get_app_config_dir()?;

    // Parse workspace type
    let ws_type = match workspace_type.as_deref() {
        Some("full_directory") => WorkspaceType::FullDirectory,
        _ => WorkspaceType::SettingsOnly,
    };
    let include_scripts_flag = include_scripts.unwrap_or(false);

    // Ensure app config directory exists
    std::fs::create_dir_all(&app_config_path)
        .map_err(|e| format!("Failed to create app config directory: {}", e))?;

    // Read existing stores
    let mut stores_data = read_stores()?;

    // Determine if this should be the active store (true if no other stores exist)
    let should_be_active = stores_data.configs.is_empty();

    // If this is the first config being created and there's an existing settings.json, create an Original Config store
    if should_be_active {
        let claude_settings_path = home_dir.join(".claude/settings.json");
        if claude_settings_path.exists() {
            // Read existing settings
            let settings_content = std::fs::read_to_string(&claude_settings_path)
                .map_err(|e| format!("Failed to read existing Claude settings: {}", e))?;

            let settings_json: Value = serde_json::from_str(&settings_content)
                .map_err(|e| format!("Failed to parse existing Claude settings: {}", e))?;

            // Create an Original Config store with existing settings
            let (orig_ws_path, orig_skills, orig_commands, orig_agents, orig_plugins) =
                if ws_type == WorkspaceType::FullDirectory {
                    let orig_id = nanoid::nanoid!(6);
                    let path = copy_claude_to_workspace(&orig_id, include_scripts_flag)?;
                    let (s, c, a, p) = count_workspace_items(&path)?;
                    (Some(path), s, c, a, p)
                } else {
                    (None, None, None, None, None)
                };

            let original_store = ConfigStore {
                id: nanoid::nanoid!(6),
                title: "Original Config".to_string(),
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| format!("Failed to get timestamp: {}", e))?
                    .as_secs(),
                settings: settings_json,
                using: false,
                workspace_type: ws_type.clone(),
                workspace_path: orig_ws_path,
                include_scripts: include_scripts_flag,
                skills_count: orig_skills,
                commands_count: orig_commands,
                agents_count: orig_agents,
                plugins_count: orig_plugins,
                last_synced: Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|e| format!("Failed to get timestamp: {}", e))?
                        .as_secs(),
                ),
                source_url: None,
            };

            stores_data.configs.push(original_store);
            println!("Created Original Config store from existing settings.json");
        }
    }

    // If this is the first store (and therefore active), write its settings to the user's actual settings.json
    if should_be_active {
        let user_settings_path = home_dir.join(".claude/settings.json");

        // Create .claude directory if it doesn't exist
        if let Some(parent) = user_settings_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create .claude directory: {}", e))?;
        }

        // Read existing settings if file exists, otherwise start with empty object
        let mut existing_settings = if user_settings_path.exists() {
            let content = std::fs::read_to_string(&user_settings_path)
                .map_err(|e| format!("Failed to read existing settings: {}", e))?;
            serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse existing settings: {}", e))?
        } else {
            serde_json::Value::Object(serde_json::Map::new())
        };

        // Merge the new settings into existing settings (partial update)
        if let Some(settings_obj) = settings.as_object() {
            if let Some(existing_obj) = existing_settings.as_object_mut() {
                for (key, value) in settings_obj {
                    existing_obj.insert(key.clone(), value.clone());
                }
            } else {
                existing_settings = settings.clone();
            }
        } else {
            existing_settings = settings.clone();
        }

        // Write the merged settings back to file
        let json_content = serde_json::to_string_pretty(&existing_settings)
            .map_err(|e| format!("Failed to serialize merged settings: {}", e))?;

        std::fs::write(&user_settings_path, json_content)
            .map_err(|e| format!("Failed to write user settings: {}", e))?;
    }

    // For full directory workspace, copy current ~/.claude
    let (workspace_path, skills_count, commands_count, agents_count, plugins_count) =
        if ws_type == WorkspaceType::FullDirectory {
            let path = copy_claude_to_workspace(&id, include_scripts_flag)?;
            let (s, c, a, p) = count_workspace_items(&path)?;
            (Some(path), s, c, a, p)
        } else {
            (None, None, None, None, None)
        };

    // Create new store
    let new_store = ConfigStore {
        id: id.clone(),
        title: title.clone(),
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("Failed to get timestamp: {}", e))?
            .as_secs(),
        settings,
        using: should_be_active,
        workspace_type: ws_type,
        workspace_path,
        include_scripts: include_scripts_flag,
        skills_count,
        commands_count,
        agents_count,
        plugins_count,
        last_synced: Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| format!("Failed to get timestamp: {}", e))?
                .as_secs(),
        ),
        source_url: None,
    };

    // Add store to collection
    stores_data.configs.push(new_store.clone());

    // Write back to stores file
    write_stores(&stores_data)?;

    // Automatically unlock CC extension when creating new config
    if let Err(e) = unlock_cc_ext().await {
        eprintln!("Warning: Failed to unlock CC extension: {}", e);
    }

    Ok(new_store)
}

#[tauri::command]
pub async fn update_config(
    store_id: String,
    title: String,
    settings: Value,
) -> Result<ConfigStore, String> {
    let home_dir = get_home_dir()?;
    let mut stores_data = read_stores()?;

    if stores_data.configs.is_empty() {
        return Err("No stores found".to_string());
    }

    // Find the store by ID
    let store_index = stores_data
        .configs
        .iter()
        .position(|store| store.id == store_id)
        .ok_or_else(|| format!("Store with id '{}' not found", store_id))?;

    // Update the store
    let store = &mut stores_data.configs[store_index];
    store.title = title.clone();
    store.settings = settings.clone();

    // If this store is currently in use, also update the user's settings.json with partial update
    if store.using {
        let user_settings_path = home_dir.join(".claude/settings.json");

        // Create .claude directory if it doesn't exist
        if let Some(parent) = user_settings_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create .claude directory: {}", e))?;
        }

        // Read existing settings if file exists, otherwise start with empty object
        let mut existing_settings = if user_settings_path.exists() {
            let content = std::fs::read_to_string(&user_settings_path)
                .map_err(|e| format!("Failed to read existing settings: {}", e))?;
            serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse existing settings: {}", e))?
        } else {
            serde_json::Value::Object(serde_json::Map::new())
        };

        // Merge the new settings into existing settings (partial update)
        if let Some(settings_obj) = settings.as_object() {
            if let Some(existing_obj) = existing_settings.as_object_mut() {
                for (key, value) in settings_obj {
                    existing_obj.insert(key.clone(), value.clone());
                }
            } else {
                existing_settings = settings.clone();
            }
        } else {
            existing_settings = settings.clone();
        }

        // Write the merged settings back to file
        let json_content = serde_json::to_string_pretty(&existing_settings)
            .map_err(|e| format!("Failed to serialize merged settings: {}", e))?;

        std::fs::write(&user_settings_path, json_content)
            .map_err(|e| format!("Failed to write user settings: {}", e))?;
    }

    // Write back to stores file
    write_stores(&stores_data)?;

    // Automatically unlock CC extension when updating config
    if let Err(e) = unlock_cc_ext().await {
        eprintln!("Warning: Failed to unlock CC extension: {}", e);
    }

    Ok(stores_data.configs[store_index].clone())
}

#[tauri::command]
pub async fn delete_config(store_id: String) -> Result<(), String> {
    let mut stores_data = read_stores()?;

    if stores_data.configs.is_empty() {
        return Err("No stores found".to_string());
    }

    // Find and remove store by ID
    let original_len = stores_data.configs.len();
    stores_data.configs.retain(|store| store.id != store_id);

    if stores_data.configs.len() == original_len {
        return Err("Store not found".to_string());
    }

    // Write back to file
    write_stores(&stores_data)?;

    Ok(())
}

#[tauri::command]
pub async fn set_using_config(store_id: String) -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let mut stores_data = read_stores()?;

    if stores_data.configs.is_empty() {
        return Err("No stores found".to_string());
    }

    // Find the store and check if it exists
    let selected_store = stores_data
        .configs
        .iter()
        .find(|store| store.id == store_id)
        .cloned()
        .ok_or("Store not found")?;

    // Handle switching based on workspace type
    match selected_store.workspace_type {
        WorkspaceType::FullDirectory => {
            println!(
                "Switching to full directory workspace: {}",
                selected_store.title
            );

            let workspace_path = selected_store
                .workspace_path
                .as_ref()
                .ok_or("Workspace path not found for full directory workspace")?;

            // Try to auto-commit git changes, but don't block if it fails
            if let Ok(true) = git_is_repo() {
                if let Ok(true) = git_has_changes() {
                    if let Ok(branch) = git_current_branch() {
                        match git_auto_commit(&format!("Auto-save before switching to {}", store_id))
                        {
                            Ok(_) => println!("Auto-committed changes on branch: {}", branch),
                            Err(e) => {
                                println!("Warning: Could not auto-commit (non-blocking): {}", e)
                            }
                        }
                    }
                }
            }

            // Use file-copy switching (safer with Ghostty + Claude Code)
            // Adding delays between operations to let file watchers stabilize
            println!("Using file-copy switching for compatibility (with delays)");

            // 1. Sync current workspace before switching (save current state)
            if let Some(current_store) = stores_data.configs.iter().find(|s| s.using) {
                if current_store.workspace_type == WorkspaceType::FullDirectory {
                    if let Some(current_ws_path) = &current_store.workspace_path {
                        println!(
                            "Syncing current workspace before switch: {}",
                            current_store.title
                        );
                        let _ = sync_workspace_content(current_ws_path, current_store.include_scripts);
                    }
                }
            }

            // Small delay to let file watchers settle after sync
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            // 2. Clear ~/.claude (managed items only)
            clear_claude_dir_for_switch()?;
            println!("Cleared ~/.claude for switch");

            // Delay after clearing to let watchers process deletions
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // 3. Copy target workspace to ~/.claude
            copy_workspace_to_claude(workspace_path)?;
            println!("Restored workspace from: {}", workspace_path);

            // Delay after copying to let watchers process new files
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            // 4. Switch git branch reference (without checkout) to keep git in sync
            if let Ok(true) = git_is_repo() {
                let sanitized_title = sanitize_branch_name(&selected_store.title);
                let branch_name = format!("workspace/{}", sanitized_title);
                match git_switch_branch_ref(&branch_name) {
                    Ok(_) => println!("Git branch ref switched to: {}", branch_name),
                    Err(e) => println!(
                        "Warning: Could not switch git branch ref (non-blocking): {}",
                        e
                    ),
                }

                // 5. Auto-commit only managed workspace files (not user's WIP)
                match git_auto_commit_managed(&format!("Restore workspace: {}", selected_store.title)) {
                    Ok(_) => println!("Auto-committed managed workspace files"),
                    Err(e) => println!("Warning: Could not auto-commit managed files: {}", e),
                }
            }
        }
        WorkspaceType::SettingsOnly => {
            println!(
                "Switching to settings-only workspace: {}",
                selected_store.title
            );

            // Original CC Mate behavior: merge settings.json
            let user_settings_path = home_dir.join(".claude/settings.json");

            // Create .claude directory if it doesn't exist
            if let Some(parent) = user_settings_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create .claude directory: {}", e))?;
            }

            // Read existing settings if file exists, otherwise start with empty object
            let mut existing_settings = if user_settings_path.exists() {
                let content = std::fs::read_to_string(&user_settings_path)
                    .map_err(|e| format!("Failed to read existing settings: {}", e))?;
                serde_json::from_str(&content)
                    .map_err(|e| format!("Failed to parse existing settings: {}", e))?
            } else {
                serde_json::Value::Object(serde_json::Map::new())
            };

            // Merge the new settings into existing settings (partial update)
            if let Some(settings_obj) = selected_store.settings.as_object() {
                if let Some(existing_obj) = existing_settings.as_object_mut() {
                    for (key, value) in settings_obj {
                        existing_obj.insert(key.clone(), value.clone());
                    }
                } else {
                    existing_settings = selected_store.settings.clone();
                }
            } else {
                existing_settings = selected_store.settings.clone();
            }

            // Write the merged settings back to file
            let json_content = serde_json::to_string_pretty(&existing_settings)
                .map_err(|e| format!("Failed to serialize merged settings: {}", e))?;

            std::fs::write(&user_settings_path, json_content)
                .map_err(|e| format!("Failed to write user settings: {}", e))?;
        }
    }

    // Update the using flag for all stores
    for store in &mut stores_data.configs {
        store.using = store.id == store_id;
    }

    // Write back to stores file
    write_stores(&stores_data)?;

    println!("✅ Workspace switch completed successfully");
    Ok(())
}

#[tauri::command]
pub async fn reset_to_original_config() -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let stores_file = get_stores_file()?;

    // Set all stores to not using
    if stores_file.exists() {
        let mut stores_data = read_stores()?;

        // Set all stores to not using
        for store in &mut stores_data.configs {
            store.using = false;
        }

        // Write back to stores file
        write_stores(&stores_data)?;
    }

    // Clear env field in settings.json
    let user_settings_path = home_dir.join(".claude/settings.json");

    // Create .claude directory if it doesn't exist
    if let Some(parent) = user_settings_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create .claude directory: {}", e))?;
    }

    // Read existing settings if file exists, otherwise start with empty object
    let mut existing_settings = if user_settings_path.exists() {
        let content = std::fs::read_to_string(&user_settings_path)
            .map_err(|e| format!("Failed to read existing settings: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse existing settings: {}", e))?
    } else {
        serde_json::Value::Object(serde_json::Map::new())
    };

    // Set env to empty object
    if let Some(existing_obj) = existing_settings.as_object_mut() {
        existing_obj.insert("env".to_string(), serde_json::json!({}));
    }

    // Write the merged settings back to file
    let json_content = serde_json::to_string_pretty(&existing_settings)
        .map_err(|e| format!("Failed to serialize merged settings: {}", e))?;

    std::fs::write(&user_settings_path, json_content)
        .map_err(|e| format!("Failed to write user settings: {}", e))?;

    Ok(())
}
