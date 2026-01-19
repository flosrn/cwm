use serde_json::Value;

use crate::commands::utils::get_home_dir;

// ============================================================================
// TYPES
// ============================================================================

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct McpServer {
    #[serde(flatten)]
    pub config: serde_json::Value,
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

#[tauri::command]
pub async fn get_global_mcp_servers() -> Result<std::collections::HashMap<String, McpServer>, String>
{
    let home_dir = get_home_dir()?;
    let claude_json_path = home_dir.join(".claude.json");

    if !claude_json_path.exists() {
        return Ok(std::collections::HashMap::new());
    }

    let content = std::fs::read_to_string(&claude_json_path)
        .map_err(|e| format!("Failed to read .claude.json: {}", e))?;

    let json_value: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse .claude.json: {}", e))?;

    let mcp_servers_obj = json_value
        .get("mcpServers")
        .and_then(|servers| servers.as_object())
        .cloned()
        .unwrap_or_else(serde_json::Map::new);

    let mut result = std::collections::HashMap::new();
    for (name, config) in mcp_servers_obj {
        let mcp_server = McpServer {
            config: config.clone(),
        };
        result.insert(name.clone(), mcp_server);
    }

    Ok(result)
}

#[tauri::command]
pub async fn check_mcp_server_exists(server_name: String) -> Result<bool, String> {
    let mcp_servers = get_global_mcp_servers().await?;
    Ok(mcp_servers.contains_key(&server_name))
}

#[tauri::command]
pub async fn update_global_mcp_server(
    server_name: String,
    server_config: Value,
) -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let claude_json_path = home_dir.join(".claude.json");

    // Read existing .claude.json or create new structure
    let mut json_value = if claude_json_path.exists() {
        let content = std::fs::read_to_string(&claude_json_path)
            .map_err(|e| format!("Failed to read .claude.json: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse .claude.json: {}", e))?
    } else {
        Value::Object(serde_json::Map::new())
    };

    // Update mcpServers object
    let mcp_servers = json_value
        .as_object_mut()
        .unwrap()
        .entry("mcpServers".to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .unwrap();

    // Update the specific server
    mcp_servers.insert(server_name, server_config);

    // Write back to file
    let json_content = serde_json::to_string_pretty(&json_value)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

    std::fs::write(&claude_json_path, json_content)
        .map_err(|e| format!("Failed to write .claude.json: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn delete_global_mcp_server(server_name: String) -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let claude_json_path = home_dir.join(".claude.json");

    if !claude_json_path.exists() {
        return Err("Claude configuration file does not exist".to_string());
    }

    // Read existing .claude.json
    let content = std::fs::read_to_string(&claude_json_path)
        .map_err(|e| format!("Failed to read .claude.json: {}", e))?;

    let mut json_value: Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse .claude.json: {}", e))?;

    // Check if mcpServers exists
    let mcp_servers = json_value
        .as_object_mut()
        .unwrap()
        .get_mut("mcpServers")
        .and_then(|servers| servers.as_object_mut());

    let mcp_servers = match mcp_servers {
        Some(servers) => servers,
        None => return Err("No mcpServers found in .claude.json".to_string()),
    };

    // Check if the server exists
    if !mcp_servers.contains_key(&server_name) {
        return Err(format!("MCP server '{}' not found", server_name));
    }

    // Remove the server
    mcp_servers.remove(&server_name);

    // If mcpServers is now empty, remove the entire mcpServers object
    if mcp_servers.is_empty() {
        json_value.as_object_mut().unwrap().remove("mcpServers");
    }

    // Write back to file
    let json_content = serde_json::to_string_pretty(&json_value)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

    std::fs::write(&claude_json_path, json_content)
        .map_err(|e| format!("Failed to write .claude.json: {}", e))?;

    Ok(())
}
