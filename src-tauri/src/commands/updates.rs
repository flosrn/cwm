use serde_json::Value;
use tauri_plugin_updater::UpdaterExt;
use uuid::Uuid;

use crate::commands::utils::{get_app_config_dir, get_home_dir, read_stores, write_stores};

// ============================================================================
// TYPES
// ============================================================================

#[derive(serde::Serialize, serde::Deserialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub version: Option<String>,
    pub body: Option<String>,
    pub date: Option<String>,
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Get or create distinct_id from stores.json
pub async fn get_or_create_distinct_id() -> Result<String, String> {
    let app_config_path = get_app_config_dir()?;

    // Ensure app config directory exists
    std::fs::create_dir_all(&app_config_path)
        .map_err(|e| format!("Failed to create app config directory: {}", e))?;

    // Read existing stores or create new
    let mut stores_data = read_stores()?;

    // Return existing distinct_id or create new one
    if let Some(ref id) = stores_data.distinct_id {
        Ok(id.clone())
    } else {
        // Generate new UUID
        let new_id = Uuid::new_v4().to_string();
        stores_data.distinct_id = Some(new_id.clone());

        // Write back to stores.json
        write_stores(&stores_data)?;

        println!("Created new distinct_id: {}", new_id);
        Ok(new_id)
    }
}

/// Get operating system name in PostHog format
fn get_os_name() -> &'static str {
    #[cfg(target_os = "macos")]
    return "macOS";
    #[cfg(target_os = "windows")]
    return "Windows";
    #[cfg(target_os = "linux")]
    return "Linux";
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return "Unknown";
}

/// Get operating system version
fn get_os_version() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let output = Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .map_err(|e| format!("Failed to get macOS version: {}", e))?;

        let version = String::from_utf8(output.stdout)
            .map_err(|e| format!("Failed to parse macOS version: {}", e))?;

        Ok(version.trim().to_string())
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let output = Command::new("cmd")
            .args(&["/C", "ver"])
            .output()
            .map_err(|e| format!("Failed to get Windows version: {}", e))?;

        let version_str = String::from_utf8(output.stdout)
            .map_err(|e| format!("Failed to parse Windows version: {}", e))?;

        if let Some(start) = version_str.find("Version ") {
            let version_part = &version_str[start + 8..];
            let version = version_part.trim_end_matches("]").trim().to_string();
            Ok(version)
        } else {
            Ok("Unknown".to_string())
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(content) = fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if line.starts_with("VERSION_ID=") {
                    let version = line.split('=').nth(1).unwrap_or("Unknown").trim_matches('"');
                    return Ok(version.to_string());
                }
            }
        }

        use std::process::Command;
        let output = Command::new("uname")
            .arg("-r")
            .output()
            .map_err(|e| format!("Failed to get Linux kernel version: {}", e))?;

        let version = String::from_utf8(output.stdout)
            .map_err(|e| format!("Failed to parse Linux version: {}", e))?;

        Ok(version.trim().to_string())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    Ok("Unknown".to_string())
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

#[tauri::command]
pub async fn check_for_updates(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    println!("🔍 Checking for updates...");
    println!("📱 App version: {}", app.package_info().version);
    println!("🏷️  App identifier: {}", app.package_info().name);

    match app.updater() {
        Ok(updater) => {
            println!("✅ Updater initialized successfully");
            println!("📡 Checking update endpoint: https://github.com/djyde/ccmate-release/releases/latest/download/latest.json");

            match updater.check().await {
                Ok(Some(update)) => {
                    println!("🎉 Update available!");
                    println!("📦 Current version: {}", update.current_version);
                    println!("🚀 New version: {}", update.version);
                    println!("📝 Release notes: {:?}", update.body);
                    println!("📅 Release date: {:?}", update.date);
                    println!("🎯 Target platform: {:?}", update.target);

                    Ok(UpdateInfo {
                        available: true,
                        version: Some(update.version.clone()),
                        body: update.body.clone(),
                        date: update.date.map(|d| d.to_string()),
                    })
                }
                Ok(None) => {
                    println!("✅ No updates available - you're on the latest version");

                    Ok(UpdateInfo {
                        available: false,
                        version: None,
                        body: None,
                        date: None,
                    })
                }
                Err(e) => {
                    println!("❌ Error checking for updates: {}", e);
                    Err(format!("Failed to check for updates: {}", e))
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to initialize updater: {}", e);
            Err(format!("Failed to get updater: {}", e))
        }
    }
}

#[tauri::command]
pub async fn install_and_restart(app: tauri::AppHandle) -> Result<(), String> {
    println!("🚀 Starting update installation process...");

    match app.updater() {
        Ok(updater) => {
            println!("✅ Updater ready for installation");
            println!("📡 Re-checking for updates to get download info...");

            match updater.check().await {
                Ok(Some(update)) => {
                    println!("📥 Starting download and installation...");
                    println!("🎯 Update version: {}", update.version);
                    println!("🎯 Update target: {:?}", update.target);

                    match update
                        .download_and_install(
                            |chunk_length, content_length| {
                                let progress = if let Some(total) = content_length {
                                    (chunk_length as f64 / total as f64) * 100.0
                                } else {
                                    0.0
                                };
                                println!(
                                    "⬇️  Download progress: {:.1}% ({} bytes)",
                                    progress, chunk_length
                                );
                            },
                            || {
                                println!("✅ Download completed! Preparing to restart...");
                            },
                        )
                        .await
                    {
                        Ok(_) => {
                            println!(
                                "🔄 Update installed successfully! Restarting application in 500ms..."
                            );

                            let app_handle = app.clone();
                            tauri::async_runtime::spawn(async move {
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                println!("🔄 Restarting now!");
                                app_handle.restart();
                            });
                            Ok(())
                        }
                        Err(e) => {
                            println!("❌ Failed to install update: {}", e);
                            Err(format!("Failed to install update: {}", e))
                        }
                    }
                }
                Ok(None) => {
                    println!("ℹ️  No update available for installation");
                    Err("No update available".to_string())
                }
                Err(e) => {
                    println!("❌ Error checking for updates before installation: {}", e);
                    Err(format!("Failed to check for updates: {}", e))
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to get updater for installation: {}", e);
            Err(format!("Failed to get updater: {}", e))
        }
    }
}

#[tauri::command]
pub async fn rebuild_tray_menu_command(app: tauri::AppHandle) -> Result<(), String> {
    crate::tray::rebuild_tray_menu(app).await
}

#[tauri::command]
pub async fn unlock_cc_ext() -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let claude_config_path = home_dir.join(".claude/config.json");

    // Ensure .claude directory exists
    if let Some(parent) = claude_config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create .claude directory: {}", e))?;
    }

    if claude_config_path.exists() {
        let content = std::fs::read_to_string(&claude_config_path)
            .map_err(|e| format!("Failed to read config.json: {}", e))?;

        let mut json_value: Value = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config.json: {}", e))?;

        if json_value.get("primaryApiKey").is_none() {
            if let Some(obj) = json_value.as_object_mut() {
                obj.insert(
                    "primaryApiKey".to_string(),
                    Value::String("xxx".to_string()),
                );
            }

            let json_content = serde_json::to_string_pretty(&json_value)
                .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

            std::fs::write(&claude_config_path, json_content)
                .map_err(|e| format!("Failed to write config.json: {}", e))?;

            println!("Added primaryApiKey to existing config.json");
        } else {
            println!("primaryApiKey already exists in config.json, no action needed");
        }
    } else {
        let config = serde_json::json!({
            "primaryApiKey": "xxx"
        });

        let json_content = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

        std::fs::write(&claude_config_path, json_content)
            .map_err(|e| format!("Failed to write config.json: {}", e))?;

        println!("Created new config.json with primaryApiKey");
    }

    Ok(())
}

#[tauri::command]
pub async fn track(
    event: String,
    properties: serde_json::Value,
    app: tauri::AppHandle,
) -> Result<(), String> {
    println!("📊 Tracking event: {}", event);

    let distinct_id = get_or_create_distinct_id().await?;
    let app_version = app.package_info().version.to_string();
    let os_name = get_os_name();
    let os_version = get_os_version().unwrap_or_else(|_| "Unknown".to_string());

    let mut payload = serde_json::json!({
        "api_key": "phc_zlfJLeYsreOvash1EhL6IO6tnP00exm75OT50SjnNcy",
        "event": event,
        "properties": {
            "distinct_id": distinct_id,
            "app_version": app_version,
            "$os": os_name,
            "$os_version": os_version
        }
    });

    if let Some(props_obj) = payload["properties"].as_object_mut() {
        if let Some(additional_props) = properties.as_object() {
            for (key, value) in additional_props {
                props_obj.insert(key.clone(), value.clone());
            }
        }
    }

    if !payload["properties"]
        .as_object()
        .unwrap()
        .contains_key("timestamp")
    {
        let timestamp = chrono::Utc::now().to_rfc3339();
        payload["properties"]["timestamp"] = serde_json::Value::String(timestamp);
    }

    println!(
        "📤 Sending to PostHog: {}",
        serde_json::to_string_pretty(&payload).unwrap()
    );

    let client = reqwest::Client::new();
    let response = client
        .post("https://us.i.posthog.com/capture/")
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to send request to PostHog: {}", e))?;

    if response.status().is_success() {
        println!("✅ Event tracked successfully");
        Ok(())
    } else {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        println!("❌ Failed to track event: {} - {}", status, error_text);
        Err(format!("PostHog API error: {} - {}", status, error_text))
    }
}
