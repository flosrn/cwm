use serde_json::Value;

use crate::commands::utils::get_home_dir;

// ============================================================================
// TYPES
// ============================================================================

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct UsageData {
    pub input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ProjectUsageRecord {
    pub uuid: String,
    pub timestamp: String,
    pub model: Option<String>,
    pub usage: Option<UsageData>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ProjectConfig {
    pub path: String,
    pub config: serde_json::Value,
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

#[tauri::command]
pub async fn read_project_usage_files() -> Result<Vec<ProjectUsageRecord>, String> {
    let home_dir = get_home_dir()?;
    let projects_dir = home_dir.join(".claude/projects");

    println!(
        "🔍 Looking for projects directory: {}",
        projects_dir.display()
    );

    if !projects_dir.exists() {
        println!("❌ Projects directory does not exist");
        return Ok(vec![]);
    }

    println!("✅ Projects directory exists");

    let mut all_records = Vec::new();
    let mut files_processed = 0;
    let mut lines_processed = 0;

    // Recursively find all .jsonl files in the projects directory and subdirectories
    fn find_jsonl_files(
        dir: &std::path::Path,
        files: &mut Vec<std::path::PathBuf>,
    ) -> Result<(), String> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory {}: {}", dir.display(), e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            if path.is_file() && path.extension().map(|ext| ext == "jsonl").unwrap_or(false) {
                files.push(path);
            } else if path.is_dir() {
                // Recursively search subdirectories
                if let Err(e) = find_jsonl_files(&path, files) {
                    println!("Warning: {}", e);
                }
            }
        }
        Ok(())
    }

    let mut jsonl_files = Vec::new();
    find_jsonl_files(&projects_dir, &mut jsonl_files)?;

    for path in jsonl_files {
        files_processed += 1;

        // Read the JSONL file
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?;

        // Process each line in the JSONL file
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            lines_processed += 1;

            // Parse the JSON line
            let json_value: Value = serde_json::from_str(line)
                .map_err(|e| format!("Failed to parse JSON line: {}", e))?;

            // Extract the required fields
            let uuid = json_value
                .get("uuid")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let timestamp = json_value
                .get("timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Extract model field (optional)
            let model = if let Some(model_str) = json_value.get("model").and_then(|v| v.as_str()) {
                Some(model_str.to_string())
            } else if let Some(message_obj) = json_value.get("message") {
                message_obj
                    .get("model")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            };

            // Extract usage data (optional)
            let usage = if let Some(usage_obj) = json_value.get("usage") {
                Some(UsageData {
                    input_tokens: usage_obj.get("input_tokens").and_then(|v| v.as_u64()),
                    cache_read_input_tokens: usage_obj
                        .get("cache_read_input_tokens")
                        .and_then(|v| v.as_u64()),
                    output_tokens: usage_obj.get("output_tokens").and_then(|v| v.as_u64()),
                })
            } else if let Some(message_obj) = json_value.get("message") {
                if let Some(usage_obj) = message_obj.get("usage") {
                    Some(UsageData {
                        input_tokens: usage_obj.get("input_tokens").and_then(|v| v.as_u64()),
                        cache_read_input_tokens: usage_obj
                            .get("cache_read_input_tokens")
                            .and_then(|v| v.as_u64()),
                        output_tokens: usage_obj.get("output_tokens").and_then(|v| v.as_u64()),
                    })
                } else {
                    None
                }
            } else {
                None
            };

            // Only include records with valid uuid, timestamp, and valid usage data
            if !uuid.is_empty() && !timestamp.is_empty() {
                if let Some(ref usage_data) = usage {
                    let input_tokens = usage_data.input_tokens.unwrap_or(0);
                    let output_tokens = usage_data.output_tokens.unwrap_or(0);

                    if input_tokens + output_tokens > 0 {
                        all_records.push(ProjectUsageRecord {
                            uuid,
                            timestamp,
                            model,
                            usage,
                        });
                    }
                }
            }
        }
    }

    println!(
        "📊 Summary: Processed {} files, {} lines, found {} records",
        files_processed,
        lines_processed,
        all_records.len()
    );
    Ok(all_records)
}

#[tauri::command]
pub async fn read_claude_projects() -> Result<Vec<ProjectConfig>, String> {
    let home_dir = get_home_dir()?;
    let claude_json_path = home_dir.join(".claude.json");

    if !claude_json_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&claude_json_path)
        .map_err(|e| format!("Failed to read .claude.json: {}", e))?;

    let json_value: Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse .claude.json: {}", e))?;

    let projects_obj = json_value
        .get("projects")
        .and_then(|projects| projects.as_object())
        .cloned()
        .unwrap_or_else(serde_json::Map::new);

    let mut result = Vec::new();
    for (path, config) in projects_obj {
        let project_config = ProjectConfig {
            path: path.clone(),
            config: config.clone(),
        };
        result.push(project_config);
    }

    Ok(result)
}
