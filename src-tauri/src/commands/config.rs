use serde_json::Value;
use std::path::PathBuf;

use crate::commands::utils::{get_app_config_dir, get_home_dir};

// ============================================================================
// TYPES
// ============================================================================

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ConfigFile {
    pub path: String,
    pub content: Value,
    pub exists: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ClaudeConfigFile {
    pub path: String,
    pub content: Value,
    pub exists: bool,
}

// ============================================================================
// INITIALIZATION
// ============================================================================

pub async fn initialize_app_config() -> Result<(), String> {
    println!("initialize_app_config called");

    let home_dir = get_home_dir()?;
    let app_config_path = get_app_config_dir()?;

    println!(
        "Checking if app config directory exists: {}",
        app_config_path.display()
    );

    // Create config directory if it doesn't exist
    if !app_config_path.exists() {
        println!("App config directory does not exist, creating...");
        std::fs::create_dir_all(&app_config_path)
            .map_err(|e| format!("Failed to create app config directory: {}", e))?;
        println!(
            "App config directory created: {}",
            app_config_path.display()
        );
    } else {
        println!("App config directory already exists");
    }

    // Check if we need to backup Claude configs
    let claude_dir = home_dir.join(".claude");
    println!(
        "Checking if Claude directory exists: {}",
        claude_dir.display()
    );

    if claude_dir.exists() {
        // Check if we already have a backup
        let backup_dir = app_config_path.join("claude_backup");
        if backup_dir.exists() {
            println!("Claude backup already exists, skipping backup");
        } else {
            println!("Claude directory exists but no backup found, backing up...");
            if let Err(e) = backup_claude_configs_internal(&app_config_path, &claude_dir) {
                return Err(format!("Failed to backup Claude configs: {}", e));
            }
            println!("Claude configs backed up successfully");
        }
    } else {
        println!("Claude directory does not exist, skipping backup");
    }

    Ok(())
}

fn backup_claude_configs_internal(
    app_config_path: &std::path::Path,
    claude_dir: &std::path::Path,
) -> Result<(), String> {
    // Create backup directory
    let backup_dir = app_config_path.join("claude_backup");

    std::fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Failed to create backup directory: {}", e))?;

    // Copy all files from .claude directory to backup
    for entry in std::fs::read_dir(claude_dir)
        .map_err(|e| format!("Failed to read Claude directory: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let source_path = entry.path();
        let file_name = source_path.file_name().ok_or("Invalid file name")?;
        let dest_path = backup_dir.join(file_name);

        if source_path.is_file() {
            std::fs::copy(&source_path, &dest_path)
                .map_err(|e| format!("Failed to copy file {}: {}", source_path.display(), e))?;
        }
    }

    Ok(())
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

#[tauri::command]
pub async fn read_config_file(config_type: String) -> Result<ConfigFile, String> {
    let home_dir = get_home_dir()?;

    let path = match config_type.as_str() {
        "user" => home_dir.join(".claude/settings.json"),
        "enterprise_macos" => {
            PathBuf::from("/Library/Application Support/ClaudeCode/managed-settings.json")
        }
        "enterprise_linux" => PathBuf::from("/etc/claude-code/managed-settings.json"),
        "enterprise_windows" => PathBuf::from("C:\\ProgramData\\ClaudeCode\\managed-settings.json"),
        "mcp_macos" => PathBuf::from("/Library/Application Support/ClaudeCode/managed-mcp.json"),
        "mcp_linux" => PathBuf::from("/etc/claude-code/managed-mcp.json"),
        "mcp_windows" => PathBuf::from("C:\\ProgramData\\ClaudeCode\\managed-mcp.json"),
        _ => return Err("Invalid configuration type".to_string()),
    };

    let path_str = path.to_string_lossy().to_string();

    if path.exists() {
        let content =
            std::fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))?;

        let json_content: Value =
            serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        Ok(ConfigFile {
            path: path_str,
            content: json_content,
            exists: true,
        })
    } else {
        Ok(ConfigFile {
            path: path_str,
            content: Value::Object(serde_json::Map::new()),
            exists: false,
        })
    }
}

#[tauri::command]
pub async fn write_config_file(config_type: String, content: Value) -> Result<(), String> {
    let home_dir = get_home_dir()?;

    let path = match config_type.as_str() {
        "user" => home_dir.join(".claude/settings.json"),
        _ => return Err("Cannot write to enterprise configuration files".to_string()),
    };

    let json_content = serde_json::to_string_pretty(&content)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

    std::fs::write(&path, json_content).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn list_config_files() -> Result<Vec<String>, String> {
    let mut configs = vec![];

    // User settings
    if let Ok(home) = get_home_dir() {
        let user_settings = home.join(".claude/settings.json");
        if user_settings.exists() {
            configs.push("user".to_string());
        }
    }

    // Enterprise settings (read-only)
    if cfg!(target_os = "macos") {
        let enterprise_path =
            PathBuf::from("/Library/Application Support/ClaudeCode/managed-settings.json");
        if enterprise_path.exists() {
            configs.push("enterprise_macos".to_string());
        }

        let mcp_path = PathBuf::from("/Library/Application Support/ClaudeCode/managed-mcp.json");
        if mcp_path.exists() {
            configs.push("mcp_macos".to_string());
        }
    } else if cfg!(target_os = "linux") {
        let enterprise_path = PathBuf::from("/etc/claude-code/managed-settings.json");
        if enterprise_path.exists() {
            configs.push("enterprise_linux".to_string());
        }

        let mcp_path = PathBuf::from("/etc/claude-code/managed-mcp.json");
        if mcp_path.exists() {
            configs.push("mcp_linux".to_string());
        }
    } else if cfg!(target_os = "windows") {
        let enterprise_path = PathBuf::from("C:\\ProgramData\\ClaudeCode\\managed-settings.json");
        if enterprise_path.exists() {
            configs.push("enterprise_windows".to_string());
        }

        let mcp_path = PathBuf::from("C:\\ProgramData\\ClaudeCode\\managed-mcp.json");
        if mcp_path.exists() {
            configs.push("mcp_windows".to_string());
        }
    }

    Ok(configs)
}

#[tauri::command]
pub async fn check_app_config_exists() -> Result<bool, String> {
    let app_config_path = get_app_config_dir()?;
    Ok(app_config_path.exists())
}

#[tauri::command]
pub async fn create_app_config_dir() -> Result<(), String> {
    let app_config_path = get_app_config_dir()?;

    std::fs::create_dir_all(&app_config_path)
        .map_err(|e| format!("Failed to create app config directory: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn backup_claude_configs() -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let claude_dir = home_dir.join(".claude");
    let app_config_path = get_app_config_dir()?;

    if !claude_dir.exists() {
        return Err("Claude configuration directory does not exist".to_string());
    }

    // Ensure app config directory exists
    std::fs::create_dir_all(&app_config_path)
        .map_err(|e| format!("Failed to create app config directory: {}", e))?;

    backup_claude_configs_internal(&app_config_path, &claude_dir)
}

#[tauri::command]
pub async fn open_config_path() -> Result<(), String> {
    let app_config_path = get_app_config_dir()?;

    // Ensure the directory exists
    if !app_config_path.exists() {
        std::fs::create_dir_all(&app_config_path)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    // Open the directory in the system's file manager
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&app_config_path)
            .spawn()
            .map_err(|e| format!("Failed to open config directory: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&app_config_path)
            .spawn()
            .map_err(|e| format!("Failed to open config directory: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&app_config_path)
            .spawn()
            .map_err(|e| format!("Failed to open config directory: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn read_claude_config_file() -> Result<ClaudeConfigFile, String> {
    let home_dir = get_home_dir()?;
    let claude_json_path = home_dir.join(".claude.json");

    let path_str = claude_json_path.to_string_lossy().to_string();

    if claude_json_path.exists() {
        let content = std::fs::read_to_string(&claude_json_path)
            .map_err(|e| format!("Failed to read .claude.json: {}", e))?;

        let json_content: Value =
            serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        Ok(ClaudeConfigFile {
            path: path_str,
            content: json_content,
            exists: true,
        })
    } else {
        Ok(ClaudeConfigFile {
            path: path_str,
            content: Value::Object(serde_json::Map::new()),
            exists: false,
        })
    }
}

#[tauri::command]
pub async fn write_claude_config_file(content: Value) -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let claude_json_path = home_dir.join(".claude.json");

    let json_content = serde_json::to_string_pretty(&content)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

    std::fs::write(&claude_json_path, json_content)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}
