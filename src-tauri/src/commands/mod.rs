// ============================================================================
// Module declarations
// ============================================================================

pub mod config;
pub mod git;
pub mod git_import;
pub mod hooks;
pub mod mcp;
pub mod memory;
pub mod plugins;
pub mod projects;
pub mod skills;
pub mod stores;
pub mod updates;
pub mod utils;
pub mod workspace;

// ============================================================================
// Public re-exports - Tauri Commands
// ============================================================================

// Config commands
pub use config::{
    backup_claude_configs, check_app_config_exists, create_app_config_dir, initialize_app_config,
    list_config_files, open_config_path, read_claude_config_file, read_config_file,
    write_claude_config_file, write_config_file,
};

// Stores commands
pub use stores::{
    create_config, delete_config, get_current_store, get_store, get_stores, reset_to_original_config,
    set_using_config, update_config,
};

// Workspace commands
pub use workspace::{get_claude_dir_counts, refresh_workspace_counts, sync_workspace_from_claude};

// MCP commands
pub use mcp::{
    check_mcp_server_exists, delete_global_mcp_server, get_global_mcp_servers,
    update_global_mcp_server,
};

// Memory commands
pub use memory::{read_claude_memory, write_claude_memory};

// Projects commands
pub use projects::{read_claude_projects, read_project_usage_files};

// Skills commands
pub use skills::{
    delete_claude_agent, delete_claude_command, delete_claude_skill, read_claude_agents,
    read_claude_commands, read_claude_skills, write_claude_agent, write_claude_command,
    write_claude_skill,
};

// Plugins commands
pub use plugins::{delete_local_plugin, read_claude_plugins, toggle_plugin};

// Hooks commands
pub use hooks::{
    add_claude_code_hook, get_notification_settings, remove_claude_code_hook,
    update_claude_code_hook, update_notification_settings,
};

// Updates commands
pub use updates::{
    check_for_updates, install_and_restart, rebuild_tray_menu_command, track, unlock_cc_ext,
};

// Git import commands
pub use git_import::{import_workspace_from_git, preview_git_import};
