use fs_extra::dir::{copy as copy_dir, CopyOptions};

use crate::commands::git::{git_auto_commit, git_has_changes, git_is_repo};
use crate::commands::stores::{ConfigStore, WorkspaceType};
use crate::commands::utils::{
    get_claude_dir, get_stores_file, get_workspaces_dir, read_stores, should_exclude,
    write_stores,
};

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

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
    let claude_dir = get_claude_dir()?;
    let workspace = std::path::Path::new(workspace_path);

    if !workspace.exists() {
        return Err(format!("Workspace path does not exist: {}", workspace_path));
    }

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
    Ok(())
}

/// Clear ~/.claude directory for switch (preserving session-specific items)
pub fn clear_claude_dir_for_switch() -> Result<(), String> {
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
