use crate::commands::utils::get_home_dir;

// ============================================================================
// TYPES
// ============================================================================

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct CommandFile {
    pub name: String,
    pub content: String,
    pub exists: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct AgentFile {
    pub name: String,
    pub content: String,
    pub exists: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct SkillFile {
    pub name: String,
    pub content: String,
    pub references_count: u32,
}

// ============================================================================
// COMMANDS TAURI COMMANDS
// ============================================================================

#[tauri::command]
pub async fn read_claude_commands() -> Result<Vec<CommandFile>, String> {
    let home_dir = get_home_dir()?;
    let commands_dir = home_dir.join(".claude/commands");

    if !commands_dir.exists() {
        return Ok(vec![]);
    }

    let mut command_files = Vec::new();

    let entries = std::fs::read_dir(&commands_dir)
        .map_err(|e| format!("Failed to read commands directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if path.is_file() && path.extension().map(|ext| ext == "md").unwrap_or(false) {
            let file_name = path
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown")
                .to_string();

            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read command file {}: {}", path.display(), e))?;

            command_files.push(CommandFile {
                name: file_name,
                content,
                exists: true,
            });
        }
    }

    command_files.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(command_files)
}

#[tauri::command]
pub async fn write_claude_command(command_name: String, content: String) -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let commands_dir = home_dir.join(".claude/commands");
    let command_file_path = commands_dir.join(format!("{}.md", command_name));

    std::fs::create_dir_all(&commands_dir)
        .map_err(|e| format!("Failed to create .claude/commands directory: {}", e))?;

    std::fs::write(&command_file_path, content)
        .map_err(|e| format!("Failed to write command file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn delete_claude_command(command_name: String) -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let commands_dir = home_dir.join(".claude/commands");
    let command_file_path = commands_dir.join(format!("{}.md", command_name));

    if command_file_path.exists() {
        std::fs::remove_file(&command_file_path)
            .map_err(|e| format!("Failed to delete command file: {}", e))?;
    }

    Ok(())
}

// ============================================================================
// AGENTS TAURI COMMANDS
// ============================================================================

#[tauri::command]
pub async fn read_claude_agents() -> Result<Vec<AgentFile>, String> {
    let home_dir = get_home_dir()?;
    let agents_dir = home_dir.join(".claude/agents");

    if !agents_dir.exists() {
        return Ok(vec![]);
    }

    let mut agent_files = Vec::new();

    let entries = std::fs::read_dir(&agents_dir)
        .map_err(|e| format!("Failed to read agents directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if path.is_file() && path.extension().map(|ext| ext == "md").unwrap_or(false) {
            let file_name = path
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown")
                .to_string();

            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read agent file {}: {}", path.display(), e))?;

            agent_files.push(AgentFile {
                name: file_name,
                content,
                exists: true,
            });
        }
    }

    agent_files.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(agent_files)
}

#[tauri::command]
pub async fn write_claude_agent(agent_name: String, content: String) -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let agents_dir = home_dir.join(".claude/agents");
    let agent_file_path = agents_dir.join(format!("{}.md", agent_name));

    std::fs::create_dir_all(&agents_dir)
        .map_err(|e| format!("Failed to create .claude/agents directory: {}", e))?;

    std::fs::write(&agent_file_path, content)
        .map_err(|e| format!("Failed to write agent file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn delete_claude_agent(agent_name: String) -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let agents_dir = home_dir.join(".claude/agents");
    let agent_file_path = agents_dir.join(format!("{}.md", agent_name));

    if agent_file_path.exists() {
        std::fs::remove_file(&agent_file_path)
            .map_err(|e| format!("Failed to delete agent file: {}", e))?;
    }

    Ok(())
}

// ============================================================================
// SKILLS TAURI COMMANDS
// ============================================================================

#[tauri::command]
pub async fn read_claude_skills() -> Result<Vec<SkillFile>, String> {
    let home_dir = get_home_dir()?;
    let skills_dir = home_dir.join(".claude/skills");

    if !skills_dir.exists() {
        return Ok(vec![]);
    }

    let mut skill_files = Vec::new();

    let entries = std::fs::read_dir(&skills_dir)
        .map_err(|e| format!("Failed to read skills directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            let skill_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown")
                .to_string();

            let skill_md_path = path.join("SKILL.md");
            let content = if skill_md_path.exists() {
                std::fs::read_to_string(&skill_md_path).unwrap_or_else(|_| String::new())
            } else {
                String::new()
            };

            let references_dir = path.join("references");
            let references_count = if references_dir.exists() {
                std::fs::read_dir(&references_dir)
                    .map(|entries| entries.count() as u32)
                    .unwrap_or(0)
            } else {
                0
            };

            skill_files.push(SkillFile {
                name: skill_name,
                content,
                references_count,
            });
        }
    }

    skill_files.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(skill_files)
}

#[tauri::command]
pub async fn write_claude_skill(skill_name: String, content: String) -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let skill_dir = home_dir.join(".claude/skills").join(&skill_name);
    let skill_file_path = skill_dir.join("SKILL.md");

    std::fs::create_dir_all(&skill_dir)
        .map_err(|e| format!("Failed to create skill directory: {}", e))?;

    std::fs::write(&skill_file_path, content)
        .map_err(|e| format!("Failed to write skill file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn delete_claude_skill(skill_name: String) -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let skill_dir = home_dir.join(".claude/skills").join(&skill_name);

    if skill_dir.exists() {
        std::fs::remove_dir_all(&skill_dir)
            .map_err(|e| format!("Failed to delete skill directory: {}", e))?;
    }

    Ok(())
}
