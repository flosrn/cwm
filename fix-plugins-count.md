# Bug Fix: Plugin Count Always Shows 0 in CWM

## Problem

In CWM (Claude Workspace Manager), the workspace cards on the Configurations page show `0 plugins` even when plugins exist in `~/.claude/plugins/local/`.

Screenshot shows: `25 skills  30 cmds  18 agents  0 plugins` - but plugins DO exist.

## Context

CWM is a Tauri + React app that manages Claude Code workspaces. Each workspace stores metadata including counts for skills, commands, agents, and plugins.

### Current Plugin Structure in ~/.claude/

```
~/.claude/plugins/
├── installed_plugins.json      # Registry of installed plugins
├── cache/                      # New: cached plugin installations
│   └── local/
│       └── makerkit/1.0.0/     # Actual plugin content
├── local/                      # Plugin source directories
│   ├── makerkit/
│   └── next-react-optimizer/
└── marketplaces/               # Marketplace plugins
```

### Current installed_plugins.json Format

```json
{
  "version": 2,
  "plugins": {
    "makerkit@local": [
      {
        "scope": "user",
        "installPath": "/Users/flo/.claude/plugins/cache/local/makerkit/1.0.0",
        "version": "1.0.0",
        "installedAt": "2026-01-18T17:39:22.558Z"
      }
    ]
  }
}
```

## Files to Investigate

1. **`src-tauri/src/commands.rs`** - Contains `count_workspace_items()` and `count_subdirectories()` functions
2. **`src-tauri/src/commands.rs`** - Function `sync_workspace_from_claude` that updates metadata
3. **`src/pages/ConfigSwitcherPage.tsx`** - Displays `store.pluginsCount`

## Current Implementation (likely buggy)

```rust
// In commands.rs around line 383
fn count_workspace_items(workspace_path: &str) -> Result<(Option<u32>, Option<u32>, Option<u32>, Option<u32>), String> {
    let path = std::path::Path::new(workspace_path);

    let skills_count = count_directory_items(&path.join("skills"));
    let commands_count = count_directory_items(&path.join("commands"));
    let agents_count = count_directory_items(&path.join("agents"));
    // Plugins are directories, not .md files
    let plugins_count = count_subdirectories(&path.join("plugins/local"));

    Ok((Some(skills_count), Some(commands_count), Some(agents_count), Some(plugins_count)))
}
```

## Potential Issues to Investigate

1. **Wrong path**: Workspace path might be `~/.ccconfig/workspaces/ws_{id}/` - does it contain `plugins/local/`?

2. **Plugins not copied**: When syncing workspace, are plugins being copied from `~/.claude/plugins/` to the workspace?

3. **New plugin format**: Claude Code might now use `installed_plugins.json` registry instead of scanning `plugins/local/` directory

4. **count_subdirectories function**: Verify it correctly counts non-hidden directories

## Tasks

1. Check workspace content: `ls -la ~/.ccconfig/workspaces/ws_default/plugins/`

2. Verify if plugins directory is being copied during `copy_claude_to_workspace()`

3. Consider reading from `installed_plugins.json` instead of scanning directories

4. Test the `count_subdirectories()` function independently

5. After fixing, user needs to click sync button (🔄) to refresh metadata

## Expected Result

After fix, workspace cards should show accurate plugin count like:
`25 skills  30 cmds  18 agents  2 plugins`
