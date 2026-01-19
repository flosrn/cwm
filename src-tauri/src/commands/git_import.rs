use fs_extra::dir::{copy as copy_dir, CopyOptions};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::commands::stores::{ConfigStore, WorkspaceType};
use crate::commands::utils::{get_workspaces_dir, read_stores, write_stores};
use crate::commands::workspace::count_workspace_items;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Directories to INCLUDE when importing from Git
const INCLUDE_DIRS: &[&str] = &[
    "agents", "commands", "hooks", "skills", "plugins", "rules", "docs",
];

/// Root files to INCLUDE when importing from Git
const INCLUDE_ROOT_FILES: &[&str] = &["settings.json", "CLAUDE.md", ".mcp.json"];

/// Directories to EXCLUDE when scanning Git repos
const EXCLUDE_DIRS: &[&str] = &[
    ".git",
    ".github",
    "node_modules",
    "bin",
    "src",
    "tests",
    "dist",
    "build",
    "target",
    "__pycache__",
    ".vscode",
    ".idea",
];

// ============================================================================
// TYPES
// ============================================================================

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct GitImportPreview {
    #[serde(rename = "repoName")]
    pub repo_name: String,
    #[serde(rename = "hasSettingsJson")]
    pub has_settings_json: bool,
    #[serde(rename = "hasClaudeMd")]
    pub has_claude_md: bool,
    #[serde(rename = "hasMcpJson")]
    pub has_mcp_json: bool,
    #[serde(rename = "skillsCount")]
    pub skills_count: u32,
    #[serde(rename = "commandsCount")]
    pub commands_count: u32,
    #[serde(rename = "agentsCount")]
    pub agents_count: u32,
    #[serde(rename = "pluginsCount")]
    pub plugins_count: u32,
    #[serde(rename = "hasHooks")]
    pub has_hooks: bool,
    #[serde(rename = "rootMdFiles")]
    pub root_md_files: Vec<String>,
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Extract repository name from Git URL
fn extract_repo_name(url: &str) -> String {
    // Handle various URL formats:
    // https://github.com/user/repo.git
    // https://github.com/user/repo
    // git@github.com:user/repo.git
    let url = url.trim_end_matches(".git");
    let url = url.trim_end_matches('/');

    if let Some(last_part) = url.rsplit('/').next() {
        last_part.to_string()
    } else if let Some(last_part) = url.rsplit(':').next() {
        last_part.rsplit('/').next().unwrap_or(last_part).to_string()
    } else {
        "imported-workspace".to_string()
    }
}

/// Clone a Git repository to a temporary directory
fn clone_repo_to_temp(url: &str) -> Result<PathBuf, String> {
    let temp_dir = std::env::temp_dir().join(format!("cwm_git_import_{}", nanoid::nanoid!(8)));

    // Create temp directory
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;

    // Clone the repository
    let output = Command::new("git")
        .args(["clone", "--depth", "1", url, temp_dir.to_string_lossy().as_ref()])
        .output()
        .map_err(|e| format!("Failed to execute git clone: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Clean up temp directory on failure
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(format!("Git clone failed: {}", stderr));
    }

    println!("Cloned repository to: {}", temp_dir.display());
    Ok(temp_dir)
}

/// Cleanup temporary directory
fn cleanup_temp_dir(path: &Path) {
    if path.exists() {
        if let Err(e) = std::fs::remove_dir_all(path) {
            eprintln!("Warning: Failed to cleanup temp directory: {}", e);
        } else {
            println!("Cleaned up temp directory: {}", path.display());
        }
    }
}

/// Count .md files in a directory recursively
fn count_md_files_recursive(dir_path: &Path) -> u32 {
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
                // Skip hidden directories
                if !name_str.starts_with('.') {
                    count += count_md_files_recursive(&path);
                }
            }
        }
    }
    count
}

/// Count subdirectories containing SKILL.md recursively (for skills)
fn count_skill_dirs_recursive(dir_path: &Path) -> u32 {
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
                    count += count_skill_dirs_recursive(&path);
                }
            }
        }
    }
    count
}

/// Count plugin directories (plugins/local and plugins at root level)
fn count_plugins(dir_path: &Path) -> u32 {
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

/// Get list of .md files at root level
fn get_root_md_files(dir_path: &Path) -> Vec<String> {
    if !dir_path.exists() {
        return vec![];
    }

    std::fs::read_dir(dir_path)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let path = e.path();
                    path.is_file()
                        && path.extension().map(|ext| ext == "md").unwrap_or(false)
                        && e.file_name().to_string_lossy() != "CLAUDE.md" // CLAUDE.md is tracked separately
                })
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Scan a cloned repository and generate preview
fn scan_repo_for_preview(repo_path: &Path) -> GitImportPreview {
    let repo_name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("workspace")
        .to_string();

    // Check for root files
    let has_settings_json = repo_path.join("settings.json").exists();
    let has_claude_md = repo_path.join("CLAUDE.md").exists();
    let has_mcp_json = repo_path.join(".mcp.json").exists();

    // Count items in standard directories (recursively)
    let skills_count = count_skill_dirs_recursive(&repo_path.join("skills"));
    let commands_count = count_md_files_recursive(&repo_path.join("commands"));
    let agents_count = count_md_files_recursive(&repo_path.join("agents"));
    let plugins_count = count_plugins(&repo_path.join("plugins"));
    let has_hooks = repo_path.join("hooks").exists()
        && std::fs::read_dir(repo_path.join("hooks"))
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false);

    // Get root .md files (README, CHANGELOG, etc.)
    let root_md_files = get_root_md_files(repo_path);

    GitImportPreview {
        repo_name,
        has_settings_json,
        has_claude_md,
        has_mcp_json,
        skills_count,
        commands_count,
        agents_count,
        plugins_count,
        has_hooks,
        root_md_files,
    }
}

/// Copy relevant files from cloned repo to workspace
fn copy_repo_to_workspace(repo_path: &Path, workspace_path: &Path) -> Result<(), String> {
    // Ensure workspace directory exists
    std::fs::create_dir_all(workspace_path)
        .map_err(|e| format!("Failed to create workspace directory: {}", e))?;

    // Copy included directories
    for dir_name in INCLUDE_DIRS {
        let source_dir = repo_path.join(dir_name);
        if source_dir.exists() && source_dir.is_dir() {
            let mut options = CopyOptions::new();
            options.overwrite = true;
            options.copy_inside = true;

            copy_dir(&source_dir, workspace_path, &options)
                .map_err(|e| format!("Failed to copy directory {}: {}", dir_name, e))?;
            println!("Copied directory: {}", dir_name);
        }
    }

    // Copy included root files
    for file_name in INCLUDE_ROOT_FILES {
        let source_file = repo_path.join(file_name);
        if source_file.exists() && source_file.is_file() {
            let dest_file = workspace_path.join(file_name);
            std::fs::copy(&source_file, &dest_file)
                .map_err(|e| format!("Failed to copy file {}: {}", file_name, e))?;
            println!("Copied file: {}", file_name);
        }
    }

    // Copy all .md files at root level (README, CHANGELOG, etc.)
    for entry in std::fs::read_dir(repo_path)
        .map_err(|e| format!("Failed to read repo directory: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        // Skip directories in the exclude list
        if path.is_dir() && EXCLUDE_DIRS.contains(&file_name_str.as_ref()) {
            continue;
        }

        // Copy .md files at root level
        if path.is_file() && path.extension().map(|ext| ext == "md").unwrap_or(false) {
            let dest_file = workspace_path.join(&file_name);
            std::fs::copy(&path, &dest_file)
                .map_err(|e| format!("Failed to copy file {}: {}", file_name_str, e))?;
            println!("Copied root file: {}", file_name_str);
        }
    }

    Ok(())
}

/// Read settings.json from workspace if it exists
fn read_workspace_settings(workspace_path: &Path) -> Value {
    let settings_path = workspace_path.join("settings.json");
    if settings_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&settings_path) {
            if let Ok(json) = serde_json::from_str(&content) {
                return json;
            }
        }
    }
    serde_json::json!({})
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

/// Preview a Git import without actually importing
#[tauri::command]
pub async fn preview_git_import(url: String) -> Result<GitImportPreview, String> {
    // Validate URL
    if url.trim().is_empty() {
        return Err("URL cannot be empty".to_string());
    }

    // Clone to temp directory
    let temp_dir = clone_repo_to_temp(&url)?;

    // Scan and generate preview
    let mut preview = scan_repo_for_preview(&temp_dir);

    // Extract the real repo name from URL
    preview.repo_name = extract_repo_name(&url);

    // Cleanup temp directory
    cleanup_temp_dir(&temp_dir);

    Ok(preview)
}

/// Import a workspace from a Git repository
#[tauri::command]
pub async fn import_workspace_from_git(
    url: String,
    title: String,
    id: String,
) -> Result<ConfigStore, String> {
    // Validate inputs
    if url.trim().is_empty() {
        return Err("URL cannot be empty".to_string());
    }
    if title.trim().is_empty() {
        return Err("Title cannot be empty".to_string());
    }

    // Clone to temp directory
    let temp_dir = clone_repo_to_temp(&url)?;

    // Get workspaces directory and create workspace path
    let workspaces_dir = get_workspaces_dir()?;
    std::fs::create_dir_all(&workspaces_dir)
        .map_err(|e| format!("Failed to create workspaces directory: {}", e))?;

    let workspace_path = workspaces_dir.join(format!("ws_{}", id));

    // Copy relevant files to workspace
    if let Err(e) = copy_repo_to_workspace(&temp_dir, &workspace_path) {
        cleanup_temp_dir(&temp_dir);
        return Err(e);
    }

    // Cleanup temp directory
    cleanup_temp_dir(&temp_dir);

    // Read settings from imported workspace
    let settings = read_workspace_settings(&workspace_path);

    // Count items in the new workspace
    let workspace_path_str = workspace_path.to_string_lossy().to_string();
    let (skills_count, commands_count, agents_count, plugins_count) =
        count_workspace_items(&workspace_path_str)?;

    // Read existing stores
    let mut stores_data = read_stores()?;

    // Check if we should activate this store
    let should_be_active = stores_data.configs.is_empty();

    // Create new ConfigStore
    let new_store = ConfigStore {
        id: id.clone(),
        title: title.clone(),
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("Failed to get timestamp: {}", e))?
            .as_secs(),
        settings,
        using: should_be_active,
        workspace_type: WorkspaceType::FullDirectory,
        workspace_path: Some(workspace_path_str),
        include_scripts: true, // Include hooks/scripts from imported repos
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
        source_url: Some(url),
    };

    // Add store to collection
    stores_data.configs.push(new_store.clone());

    // Write back to stores file
    write_stores(&stores_data)?;

    println!(
        "Imported workspace '{}' from Git repository",
        new_store.title
    );
    Ok(new_store)
}
