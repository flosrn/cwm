use crate::commands::utils::get_claude_dir;

/// Check if ~/.claude is a git repository
pub fn git_is_repo() -> Result<bool, String> {
    let claude_dir = get_claude_dir()?;
    let git_dir = claude_dir.join(".git");
    Ok(git_dir.exists())
}

/// Get current git branch name
pub fn git_current_branch() -> Result<String, String> {
    let claude_dir = get_claude_dir()?;

    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&claude_dir)
        .output()
        .map_err(|e| format!("Failed to run git command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Git command failed: {}", stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Check if there are uncommitted changes (staged or unstaged)
pub fn git_has_changes() -> Result<bool, String> {
    let claude_dir = get_claude_dir()?;

    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&claude_dir)
        .output()
        .map_err(|e| format!("Failed to run git status: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Git status failed: {}", stderr));
    }

    let status = String::from_utf8_lossy(&output.stdout);
    Ok(!status.trim().is_empty())
}

/// Switch git branch reference WITHOUT checkout (no file operations)
/// This avoids the Ghostty crash by not triggering file system events
pub fn git_switch_branch_ref(branch_name: &str) -> Result<(), String> {
    let claude_dir = get_claude_dir()?;

    let full_branch = if branch_name.starts_with("refs/") {
        branch_name.to_string()
    } else if branch_name.starts_with("workspace/") {
        format!("refs/heads/{}", branch_name)
    } else {
        format!("refs/heads/workspace/{}", branch_name)
    };

    // Check if the branch exists
    let check_output = std::process::Command::new("git")
        .args(["show-ref", "--verify", "--quiet", &full_branch])
        .current_dir(&claude_dir)
        .output()
        .map_err(|e| format!("Failed to check branch: {}", e))?;

    if !check_output.status.success() {
        // Branch doesn't exist, create it from current HEAD
        let create_output = std::process::Command::new("git")
            .args(["branch", branch_name.trim_start_matches("workspace/")])
            .current_dir(&claude_dir)
            .output()
            .map_err(|e| format!("Failed to create branch: {}", e))?;

        if !create_output.status.success() {
            let stderr = String::from_utf8_lossy(&create_output.stderr);
            // Ignore "already exists" error
            if !stderr.contains("already exists") {
                return Err(format!("Failed to create branch: {}", stderr));
            }
        }
    }

    // Switch HEAD to point to the new branch (no file checkout)
    let output = std::process::Command::new("git")
        .args(["symbolic-ref", "HEAD", &full_branch])
        .current_dir(&claude_dir)
        .output()
        .map_err(|e| format!("Failed to switch branch ref: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Git symbolic-ref failed: {}", stderr));
    }

    // Reset index to match HEAD (soft reset, no file changes)
    let reset_output = std::process::Command::new("git")
        .args(["reset", "--soft", "HEAD"])
        .current_dir(&claude_dir)
        .output()
        .map_err(|e| format!("Failed to reset index: {}", e))?;

    if !reset_output.status.success() {
        // Non-fatal, just log
        let stderr = String::from_utf8_lossy(&reset_output.stderr);
        println!("Warning: git reset soft failed (non-fatal): {}", stderr);
    }

    println!("Switched git branch ref to: {}", branch_name);
    Ok(())
}

/// Auto-commit all changes with a message
pub fn git_auto_commit(message: &str) -> Result<(), String> {
    let claude_dir = get_claude_dir()?;

    // Check if there are changes to commit
    if !git_has_changes()? {
        println!("No changes to commit");
        return Ok(());
    }

    // Stage all changes
    let add_output = std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(&claude_dir)
        .output()
        .map_err(|e| format!("Failed to run git add: {}", e))?;

    if !add_output.status.success() {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        return Err(format!("Git add failed: {}", stderr));
    }

    // Commit with message
    let commit_output = std::process::Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(&claude_dir)
        .output()
        .map_err(|e| format!("Failed to run git commit: {}", e))?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        // It's okay if there's nothing to commit
        if !stderr.contains("nothing to commit") {
            return Err(format!("Git commit failed: {}", stderr));
        }
    }

    println!("Git auto-commit: {}", message);
    Ok(())
}

/// Managed items that are part of workspace switching
const MANAGED_ITEMS: &[&str] = &[
    "settings.json",
    "CLAUDE.md",
    ".mcp.json",
    "skills",
    "commands",
    "agents",
    "rules",
    "plugins",
    "docs",
    "chrome",
    "song",
    "scripts",
];

/// Auto-commit only managed workspace items (not user's work-in-progress files)
pub fn git_auto_commit_managed(message: &str) -> Result<(), String> {
    let claude_dir = get_claude_dir()?;

    // Stage only managed items (ignore errors for items that don't exist)
    for item in MANAGED_ITEMS {
        let item_path = claude_dir.join(item);
        if item_path.exists() {
            let _ = std::process::Command::new("git")
                .args(["add", item])
                .current_dir(&claude_dir)
                .output();
        } else {
            // Item was deleted, stage the deletion
            let _ = std::process::Command::new("git")
                .args(["add", item])
                .current_dir(&claude_dir)
                .output();
        }
    }

    // Check if there are staged changes
    let diff_output = std::process::Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(&claude_dir)
        .output()
        .map_err(|e| format!("Failed to check staged changes: {}", e))?;

    // Exit code 0 = no changes, 1 = has changes
    if diff_output.status.success() {
        println!("No managed items to commit");
        return Ok(());
    }

    // Commit with message
    let commit_output = std::process::Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(&claude_dir)
        .output()
        .map_err(|e| format!("Failed to run git commit: {}", e))?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        if !stderr.contains("nothing to commit") {
            return Err(format!("Git commit failed: {}", stderr));
        }
    }

    println!("Git auto-commit (managed items only): {}", message);
    Ok(())
}
