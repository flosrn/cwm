use std::path::PathBuf;

// Application configuration directory
pub const APP_CONFIG_DIR: &str = ".ccconfig";
pub const WORKSPACES_DIR: &str = "workspaces";

// Directories to EXCLUDE when copying ~/.claude to workspace (session-specific, caches)
pub const EXCLUDED_DIRS: &[&str] = &[
    "debug",            // Debug logs
    "file-history",     // File history
    "session-env",      // Session variables
    "shell-snapshots",  // Shell snapshots
    "todos",            // Todos (session-specific)
    "telemetry",        // Telemetry data
    "statsig",          // Feature flags
    "projects",         // Project state
    "paste-cache",      // Paste cache
    ".git",             // Git repo
    ".claude",          // User's local settings (settings.local.json, tasks)
    "history.jsonl",    // Conversation history
    "tool-usage.log",   // Tool usage log
    "stats-cache.json", // Stats cache
    "cache",            // General cache
];

/// Get the user's home directory
pub fn get_home_dir() -> Result<PathBuf, String> {
    dirs::home_dir().ok_or_else(|| "Could not find home directory".to_string())
}

/// Get the ~/.claude directory path
pub fn get_claude_dir() -> Result<PathBuf, String> {
    Ok(get_home_dir()?.join(".claude"))
}

/// Get the app config directory path (~/.ccconfig)
pub fn get_app_config_dir() -> Result<PathBuf, String> {
    Ok(get_home_dir()?.join(APP_CONFIG_DIR))
}

/// Get the stores.json file path
pub fn get_stores_file() -> Result<PathBuf, String> {
    Ok(get_app_config_dir()?.join("stores.json"))
}

/// Get the workspaces directory path
pub fn get_workspaces_dir() -> Result<PathBuf, String> {
    Ok(get_app_config_dir()?.join(WORKSPACES_DIR))
}

/// Read and parse the stores.json file
pub fn read_stores() -> Result<crate::commands::stores::StoresData, String> {
    let stores_file = get_stores_file()?;

    if !stores_file.exists() {
        return Ok(crate::commands::stores::StoresData::default());
    }

    let content = std::fs::read_to_string(&stores_file)
        .map_err(|e| format!("Failed to read stores file: {}", e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse stores file: {}", e))
}

/// Write the stores data to stores.json
pub fn write_stores(data: &crate::commands::stores::StoresData) -> Result<(), String> {
    let stores_file = get_stores_file()?;

    // Ensure parent directory exists
    if let Some(parent) = stores_file.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create stores directory: {}", e))?;
    }

    let json_content = serde_json::to_string_pretty(data)
        .map_err(|e| format!("Failed to serialize stores: {}", e))?;

    std::fs::write(&stores_file, json_content)
        .map_err(|e| format!("Failed to write stores file: {}", e))?;

    Ok(())
}

/// Check if a path should be excluded from copying
pub fn should_exclude(path: &std::path::Path, include_scripts: bool) -> bool {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Check if it's in the excluded list
    if EXCLUDED_DIRS.contains(&file_name) {
        return true;
    }

    // Handle scripts directory based on user preference
    if file_name == "scripts" && !include_scripts {
        return true;
    }

    false
}
