use crate::commands::stores::NotificationSettings;
use crate::commands::utils::{get_home_dir, read_stores, write_stores};

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Get the latest hook command based on the current operating system
fn get_latest_hook_command() -> serde_json::Value {
    if cfg!(target_os = "windows") {
        serde_json::json!({
            "__ccmate__": true,
            "type": "command",
            "command": "powershell -Command \"try { Invoke-RestMethod -Uri http://localhost:59948/claude_code/hooks -Method POST -ContentType 'application/json' -Body $input -ErrorAction Stop } catch { '' }\""
        })
    } else {
        serde_json::json!({
            "__ccmate__": true,
            "type": "command",
            "command": "curl -s -X POST http://localhost:59948/claude_code/hooks -H 'Content-Type: application/json' --data-binary @- 2>/dev/null || echo"
        })
    }
}

/// Update existing ccmate hooks for specified events (doesn't add new ones)
fn update_existing_hooks(
    hooks_obj: &mut serde_json::Map<String, serde_json::Value>,
    events: &[&str],
) -> Result<bool, String> {
    let latest_hook_command = get_latest_hook_command();
    let latest_command_str = latest_hook_command
        .get("command")
        .and_then(|cmd| cmd.as_str())
        .unwrap_or("");

    let mut hook_updated = false;

    for event in events {
        if let Some(event_hooks) = hooks_obj.get_mut(*event).and_then(|h| h.as_array_mut()) {
            for entry in event_hooks.iter_mut() {
                if let Some(hooks_array) = entry.get_mut("hooks").and_then(|h| h.as_array_mut()) {
                    for hook in hooks_array.iter_mut() {
                        if hook.get("__ccmate__").is_some() {
                            if let Some(existing_command) =
                                hook.get("command").and_then(|cmd| cmd.as_str())
                            {
                                if existing_command != latest_command_str {
                                    hook["command"] =
                                        serde_json::Value::String(latest_command_str.to_string());
                                    hook_updated = true;
                                    println!(
                                        "🔄 Updated {} hook command: {}",
                                        event, latest_command_str
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(hook_updated)
}

/// Update or add ccmate hooks for specified events
fn update_or_add_hooks(
    hooks_obj: &mut serde_json::Map<String, serde_json::Value>,
    events: &[&str],
) -> Result<bool, String> {
    let latest_hook_command = get_latest_hook_command();
    let mut hook_updated = false;

    for event in events {
        if let Some(event_hooks) = hooks_obj.get_mut(*event).and_then(|h| h.as_array_mut()) {
            for entry in event_hooks.iter_mut() {
                if let Some(hooks_array) = entry.get_mut("hooks").and_then(|h| h.as_array_mut()) {
                    for hook in hooks_array.iter_mut() {
                        if hook.get("__ccmate__").is_some() {
                            if hook.get("command") != latest_hook_command.get("command") {
                                *hook = latest_hook_command.clone();
                                hook_updated = true;
                            }
                        }
                    }
                }
            }

            let ccmate_hook_exists = event_hooks.iter().any(|entry| {
                if let Some(hooks_array) = entry.get("hooks").and_then(|h| h.as_array()) {
                    hooks_array
                        .iter()
                        .any(|hook| hook.get("__ccmate__").is_some())
                } else {
                    false
                }
            });

            if !ccmate_hook_exists {
                let ccmate_hook_entry = serde_json::json!({
                    "hooks": [latest_hook_command.clone()]
                });
                event_hooks.push(ccmate_hook_entry);
                hook_updated = true;
            }
        } else {
            let ccmate_hook_entry = serde_json::json!({
                "hooks": [latest_hook_command.clone()]
            });
            hooks_obj.insert(
                event.to_string(),
                serde_json::Value::Array(vec![ccmate_hook_entry]),
            );
            hook_updated = true;
        }
    }

    Ok(hook_updated)
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

#[tauri::command]
pub async fn get_notification_settings() -> Result<Option<NotificationSettings>, String> {
    let stores_data = read_stores()?;
    Ok(stores_data.notification)
}

#[tauri::command]
pub async fn update_notification_settings(settings: NotificationSettings) -> Result<(), String> {
    let mut stores_data = read_stores()?;
    stores_data.notification = Some(settings);
    write_stores(&stores_data)?;

    println!("✅ Notification settings updated successfully");
    Ok(())
}

#[tauri::command]
pub async fn update_claude_code_hook() -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let settings_path = home_dir.join(".claude/settings.json");

    if !settings_path.exists() {
        return add_claude_code_hook().await;
    }

    let content = std::fs::read_to_string(&settings_path)
        .map_err(|e| format!("Failed to read settings.json: {}", e))?;

    let mut settings: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse settings.json: {}", e))?;

    let hooks_obj = settings
        .as_object_mut()
        .unwrap()
        .entry("hooks".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .unwrap();

    let events = ["Notification", "Stop", "PreToolUse"];
    let hook_updated = update_existing_hooks(hooks_obj, &events)?;

    if hook_updated {
        let json_content = serde_json::to_string_pretty(&settings)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        if let Some(parent) = settings_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create .claude directory: {}", e))?;
        }

        std::fs::write(&settings_path, json_content)
            .map_err(|e| format!("Failed to write settings.json: {}", e))?;

        println!("✅ Claude Code hooks updated successfully");
    } else {
        println!("ℹ️  Claude Code hooks are already up to date - no updates needed");
    }

    Ok(())
}

#[tauri::command]
pub async fn add_claude_code_hook() -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let settings_path = home_dir.join(".claude/settings.json");

    let mut settings = if settings_path.exists() {
        let content = std::fs::read_to_string(&settings_path)
            .map_err(|e| format!("Failed to read settings.json: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse settings.json: {}", e))?
    } else {
        serde_json::Value::Object(serde_json::Map::new())
    };

    let hooks_obj = settings
        .as_object_mut()
        .unwrap()
        .entry("hooks".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .unwrap();

    let events = ["Notification", "Stop", "PreToolUse"];
    update_or_add_hooks(hooks_obj, &events)?;

    let json_content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create .claude directory: {}", e))?;
    }

    std::fs::write(&settings_path, json_content)
        .map_err(|e| format!("Failed to write settings.json: {}", e))?;

    println!("✅ Claude Code hooks added successfully");
    Ok(())
}

#[tauri::command]
pub async fn remove_claude_code_hook() -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let settings_path = home_dir.join(".claude/settings.json");

    if !settings_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(&settings_path)
        .map_err(|e| format!("Failed to read settings.json: {}", e))?;

    let mut settings: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse settings.json: {}", e))?;

    if let Some(hooks_obj) = settings.get_mut("hooks").and_then(|h| h.as_object_mut()) {
        let events = ["Notification", "Stop", "PreToolUse"];

        for event in events {
            if let Some(event_hooks) = hooks_obj.get_mut(event).and_then(|h| h.as_array_mut()) {
                let mut new_event_hooks = Vec::new();
                for entry in event_hooks.iter() {
                    if let Some(hooks_array) = entry.get("hooks").and_then(|h| h.as_array()) {
                        let filtered_hooks: Vec<serde_json::Value> = hooks_array
                            .iter()
                            .filter(|hook| hook.get("__ccmate__").is_none())
                            .cloned()
                            .collect();

                        if !filtered_hooks.is_empty() {
                            let mut new_entry = entry.clone();
                            new_entry["hooks"] = serde_json::Value::Array(filtered_hooks);
                            new_event_hooks.push(new_entry);
                        }
                    } else {
                        new_event_hooks.push(entry.clone());
                    }
                }
                *event_hooks = new_event_hooks;

                if event_hooks.is_empty() {
                    hooks_obj.remove(event);
                }
            }
        }

        if hooks_obj.is_empty() {
            settings.as_object_mut().unwrap().remove("hooks");
        }
    }

    let json_content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    std::fs::write(&settings_path, json_content)
        .map_err(|e| format!("Failed to write settings.json: {}", e))?;

    println!("✅ Claude Code hooks removed successfully");
    Ok(())
}
