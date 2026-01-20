use fs_extra::dir::{copy as copy_dir, CopyOptions};

use crate::commands::git::{git_auto_commit, git_has_changes, git_is_repo};
use crate::commands::stores::{ConfigStore, WorkspaceType};
use crate::commands::utils::{
    get_claude_dir, get_stores_file, get_workspaces_dir, read_stores, should_exclude,
    write_stores,
};

// ============================================================================
// CHERRY-PICK TYPES
// ============================================================================

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceItemType {
    Skill,
    Command,
    Agent,
    Hook,
    Plugin,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct WorkspaceItem {
    pub name: String,
    #[serde(rename = "relativePath")]
    pub relative_path: String,
    #[serde(rename = "itemType")]
    pub item_type: WorkspaceItemType,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct WorkspaceItems {
    pub skills: Vec<WorkspaceItem>,
    pub commands: Vec<WorkspaceItem>,
    pub agents: Vec<WorkspaceItem>,
    pub hooks: Vec<WorkspaceItem>,
    pub plugins: Vec<WorkspaceItem>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct CopyItemsResult {
    #[serde(rename = "copiedCount")]
    pub copied_count: u32,
    #[serde(rename = "failedItems")]
    pub failed_items: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct WorkspaceSettings {
    pub content: serde_json::Value,
    #[serde(rename = "referencedFiles")]
    pub referenced_files: Vec<String>,
    pub exists: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SettingsMergeMode {
    Replace,
    Merge,
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// List skill directories (directories containing SKILL.md) recursively
fn list_skill_items(dir_path: &std::path::Path, base_path: &std::path::Path) -> Vec<WorkspaceItem> {
    let mut items = Vec::new();
    if !dir_path.exists() {
        return items;
    }

    if let Ok(entries) = std::fs::read_dir(dir_path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                // Skip hidden directories
                if !name_str.starts_with('.') {
                    // Check if this directory contains SKILL.md
                    if path.join("SKILL.md").exists() {
                        let relative = path.strip_prefix(base_path).unwrap_or(&path);
                        items.push(WorkspaceItem {
                            name: name_str.to_string(),
                            relative_path: relative.to_string_lossy().to_string(),
                            item_type: WorkspaceItemType::Skill,
                        });
                    }
                    // Also check subdirectories
                    items.extend(list_skill_items(&path, base_path));
                }
            }
        }
    }
    items
}

/// List .md files in a directory recursively (for commands, agents, hooks)
fn list_md_items(
    dir_path: &std::path::Path,
    base_path: &std::path::Path,
    item_type: WorkspaceItemType,
) -> Vec<WorkspaceItem> {
    let mut items = Vec::new();
    if !dir_path.exists() {
        return items;
    }

    if let Ok(entries) = std::fs::read_dir(dir_path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().map(|ext| ext == "md").unwrap_or(false) {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                let relative = path.strip_prefix(base_path).unwrap_or(&path);
                items.push(WorkspaceItem {
                    name: name_str.trim_end_matches(".md").to_string(),
                    relative_path: relative.to_string_lossy().to_string(),
                    item_type: item_type.clone(),
                });
            } else if path.is_dir() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                // Skip hidden directories, recurse into others
                if !name_str.starts_with('.') {
                    items.extend(list_md_items(&path, base_path, item_type.clone()));
                }
            }
        }
    }
    items
}

/// List plugin directories (plugins/local subdirectories and direct subdirs)
fn list_plugin_items(dir_path: &std::path::Path, base_path: &std::path::Path) -> Vec<WorkspaceItem> {
    let mut items = Vec::new();
    if !dir_path.exists() {
        return items;
    }

    // Check plugins/local first
    let local_plugins = dir_path.join("local");
    if local_plugins.exists() {
        if let Ok(entries) = std::fs::read_dir(&local_plugins) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if path.is_dir() && !name_str.starts_with('.') {
                    let relative = path.strip_prefix(base_path).unwrap_or(&path);
                    items.push(WorkspaceItem {
                        name: name_str.to_string(),
                        relative_path: relative.to_string_lossy().to_string(),
                        item_type: WorkspaceItemType::Plugin,
                    });
                }
            }
        }
    }

    // Also list direct subdirectories of plugins/ that aren't "local"
    if let Ok(entries) = std::fs::read_dir(dir_path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if path.is_dir() && !name_str.starts_with('.') && name_str != "local" {
                let relative = path.strip_prefix(base_path).unwrap_or(&path);
                items.push(WorkspaceItem {
                    name: name_str.to_string(),
                    relative_path: relative.to_string_lossy().to_string(),
                    item_type: WorkspaceItemType::Plugin,
                });
            }
        }
    }

    items
}

/// Count .md files in a directory recursively (for commands, agents)
fn count_directory_items(dir_path: &std::path::Path) -> u32 {
    if !dir_path.exists() {
        return 0;
    }

    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir_path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().map(|ext| ext == "md").unwrap_or(false) {
                count += 1;
            } else if path.is_dir() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                // Skip hidden directories, recurse into others
                if !name_str.starts_with('.') {
                    count += count_directory_items(&path);
                }
            }
        }
    }
    count
}

/// Count subdirectories containing SKILL.md recursively (for skills)
fn count_skill_directories(dir_path: &std::path::Path) -> u32 {
    if !dir_path.exists() {
        return 0;
    }

    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir_path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                // Skip hidden directories
                if !name_str.starts_with('.') {
                    // Check if this directory contains SKILL.md
                    if path.join("SKILL.md").exists() {
                        count += 1;
                    }
                    // Also check subdirectories
                    count += count_skill_directories(&path);
                }
            }
        }
    }
    count
}

/// Count plugin directories (plugins/local subdirectories)
fn count_plugin_directories(dir_path: &std::path::Path) -> u32 {
    if !dir_path.exists() {
        return 0;
    }

    let mut count = 0;

    // Check plugins/local first
    let local_plugins = dir_path.join("local");
    if local_plugins.exists() {
        if let Ok(entries) = std::fs::read_dir(&local_plugins) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if path.is_dir() && !name_str.starts_with('.') {
                    count += 1;
                }
            }
        }
    }

    // Also count direct subdirectories of plugins/ that aren't "local"
    if let Ok(entries) = std::fs::read_dir(dir_path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if path.is_dir() && !name_str.starts_with('.') && name_str != "local" {
                count += 1;
            }
        }
    }

    count
}

/// Count workspace items (skills, commands, agents, plugins)
pub fn count_workspace_items(
    workspace_path: &str,
) -> Result<(Option<u32>, Option<u32>, Option<u32>, Option<u32>), String> {
    let path = std::path::Path::new(workspace_path);

    // Skills are directories containing SKILL.md (recursive)
    let skills_count = count_skill_directories(&path.join("skills"));
    // Commands and agents are .md files (recursive search)
    let commands_count = count_directory_items(&path.join("commands"));
    let agents_count = count_directory_items(&path.join("agents"));
    // Plugins are directories in plugins/ and plugins/local/
    let plugins_count = count_plugin_directories(&path.join("plugins"));

    Ok((
        Some(skills_count),
        Some(commands_count),
        Some(agents_count),
        Some(plugins_count),
    ))
}

/// Copy ~/.claude directory to workspace with exclusions
pub fn copy_claude_to_workspace(workspace_id: &str, include_scripts: bool) -> Result<String, String> {
    let claude_dir = get_claude_dir()?;
    let workspaces_path = get_workspaces_dir()?;
    let workspace_path = workspaces_path.join(format!("ws_{}", workspace_id));

    // Ensure workspaces directory exists
    std::fs::create_dir_all(&workspaces_path)
        .map_err(|e| format!("Failed to create workspaces directory: {}", e))?;

    // Create workspace directory
    std::fs::create_dir_all(&workspace_path)
        .map_err(|e| format!("Failed to create workspace directory: {}", e))?;

    if !claude_dir.exists() {
        println!("Claude directory does not exist, creating empty workspace");
        return Ok(workspace_path.to_string_lossy().to_string());
    }

    // Copy files and directories with exclusions
    for entry in std::fs::read_dir(&claude_dir)
        .map_err(|e| format!("Failed to read Claude directory: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let source_path = entry.path();
        let file_name = source_path.file_name().ok_or("Invalid file name")?;

        // Check if this should be excluded
        if should_exclude(&source_path, include_scripts) {
            println!("Excluding: {}", source_path.display());
            continue;
        }

        let dest_path = workspace_path.join(file_name);

        if source_path.is_file() {
            std::fs::copy(&source_path, &dest_path)
                .map_err(|e| format!("Failed to copy file {}: {}", source_path.display(), e))?;
            println!("Copied file: {}", file_name.to_string_lossy());
        } else if source_path.is_dir() {
            // Use fs_extra for recursive directory copy
            let mut options = CopyOptions::new();
            options.overwrite = true;
            options.copy_inside = true;

            copy_dir(&source_path, &workspace_path, &options)
                .map_err(|e| format!("Failed to copy directory {}: {}", source_path.display(), e))?;
            println!("Copied directory: {}", file_name.to_string_lossy());
        }
    }

    println!("Workspace created at: {}", workspace_path.display());
    Ok(workspace_path.to_string_lossy().to_string())
}

/// Sync current ~/.claude content to an existing workspace path
pub fn sync_workspace_content(workspace_path: &str, include_scripts: bool) -> Result<(), String> {
    let claude_dir = get_claude_dir()?;
    let workspace = std::path::Path::new(workspace_path);

    if !claude_dir.exists() {
        return Ok(());
    }

    // Ensure workspace directory exists
    std::fs::create_dir_all(workspace)
        .map_err(|e| format!("Failed to create workspace directory: {}", e))?;

    // Clear existing workspace content (managed items only) to ensure clean sync
    let managed_items = vec![
        "settings.json",
        "CLAUDE.md",
        ".mcp.json",
        "skills",
        "commands",
        "agents",
        "rules",
        "plugins",
        "docs",
        "chrome",
        "song",
        "hooks",
    ];

    for item in &managed_items {
        let item_path = workspace.join(item);
        if item_path.exists() {
            if item_path.is_file() {
                let _ = std::fs::remove_file(&item_path);
            } else if item_path.is_dir() {
                let _ = std::fs::remove_dir_all(&item_path);
            }
        }
    }

    // Copy files and directories from ~/.claude to workspace
    for entry in std::fs::read_dir(&claude_dir)
        .map_err(|e| format!("Failed to read Claude directory: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let source_path = entry.path();
        let file_name = source_path.file_name().ok_or("Invalid file name")?;

        // Check if this should be excluded
        if should_exclude(&source_path, include_scripts) {
            continue;
        }

        let dest_path = workspace.join(file_name);

        if source_path.is_file() {
            std::fs::copy(&source_path, &dest_path)
                .map_err(|e| format!("Failed to copy file {}: {}", source_path.display(), e))?;
        } else if source_path.is_dir() {
            // Remove destination if exists to ensure clean copy
            if dest_path.exists() {
                let _ = std::fs::remove_dir_all(&dest_path);
            }
            let mut options = CopyOptions::new();
            options.overwrite = true;
            options.copy_inside = true;
            copy_dir(&source_path, workspace, &options)
                .map_err(|e| format!("Failed to copy directory {}: {}", source_path.display(), e))?;
        }
    }

    println!("Workspace synced: {}", workspace_path);
    Ok(())
}

/// Copy workspace back to ~/.claude
pub fn copy_workspace_to_claude(workspace_path: &str) -> Result<(), String> {
    println!("🟢🟢🟢 COPY_WORKSPACE_TO_CLAUDE called! workspace_path: {}", workspace_path);
    let claude_dir = get_claude_dir()?;
    let workspace = std::path::Path::new(workspace_path);

    if !workspace.exists() {
        println!("❌ Workspace path does not exist: {}", workspace_path);
        return Err(format!("Workspace path does not exist: {}", workspace_path));
    }
    println!("✓ Workspace exists: {}", workspace_path);

    // Ensure .claude directory exists
    std::fs::create_dir_all(&claude_dir)
        .map_err(|e| format!("Failed to create .claude directory: {}", e))?;

    // Items to skip when copying from workspace to ~/.claude
    let skip_items: Vec<&str> = vec![
        ".git",
        ".claude",
        ".DS_Store",
        ".gitignore",
        ".gitmodules",
        "debug",
        "file-history",
        "session-env",
        "shell-snapshots",
        "todos",
        "telemetry",
        "statsig",
        "projects",
        "paste-cache",
        "history.jsonl",
        "tool-usage.log",
        "stats-cache.json",
        "cache",
    ];

    // Copy files from workspace to ~/.claude
    for entry in std::fs::read_dir(workspace)
        .map_err(|e| format!("Failed to read workspace directory: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let source_path = entry.path();
        let file_name = source_path.file_name().ok_or("Invalid file name")?;
        let file_name_str = file_name.to_string_lossy();

        // Skip excluded items
        if skip_items.contains(&file_name_str.as_ref()) {
            println!("Skipping excluded item: {}", file_name_str);
            continue;
        }

        let dest_path = claude_dir.join(file_name);

        if source_path.is_file() {
            std::fs::copy(&source_path, &dest_path)
                .map_err(|e| format!("Failed to copy file {}: {}", source_path.display(), e))?;
            println!("Restored file: {}", file_name_str);
        } else if source_path.is_dir() {
            // Remove existing directory first to avoid conflicts
            if dest_path.exists() {
                std::fs::remove_dir_all(&dest_path).map_err(|e| {
                    format!(
                        "Failed to remove existing directory {}: {}",
                        dest_path.display(),
                        e
                    )
                })?;
            }

            // Use fs_extra for recursive directory copy
            let mut options = CopyOptions::new();
            options.overwrite = true;
            options.copy_inside = true;

            copy_dir(&source_path, &claude_dir, &options)
                .map_err(|e| format!("Failed to copy directory {}: {}", source_path.display(), e))?;
            println!("Restored directory: {}", file_name_str);
        }
    }

    println!("Workspace restored to ~/.claude");

    // Verify the copy worked
    let agents_dir = claude_dir.join("agents");
    let commands_dir = claude_dir.join("commands");
    println!("🟢 Verification: agents dir exists: {}, commands dir exists: {}",
        agents_dir.exists(), commands_dir.exists());
    if agents_dir.exists() {
        let count = std::fs::read_dir(&agents_dir).map(|d| d.count()).unwrap_or(0);
        println!("🟢 Verification: {} items in agents dir", count);
    }
    if commands_dir.exists() {
        let count = std::fs::read_dir(&commands_dir).map(|d| d.count()).unwrap_or(0);
        println!("🟢 Verification: {} items in commands dir", count);
    }

    Ok(())
}

/// Clear ~/.claude directory for switch (preserving session-specific items)
pub fn clear_claude_dir_for_switch() -> Result<(), String> {
    println!("🔴🔴🔴 CLEAR_CLAUDE_DIR_FOR_SWITCH CALLED! This deletes skills/commands/agents/plugins!");
    let claude_dir = get_claude_dir()?;

    if !claude_dir.exists() {
        return Ok(());
    }

    // Only remove items that are managed by workspaces
    let items_to_clear = vec![
        "settings.json",
        "CLAUDE.md",
        ".mcp.json",
        "skills",
        "commands",
        "agents",
        "rules",
        "plugins",
        "docs",
        "chrome",
        "song",
        "hooks",
        "scripts", // Clear scripts if workspace manages them
    ];

    for item in items_to_clear {
        let item_path = claude_dir.join(item);
        if item_path.exists() {
            if item_path.is_file() {
                std::fs::remove_file(&item_path)
                    .map_err(|e| format!("Failed to remove file {}: {}", item, e))?;
            } else if item_path.is_dir() {
                std::fs::remove_dir_all(&item_path)
                    .map_err(|e| format!("Failed to remove directory {}: {}", item, e))?;
            }
            println!("Cleared: {}", item);
        }
    }

    Ok(())
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

/// Sync workspace from current ~/.claude state (update workspace with current state)
#[tauri::command]
pub async fn sync_workspace_from_claude(store_id: String) -> Result<(), String> {
    let stores_file = get_stores_file()?;

    if !stores_file.exists() {
        return Err("Stores file does not exist".to_string());
    }

    let mut stores_data = read_stores()?;

    // Find the store
    let store = stores_data
        .configs
        .iter_mut()
        .find(|s| s.id == store_id)
        .ok_or("Store not found")?;

    if store.workspace_type != WorkspaceType::FullDirectory {
        return Err("Cannot sync a settings-only workspace".to_string());
    }

    // Git: Commit current changes if ~/.claude is a git repo
    if git_is_repo()? {
        if git_has_changes()? {
            git_auto_commit(&format!("Sync: workspace/{}", store_id))?;
            println!("Git: committed changes for workspace/{}", store_id);
        } else {
            println!("Git: no changes to commit");
        }
    }

    let workspace_path = store
        .workspace_path
        .as_ref()
        .ok_or("Workspace path not found")?;

    // Remove old workspace and recreate
    let workspace_dir = std::path::Path::new(workspace_path);
    if workspace_dir.exists() {
        std::fs::remove_dir_all(workspace_dir)
            .map_err(|e| format!("Failed to remove old workspace: {}", e))?;
    }

    // Copy current ~/.claude to workspace (for backup/reference)
    let new_path = copy_claude_to_workspace(&store_id, store.include_scripts)?;

    // Update metadata
    let (skills, commands, agents, plugins) = count_workspace_items(&new_path)?;
    store.workspace_path = Some(new_path);
    store.skills_count = skills;
    store.commands_count = commands;
    store.agents_count = agents;
    store.plugins_count = plugins;
    store.last_synced = Some(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("Failed to get timestamp: {}", e))?
            .as_secs(),
    );

    // Write back
    write_stores(&stores_data)?;

    println!("Workspace synced successfully");
    Ok(())
}

/// Get current item counts from ~/.claude directory
/// This is used to compare with workspace counts to detect unsaved changes
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClaudeDirCounts {
    pub skills: u32,
    pub commands: u32,
    pub agents: u32,
    pub plugins: u32,
}

#[tauri::command]
pub async fn get_claude_dir_counts() -> Result<ClaudeDirCounts, String> {
    let claude_dir = get_claude_dir()?;

    if !claude_dir.exists() {
        println!("⚪ GET_CLAUDE_DIR_COUNTS: ~/.claude does not exist, returning 0s");
        return Ok(ClaudeDirCounts {
            skills: 0,
            commands: 0,
            agents: 0,
            plugins: 0,
        });
    }

    let skills = count_skill_directories(&claude_dir.join("skills"));
    let commands = count_directory_items(&claude_dir.join("commands"));
    let agents = count_directory_items(&claude_dir.join("agents"));
    let plugins = count_plugin_directories(&claude_dir.join("plugins"));

    println!("⚪ GET_CLAUDE_DIR_COUNTS: skills={}, commands={}, agents={}, plugins={}", skills, commands, agents, plugins);

    Ok(ClaudeDirCounts {
        skills,
        commands,
        agents,
        plugins,
    })
}

/// Refresh workspace item counts without copying files from ~/.claude
#[tauri::command]
pub async fn refresh_workspace_counts(store_id: String) -> Result<ConfigStore, String> {
    let stores_file = get_stores_file()?;

    if !stores_file.exists() {
        return Err("Stores file does not exist".to_string());
    }

    let mut stores_data = read_stores()?;

    // Find the store
    let store = stores_data
        .configs
        .iter_mut()
        .find(|s| s.id == store_id)
        .ok_or("Store not found")?;

    if store.workspace_type != WorkspaceType::FullDirectory {
        return Err("Cannot refresh counts for a settings-only workspace".to_string());
    }

    let workspace_path = store
        .workspace_path
        .as_ref()
        .ok_or("Workspace path not found")?;

    // Recalculate counts from existing workspace (no file copying!)
    let (skills, commands, agents, plugins) = count_workspace_items(workspace_path)?;
    store.skills_count = skills;
    store.commands_count = commands;
    store.agents_count = agents;
    store.plugins_count = plugins;

    let updated_store = store.clone();

    // Write back
    write_stores(&stores_data)?;

    println!("Workspace counts refreshed successfully for {}", store_id);
    Ok(updated_store)
}

/// List all items (skills, commands, agents, hooks, plugins) from a workspace
#[tauri::command]
pub async fn list_workspace_items(workspace_id: String) -> Result<WorkspaceItems, String> {
    let stores_data = read_stores()?;

    // Find the store
    let store = stores_data
        .configs
        .iter()
        .find(|s| s.id == workspace_id)
        .ok_or("Store not found")?;

    if store.workspace_type != WorkspaceType::FullDirectory {
        return Err("Cannot list items for a settings-only workspace".to_string());
    }

    let workspace_path = store
        .workspace_path
        .as_ref()
        .ok_or("Workspace path not found")?;

    let base_path = std::path::Path::new(workspace_path);

    // List all items
    let skills = list_skill_items(&base_path.join("skills"), base_path);
    let commands = list_md_items(&base_path.join("commands"), base_path, WorkspaceItemType::Command);
    let agents = list_md_items(&base_path.join("agents"), base_path, WorkspaceItemType::Agent);
    let hooks = list_md_items(&base_path.join("hooks"), base_path, WorkspaceItemType::Hook);
    let plugins = list_plugin_items(&base_path.join("plugins"), base_path);

    Ok(WorkspaceItems {
        skills,
        commands,
        agents,
        hooks,
        plugins,
    })
}

/// Copy selected items from a workspace to ~/.claude
#[tauri::command]
pub async fn copy_items_to_claude(
    source_workspace_id: String,
    items: Vec<WorkspaceItem>,
) -> Result<CopyItemsResult, String> {
    let stores_data = read_stores()?;

    // Find the source store
    let store = stores_data
        .configs
        .iter()
        .find(|s| s.id == source_workspace_id)
        .ok_or("Source store not found")?;

    if store.workspace_type != WorkspaceType::FullDirectory {
        return Err("Cannot copy items from a settings-only workspace".to_string());
    }

    let workspace_path = store
        .workspace_path
        .as_ref()
        .ok_or("Workspace path not found")?;

    let source_base = std::path::Path::new(workspace_path);
    let claude_dir = get_claude_dir()?;

    let mut copied_count = 0u32;
    let mut failed_items = Vec::new();

    for item in items {
        let source_path = source_base.join(&item.relative_path);
        let dest_path = claude_dir.join(&item.relative_path);

        // Ensure parent directory exists
        if let Some(parent) = dest_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                failed_items.push(format!("{}: Failed to create parent dir: {}", item.name, e));
                continue;
            }
        }

        let copy_result = match item.item_type {
            WorkspaceItemType::Skill | WorkspaceItemType::Plugin => {
                // Copy entire directory
                if source_path.is_dir() {
                    // Remove destination if exists to ensure clean copy
                    if dest_path.exists() {
                        let _ = std::fs::remove_dir_all(&dest_path);
                    }
                    let mut options = CopyOptions::new();
                    options.overwrite = true;
                    options.copy_inside = true;

                    // For fs_extra, we copy the directory INTO the parent
                    if let Some(parent) = dest_path.parent() {
                        copy_dir(&source_path, parent, &options)
                            .map(|_| ())
                            .map_err(|e| format!("Failed to copy directory: {}", e))
                    } else {
                        Err("Invalid destination path".to_string())
                    }
                } else {
                    Err(format!("Source is not a directory: {}", source_path.display()))
                }
            }
            WorkspaceItemType::Command | WorkspaceItemType::Agent | WorkspaceItemType::Hook => {
                // Copy single file
                if source_path.is_file() {
                    std::fs::copy(&source_path, &dest_path)
                        .map(|_| ())
                        .map_err(|e| format!("Failed to copy file: {}", e))
                } else {
                    Err(format!("Source is not a file: {}", source_path.display()))
                }
            }
        };

        match copy_result {
            Ok(_) => {
                copied_count += 1;
                println!("Copied: {} -> {}", source_path.display(), dest_path.display());
            }
            Err(e) => {
                failed_items.push(format!("{}: {}", item.name, e));
                eprintln!("Failed to copy {}: {}", item.name, e);
            }
        }
    }

    Ok(CopyItemsResult {
        copied_count,
        failed_items,
    })
}

/// Extract referenced file paths from settings.json (hooks, scripts)
fn extract_referenced_files(settings: &serde_json::Value, workspace_path: &std::path::Path) -> Vec<String> {
    let mut files = Vec::new();

    // Helper to check if a string looks like a valid file path (not a regex pattern or empty)
    let is_valid_file_path = |path: &str| -> bool {
        // Must not be empty
        if path.is_empty() {
            return false;
        }
        // Must not look like a regex pattern (contains | or starts with ^ or ends with $)
        if path.contains('|') || path.starts_with('^') || path.ends_with('$') {
            return false;
        }
        // Must not start with http
        if path.starts_with("http") {
            return false;
        }
        // Must look like a file path (contains / or . or ends with common extensions)
        path.contains('/') || path.contains('.') || path.ends_with(".ts") || path.ends_with(".js") || path.ends_with(".sh")
    };

    // Check hooks section for file references
    if let Some(hooks) = settings.get("hooks") {
        if let Some(hooks_obj) = hooks.as_object() {
            for (_hook_name, hook_value) in hooks_obj {
                // Hook can be a string (file path) or array of strings
                if let Some(path) = hook_value.as_str() {
                    if is_valid_file_path(path) && workspace_path.join(path).exists() {
                        files.push(path.to_string());
                    }
                } else if let Some(arr) = hook_value.as_array() {
                    for item in arr {
                        if let Some(obj) = item.as_object() {
                            // NOTE: "matcher" field is a regex pattern, NOT a file path - skip it!
                            // Check for "hooks" field which contains actual commands
                            if let Some(hooks_arr) = obj.get("hooks") {
                                if let Some(hooks_list) = hooks_arr.as_array() {
                                    for h in hooks_list {
                                        if let Some(path) = h.as_str() {
                                            if is_valid_file_path(path) && workspace_path.join(path).exists() {
                                                files.push(path.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        } else if let Some(path) = item.as_str() {
                            if is_valid_file_path(path) && workspace_path.join(path).exists() {
                                files.push(path.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    files.sort();
    files.dedup();
    println!("📁 extract_referenced_files: found {} files: {:?}", files.len(), files);
    files
}

/// Get workspace settings.json content and referenced files
#[tauri::command]
pub async fn get_workspace_settings(workspace_id: String) -> Result<WorkspaceSettings, String> {
    let stores_data = read_stores()?;

    let store = stores_data
        .configs
        .iter()
        .find(|s| s.id == workspace_id)
        .ok_or("Store not found")?;

    if store.workspace_type != WorkspaceType::FullDirectory {
        return Err("Cannot get settings from a settings-only workspace".to_string());
    }

    let workspace_path = store
        .workspace_path
        .as_ref()
        .ok_or("Workspace path not found")?;

    let base_path = std::path::Path::new(workspace_path);
    let settings_path = base_path.join("settings.json");

    if !settings_path.exists() {
        return Ok(WorkspaceSettings {
            content: serde_json::Value::Object(serde_json::Map::new()),
            referenced_files: vec![],
            exists: false,
        });
    }

    let content = std::fs::read_to_string(&settings_path)
        .map_err(|e| format!("Failed to read settings.json: {}", e))?;

    let settings: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse settings.json: {}", e))?;

    let referenced_files = extract_referenced_files(&settings, base_path);

    Ok(WorkspaceSettings {
        content: settings,
        referenced_files,
        exists: true,
    })
}

/// Copy or merge settings.json from workspace to ~/.claude
#[tauri::command]
pub async fn copy_workspace_settings(
    source_workspace_id: String,
    mode: SettingsMergeMode,
    copy_referenced_files: bool,
) -> Result<CopyItemsResult, String> {
    println!("🔵🔵🔵 COPY_WORKSPACE_SETTINGS called! source_workspace_id: {}, mode: {:?}", source_workspace_id, mode);
    let stores_data = read_stores()?;

    let store = stores_data
        .configs
        .iter()
        .find(|s| s.id == source_workspace_id)
        .ok_or("Source store not found")?;

    if store.workspace_type != WorkspaceType::FullDirectory {
        return Err("Cannot copy settings from a settings-only workspace".to_string());
    }

    let workspace_path = store
        .workspace_path
        .as_ref()
        .ok_or("Workspace path not found")?;

    let source_base = std::path::Path::new(workspace_path);
    let claude_dir = get_claude_dir()?;

    let source_settings_path = source_base.join("settings.json");
    let dest_settings_path = claude_dir.join("settings.json");

    if !source_settings_path.exists() {
        return Err("Source settings.json does not exist".to_string());
    }

    let mut copied_count = 0u32;
    let mut failed_items = Vec::new();

    // Read source settings
    let source_content = std::fs::read_to_string(&source_settings_path)
        .map_err(|e| format!("Failed to read source settings.json: {}", e))?;
    let source_settings: serde_json::Value = serde_json::from_str(&source_content)
        .map_err(|e| format!("Failed to parse source settings.json: {}", e))?;

    // Handle settings based on mode
    let final_settings = match mode {
        SettingsMergeMode::Replace => source_settings.clone(),
        SettingsMergeMode::Merge => {
            // Read existing dest settings if exists
            let dest_settings = if dest_settings_path.exists() {
                let content = std::fs::read_to_string(&dest_settings_path)
                    .map_err(|e| format!("Failed to read dest settings.json: {}", e))?;
                serde_json::from_str(&content)
                    .map_err(|e| format!("Failed to parse dest settings.json: {}", e))?
            } else {
                serde_json::Value::Object(serde_json::Map::new())
            };

            // Merge source into dest
            merge_json_values(dest_settings, source_settings.clone())
        }
    };

    // Write settings
    let json_content = serde_json::to_string_pretty(&final_settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    std::fs::write(&dest_settings_path, json_content)
        .map_err(|e| format!("Failed to write settings.json: {}", e))?;

    copied_count += 1;
    println!("Settings.json copied/merged successfully");

    // Verify ~/.claude content wasn't affected
    let agents_count = std::fs::read_dir(claude_dir.join("agents"))
        .map(|d| d.count())
        .unwrap_or(0);
    let commands_count = std::fs::read_dir(claude_dir.join("commands"))
        .map(|d| d.count())
        .unwrap_or(0);
    println!("🟠🟠🟠 CHERRY-PICK VERIFICATION: ~/.claude has {} agents, {} commands", agents_count, commands_count);

    // Copy referenced files if requested
    if copy_referenced_files {
        // First, copy common directories that are typically referenced in settings.json
        // These directories contain scripts, plugins, etc. that settings.json hooks depend on
        let common_dirs = ["scripts", "song", "chrome", "hooks"];
        for dir_name in common_dirs {
            let source_dir = source_base.join(dir_name);
            let dest_dir = claude_dir.join(dir_name);

            if source_dir.exists() && source_dir.is_dir() {
                println!("📂 Copying {} directory from workspace...", dir_name);

                // Remove existing directory to avoid conflicts
                if dest_dir.exists() {
                    if let Err(e) = std::fs::remove_dir_all(&dest_dir) {
                        println!("⚠️ Warning: Could not remove existing {}: {}", dir_name, e);
                    }
                }

                // Copy the directory
                let mut options = CopyOptions::new();
                options.overwrite = true;
                options.copy_inside = true;

                match copy_dir(&source_dir, &claude_dir, &options) {
                    Ok(_) => {
                        copied_count += 1;
                        println!("✅ Copied {} directory", dir_name);
                    }
                    Err(e) => {
                        failed_items.push(format!("{}: {}", dir_name, e));
                        println!("❌ Failed to copy {}: {}", dir_name, e);
                    }
                }
            }
        }

        // Then copy any other referenced files found in settings.json
        let referenced = extract_referenced_files(&source_settings, source_base);
        for file_path in referenced {
            // SAFETY: Skip empty paths or paths that would resolve to ~/.claude itself
            if file_path.is_empty() || file_path == "." || file_path == ".." {
                println!("⛔ SAFETY: Skipping dangerous path: '{}'", file_path);
                continue;
            }

            let source_file = source_base.join(&file_path);
            let dest_file = claude_dir.join(&file_path);

            // SAFETY: Ensure dest_file is inside ~/.claude, not ~/.claude itself
            if dest_file == claude_dir {
                println!("⛔ SAFETY: dest_file equals claude_dir, skipping: {}", file_path);
                continue;
            }

            // Create parent directories
            if let Some(parent) = dest_file.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    failed_items.push(format!("{}: Failed to create parent dir: {}", file_path, e));
                    continue;
                }
            }

            // Copy file or directory
            if source_file.is_file() {
                match std::fs::copy(&source_file, &dest_file) {
                    Ok(_) => {
                        copied_count += 1;
                        println!("Copied: {} -> {}", source_file.display(), dest_file.display());
                    }
                    Err(e) => {
                        failed_items.push(format!("{}: {}", file_path, e));
                    }
                }
            } else if source_file.is_dir() {
                // SAFETY: Never delete ~/.claude itself or its parent!
                if dest_file == claude_dir || dest_file.parent() == Some(&claude_dir) && file_path.is_empty() {
                    println!("⛔ SAFETY: Refusing to delete ~/.claude or its direct parent! file_path: {}", file_path);
                    continue;
                }
                if dest_file.exists() {
                    println!("📂 Removing existing directory: {}", dest_file.display());
                    let _ = std::fs::remove_dir_all(&dest_file);
                }
                let mut options = CopyOptions::new();
                options.overwrite = true;
                options.copy_inside = true;

                if let Some(parent) = dest_file.parent() {
                    match copy_dir(&source_file, parent, &options) {
                        Ok(_) => {
                            copied_count += 1;
                            println!("Copied dir: {} -> {}", source_file.display(), dest_file.display());
                        }
                        Err(e) => {
                            failed_items.push(format!("{}: {}", file_path, e));
                        }
                    }
                }
            }
        }
    }

    Ok(CopyItemsResult {
        copied_count,
        failed_items,
    })
}

/// Merge two JSON values (deep merge for objects)
fn merge_json_values(base: serde_json::Value, overlay: serde_json::Value) -> serde_json::Value {
    match (base, overlay) {
        (serde_json::Value::Object(mut base_obj), serde_json::Value::Object(overlay_obj)) => {
            for (key, value) in overlay_obj {
                base_obj.insert(key, value);
            }
            serde_json::Value::Object(base_obj)
        }
        (_, overlay) => overlay,
    }
}
