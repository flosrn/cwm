use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::commands::utils::{get_home_dir, get_claude_dir};
use std::path::PathBuf;

/// Hook definition within a methodology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyHook {
    #[serde(rename = "type")]
    pub hook_type: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
}

/// Hooks configuration for a methodology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyHooks {
    /// Whether to inherit hooks from base
    #[serde(default = "default_true")]
    pub inherit: bool,
    /// Additional hooks to add
    #[serde(default)]
    pub add: HashMap<String, Vec<MethodologyHook>>,
}

/// Settings configuration for a methodology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologySettings {
    /// Whether to merge with base settings or replace
    #[serde(default = "default_true")]
    pub merge: bool,
    /// Settings overrides
    #[serde(default)]
    pub overrides: serde_json::Value,
}

/// Methodology manifest (manifest.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    /// List of skill folder names to include
    #[serde(default)]
    pub skills: Vec<String>,
    /// List of command file/folder names to include
    #[serde(default)]
    pub commands: Vec<String>,
    /// List of agent file names to include
    #[serde(default)]
    pub agents: Vec<String>,
    /// Hooks configuration
    #[serde(default)]
    pub hooks: Option<MethodologyHooks>,
    /// Settings configuration
    #[serde(default)]
    pub settings: Option<MethodologySettings>,
}

/// Methodology metadata for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Methodology {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub path: String,
    pub skills_count: usize,
    pub commands_count: usize,
    pub agents_count: usize,
    pub is_active: bool,
}

/// Active methodology tracking in stores.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyState {
    pub active_methodology: Option<String>,
    pub last_switched: Option<i64>,
}

fn default_true() -> bool { true }
fn default_version() -> String { "1.0.0".to_string() }

impl MethodologyManifest {
    /// Load manifest from a methodology directory
    pub fn load(methodology_path: &std::path::Path) -> Result<Self, String> {
        let manifest_path = methodology_path.join("manifest.json");
        let content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read manifest: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse manifest: {}", e))
    }
}

/// Get the methodologies directory path
pub fn get_methodologies_dir() -> Result<PathBuf, String> {
    let home = get_home_dir()?;
    Ok(home.join(".ccconfig/methodologies"))
}

/// Get path to active methodology file
fn get_active_methodology_file() -> Result<PathBuf, String> {
    let home = get_home_dir()?;
    Ok(home.join(".ccconfig/active_methodology.json"))
}

/// Read active methodology state
fn read_active_methodology() -> Result<Option<String>, String> {
    let file_path = get_active_methodology_file()?;
    if !file_path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read active methodology: {}", e))?;
    let state: MethodologyState = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse active methodology: {}", e))?;
    Ok(state.active_methodology)
}

/// Write active methodology state
fn write_active_methodology(methodology_id: Option<&str>) -> Result<(), String> {
    let file_path = get_active_methodology_file()?;
    let state = MethodologyState {
        active_methodology: methodology_id.map(String::from),
        last_switched: Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64
        ),
    };
    let content = serde_json::to_string_pretty(&state)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    std::fs::write(&file_path, content)
        .map_err(|e| format!("Failed to write: {}", e))?;
    Ok(())
}

/// List all available methodologies
#[tauri::command]
pub async fn list_methodologies() -> Result<Vec<Methodology>, String> {
    let methodologies_dir = get_methodologies_dir()?;
    let active_id = read_active_methodology()?;

    if !methodologies_dir.exists() {
        return Ok(vec![]);
    }

    let mut methodologies = Vec::new();

    let entries = std::fs::read_dir(&methodologies_dir)
        .map_err(|e| format!("Failed to read methodologies dir: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("manifest.json");
        if !manifest_path.exists() {
            continue;
        }

        match MethodologyManifest::load(&path) {
            Ok(manifest) => {
                methodologies.push(Methodology {
                    id: manifest.id.clone(),
                    name: manifest.name,
                    description: manifest.description,
                    version: manifest.version,
                    color: manifest.color,
                    icon: manifest.icon,
                    path: path.to_string_lossy().to_string(),
                    skills_count: manifest.skills.len(),
                    commands_count: manifest.commands.len(),
                    agents_count: manifest.agents.len(),
                    is_active: active_id.as_ref() == Some(&manifest.id),
                });
            }
            Err(e) => {
                println!("Warning: Failed to load methodology at {:?}: {}", path, e);
            }
        }
    }

    Ok(methodologies)
}

/// Get the currently active methodology
#[tauri::command]
pub async fn get_active_methodology() -> Result<Option<Methodology>, String> {
    let active_id = read_active_methodology()?;

    match active_id {
        Some(id) => {
            let methodologies = list_methodologies().await?;
            Ok(methodologies.into_iter().find(|m| m.id == id))
        }
        None => Ok(None)
    }
}

/// Switch to a different methodology
/// This clears skills/commands/agents and copies from the methodology
#[tauri::command]
pub async fn switch_methodology(methodology_id: String) -> Result<(), String> {
    let methodologies_dir = get_methodologies_dir()?;
    let methodology_path = methodologies_dir.join(&methodology_id);

    if !methodology_path.exists() {
        return Err(format!("Methodology '{}' not found", methodology_id));
    }

    let manifest = MethodologyManifest::load(&methodology_path)?;
    let claude_dir = get_claude_dir()?;

    println!("Switching to methodology: {}", manifest.name);

    // 1. Clear existing skills, commands, agents in ~/.claude
    let items_to_clear = ["skills", "commands", "agents"];
    for item in items_to_clear {
        let item_path = claude_dir.join(item);
        if item_path.exists() && !item_path.is_symlink() {
            std::fs::remove_dir_all(&item_path)
                .map_err(|e| format!("Failed to clear {}: {}", item, e))?;
        }
    }

    // 2. Create directories
    for item in items_to_clear {
        let item_path = claude_dir.join(item);
        std::fs::create_dir_all(&item_path)
            .map_err(|e| format!("Failed to create {}: {}", item, e))?;
    }

    // 3. Copy skills from methodology
    let src_skills = methodology_path.join("skills");
    if src_skills.exists() {
        for skill in &manifest.skills {
            let src = src_skills.join(skill);
            let dst = claude_dir.join("skills").join(skill);
            if src.exists() {
                copy_dir_recursive(&src, &dst)?;
                println!("  Copied skill: {}", skill);
            }
        }
    }

    // 4. Copy commands from methodology
    let src_commands = methodology_path.join("commands");
    if src_commands.exists() {
        for command in &manifest.commands {
            let src = src_commands.join(command);
            let dst = claude_dir.join("commands").join(command);
            if src.exists() {
                if src.is_dir() {
                    copy_dir_recursive(&src, &dst)?;
                } else {
                    if let Some(parent) = dst.parent() {
                        std::fs::create_dir_all(parent).ok();
                    }
                    std::fs::copy(&src, &dst)
                        .map_err(|e| format!("Failed to copy command: {}", e))?;
                }
                println!("  Copied command: {}", command);
            }
        }
    }

    // 5. Copy agents from methodology
    let src_agents = methodology_path.join("agents");
    if src_agents.exists() {
        for agent in &manifest.agents {
            let src = src_agents.join(agent);
            let dst = claude_dir.join("agents").join(agent);
            if src.exists() {
                std::fs::copy(&src, &dst)
                    .map_err(|e| format!("Failed to copy agent: {}", e))?;
                println!("  Copied agent: {}", agent);
            }
        }
    }

    // 6. Update active methodology
    write_active_methodology(Some(&methodology_id))?;

    println!("Successfully switched to methodology: {}", manifest.name);
    Ok(())
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create dir {:?}: {}", dst, e))?;

    for entry in std::fs::read_dir(src)
        .map_err(|e| format!("Failed to read dir {:?}: {}", src, e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy {:?}: {}", src_path, e))?;
        }
    }
    Ok(())
}
