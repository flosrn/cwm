use crate::commands::utils::get_home_dir;

// ============================================================================
// TYPES
// ============================================================================

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct MemoryFile {
    pub path: String,
    pub content: String,
    pub exists: bool,
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

#[tauri::command]
pub async fn read_claude_memory() -> Result<MemoryFile, String> {
    let home_dir = get_home_dir()?;
    let claude_md_path = home_dir.join(".claude/CLAUDE.md");

    let path_str = claude_md_path.to_string_lossy().to_string();

    if claude_md_path.exists() {
        let content = std::fs::read_to_string(&claude_md_path)
            .map_err(|e| format!("Failed to read CLAUDE.md file: {}", e))?;

        Ok(MemoryFile {
            path: path_str,
            content,
            exists: true,
        })
    } else {
        Ok(MemoryFile {
            path: path_str,
            content: String::new(),
            exists: false,
        })
    }
}

#[tauri::command]
pub async fn write_claude_memory(content: String) -> Result<(), String> {
    let home_dir = get_home_dir()?;
    let claude_md_path = home_dir.join(".claude/CLAUDE.md");

    // Ensure .claude directory exists
    if let Some(parent) = claude_md_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create .claude directory: {}", e))?;
    }

    std::fs::write(&claude_md_path, content)
        .map_err(|e| format!("Failed to write CLAUDE.md file: {}", e))?;

    Ok(())
}
