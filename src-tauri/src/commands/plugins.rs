use crate::commands::utils::get_home_dir;

// ============================================================================
// TYPES
// ============================================================================

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub path: String,
    pub is_local: bool,
    pub is_enabled: bool,
    pub has_mcp: bool,
    pub commands_count: u32,
    pub agents_count: u32,
    pub skills_count: u32,
    pub description: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct InstalledPlugins {
    pub version: u32,
    pub plugins: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PluginMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn count_files_in_dir(dir: &std::path::Path) -> u32 {
    if !dir.exists() {
        return 0;
    }
    std::fs::read_dir(dir)
        .map(|entries| entries.filter_map(|e| e.ok()).count() as u32)
        .unwrap_or(0)
}

fn read_plugin_description(plugin_path: &std::path::Path) -> Option<String> {
    // Try to read from .claude-plugin/plugin.json
    let plugin_json_path = plugin_path.join(".claude-plugin/plugin.json");
    if plugin_json_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&plugin_json_path) {
            if let Ok(metadata) = serde_json::from_str::<PluginMetadata>(&content) {
                return metadata.description;
            }
        }
    }

    // Try to read first line of README.md
    let readme_path = plugin_path.join("README.md");
    if readme_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&readme_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    return Some(trimmed.chars().take(100).collect());
                }
            }
        }
    }

    None
}

fn get_installed_plugins() -> Result<InstalledPlugins, String> {
    let home_dir = get_home_dir()?;
    let installed_path = home_dir.join(".claude/plugins/installed_plugins.json");

    if !installed_path.exists() {
        return Ok(InstalledPlugins::default());
    }

    let content = std::fs::read_to_string(&installed_path)
        .map_err(|e| format!("Failed to read installed_plugins.json: {}", e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse installed_plugins.json: {}", e))
}

fn save_installed_plugins(installed: &InstalledPlugins) -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let installed_path = home_dir.join(".claude/plugins/installed_plugins.json");

    let content = serde_json::to_string_pretty(installed)
        .map_err(|e| format!("Failed to serialize installed_plugins.json: {}", e))?;

    std::fs::write(&installed_path, content)
        .map_err(|e| format!("Failed to write installed_plugins.json: {}", e))?;

    Ok(())
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

#[tauri::command]
pub async fn read_claude_plugins() -> Result<Vec<PluginInfo>, String> {
    let home_dir = get_home_dir()?;
    let plugins_dir = home_dir.join(".claude/plugins");

    if !plugins_dir.exists() {
        return Ok(vec![]);
    }

    let installed = get_installed_plugins()?;
    let mut plugins = Vec::new();

    // Read local plugins
    let local_dir = plugins_dir.join("local");
    if local_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&local_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    let plugin_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let has_mcp = path.join(".mcp.json").exists();
                    let commands_count = count_files_in_dir(&path.join("commands"));
                    let agents_count = count_files_in_dir(&path.join("agents"));
                    let skills_count = count_files_in_dir(&path.join("skills"));
                    let description = read_plugin_description(&path);

                    let plugin_key = format!("local/{}", plugin_name);
                    let is_enabled = !installed.plugins.contains_key(&plugin_key)
                        || installed
                            .plugins
                            .get(&plugin_key)
                            .and_then(|v| v.get("enabled"))
                            .and_then(|v| v.as_bool())
                            .unwrap_or(true);

                    plugins.push(PluginInfo {
                        name: plugin_name,
                        path: path.to_string_lossy().to_string(),
                        is_local: true,
                        is_enabled,
                        has_mcp,
                        commands_count,
                        agents_count,
                        skills_count,
                        description,
                    });
                }
            }
        }
    }

    // Read marketplace plugins
    let marketplaces_dir = plugins_dir.join("marketplaces");
    if marketplaces_dir.exists() {
        if let Ok(marketplace_entries) = std::fs::read_dir(&marketplaces_dir) {
            for marketplace_entry in marketplace_entries.filter_map(|e| e.ok()) {
                let marketplace_path = marketplace_entry.path();
                if marketplace_path.is_dir() {
                    let marketplace_name = marketplace_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");

                    if let Ok(plugin_entries) = std::fs::read_dir(&marketplace_path) {
                        for plugin_entry in plugin_entries.filter_map(|e| e.ok()) {
                            let path = plugin_entry.path();
                            if path.is_dir() {
                                let plugin_name = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();

                                let has_mcp = path.join(".mcp.json").exists();
                                let commands_count = count_files_in_dir(&path.join("commands"));
                                let agents_count = count_files_in_dir(&path.join("agents"));
                                let skills_count = count_files_in_dir(&path.join("skills"));
                                let description = read_plugin_description(&path);

                                let plugin_key =
                                    format!("marketplaces/{}/{}", marketplace_name, plugin_name);
                                let is_enabled = !installed.plugins.contains_key(&plugin_key)
                                    || installed
                                        .plugins
                                        .get(&plugin_key)
                                        .and_then(|v| v.get("enabled"))
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(true);

                                plugins.push(PluginInfo {
                                    name: format!("{}/{}", marketplace_name, plugin_name),
                                    path: path.to_string_lossy().to_string(),
                                    is_local: false,
                                    is_enabled,
                                    has_mcp,
                                    commands_count,
                                    agents_count,
                                    skills_count,
                                    description,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort plugins: local first, then alphabetically
    plugins.sort_by(|a, b| match (a.is_local, b.is_local) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(plugins)
}

#[tauri::command]
pub async fn toggle_plugin(plugin_path: String, enabled: bool) -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let plugins_base = home_dir.join(".claude/plugins");

    let path = std::path::Path::new(&plugin_path);
    let plugin_key = if plugin_path.contains("/local/") {
        let plugin_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("Invalid plugin path")?;
        format!("local/{}", plugin_name)
    } else if plugin_path.contains("/marketplaces/") {
        let relative = path
            .strip_prefix(&plugins_base.join("marketplaces"))
            .map_err(|_| "Invalid marketplace plugin path")?;
        format!("marketplaces/{}", relative.to_string_lossy())
    } else {
        return Err("Unknown plugin type".to_string());
    };

    let mut installed = get_installed_plugins()?;

    installed
        .plugins
        .insert(plugin_key, serde_json::json!({ "enabled": enabled }));

    save_installed_plugins(&installed)?;

    Ok(())
}

#[tauri::command]
pub async fn delete_local_plugin(plugin_name: String) -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let plugin_dir = home_dir.join(".claude/plugins/local").join(&plugin_name);

    if !plugin_dir.exists() {
        return Err(format!("Plugin '{}' not found", plugin_name));
    }

    std::fs::remove_dir_all(&plugin_dir)
        .map_err(|e| format!("Failed to delete plugin: {}", e))?;

    let mut installed = get_installed_plugins()?;
    let plugin_key = format!("local/{}", plugin_name);
    installed.plugins.remove(&plugin_key);
    save_installed_plugins(&installed)?;

    Ok(())
}
