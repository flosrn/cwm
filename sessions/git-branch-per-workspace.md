# CWM : Gestion automatique de Git avec branches par workspace

## Contexte

CWM (Claude Workspace Manager) est une app Tauri qui permet de switcher entre différentes configurations Claude Code. Chaque workspace contient des skills, commands, agents, plugins, et settings.

**Problème actuel :** Le dossier `~/.claude/` est un repo git. Quand CWM switch de workspace, les fichiers sont remplacés mais le `.git/` reste le même. Résultat : git voit des tonnes de modifications, et Claude Code affiche ces changements de manière désordonnée dans les conversations.

## Solution souhaitée

Implémenter une gestion automatique de git dans CWM :

1. **Une branche git par workspace** (ex: `workspace/default`, `workspace/light`)
2. **Lors du switch de workspace :**
   - Commit automatique des changements sur la branche actuelle
   - Checkout de la branche du workspace cible
   - Si la branche n'existe pas, la créer
3. **Lors de la sync d'un workspace :**
   - Commit les changements sur la branche correspondante

## Fichiers à modifier

### Backend Rust
- `/Users/flo/projects/claude/cwm/src-tauri/src/commands.rs`
  - Modifier `set_using_config()` (ligne ~1026) pour gérer git
  - Modifier `sync_workspace_from_claude()` (ligne ~621) pour commit
  - Ajouter des fonctions helper pour git (commit, checkout, create branch)

### Structure actuelle du switch (à modifier)

```rust
// commands.rs:1048-1067
match selected_store.workspace_type {
    WorkspaceType::FullDirectory => {
        // 1. Backup current ~/.claude before switch
        let backup_path = backup_current_claude_dir()?;

        // 2. Clear ~/.claude (managed items only)
        clear_claude_dir_for_switch()?;

        // 3. Copy workspace to ~/.claude
        copy_workspace_to_claude(workspace_path)?;
    }
}
```

### Nouveau flow proposé

```rust
match selected_store.workspace_type {
    WorkspaceType::FullDirectory => {
        // 1. Get current branch name
        let current_branch = git_current_branch()?;

        // 2. Commit pending changes on current branch (if any)
        git_auto_commit(&format!("Auto-save before switching to {}", store_id))?;

        // 3. Checkout or create target branch
        let target_branch = format!("workspace/{}", store_id);
        git_checkout_or_create(&target_branch)?;

        // 4. NO MORE backup/clear/copy needed!
        // Git handles everything via branches
    }
}
```

## Fonctions git à implémenter

```rust
/// Get current git branch name
fn git_current_branch() -> Result<String, String>

/// Check if there are uncommitted changes
fn git_has_changes() -> Result<bool, String>

/// Auto-commit all changes with a message
fn git_auto_commit(message: &str) -> Result<(), String>

/// Checkout existing branch or create new one
fn git_checkout_or_create(branch_name: &str) -> Result<(), String>

/// Check if a branch exists
fn git_branch_exists(branch_name: &str) -> Result<bool, String>
```

## Comportement attendu

### Premier lancement (migration)
1. Détecter si `~/.claude/` est un repo git
2. Si oui, créer les branches pour les workspaces existants
3. Chaque branche contient l'état du workspace correspondant

### Switch de workspace
```
User clique sur "Light"
    ↓
[Si changes non commités sur branche actuelle]
    → git add -A && git commit -m "Auto-save: workspace/default"
    ↓
[Checkout branche cible]
    → git checkout workspace/light
    ↓
[Si branche n'existe pas]
    → git checkout -b workspace/light
    → Copier le contenu du workspace stocké
    → git add -A && git commit -m "Initialize workspace/light"
    ↓
Done! Les fichiers sont maintenant ceux de "Light"
```

### Sync d'un workspace
```
User clique sur 🔄 (sync)
    ↓
[Commit les changements]
    → git add -A && git commit -m "Sync: workspace/light"
    ↓
[Copier vers le workspace stocké]
    → ~/.claude/* → ~/.ccconfig/workspaces/ws_light/
```

## Cas edge à gérer

1. **`~/.claude/` n'est pas un repo git** → Initialiser git ou continuer sans
2. **Conflits de merge** → Ne pas faire de merge auto, juste des checkouts
3. **Branche main/master** → La préserver, ne pas la toucher
4. **Fichiers non trackés** → Les inclure dans le commit auto

## Tests à effectuer

1. Switch Default → Light → Default : vérifier que les fichiers changent correctement
2. Modifier un skill sur Light, switch vers Default, revenir sur Light : le skill modifié doit être là
3. Vérifier que `git log` montre les commits auto
4. Vérifier que les branches `workspace/*` sont créées

## Notes importantes

- Utiliser `std::process::Command` pour exécuter les commandes git
- Le dossier git est `~/.claude/` (pas le workspace stocké)
- Garder la compatibilité avec les workspaces `SettingsOnly` (pas de git pour eux)
- Les workspaces stockés dans `~/.ccconfig/workspaces/` restent pour le backup/référence

## Commandes git utiles

```bash
# Current branch
git rev-parse --abbrev-ref HEAD

# Check for changes
git status --porcelain

# Commit all
git add -A && git commit -m "message"

# Checkout or create
git checkout branch_name || git checkout -b branch_name

# Check if branch exists
git show-ref --verify --quiet refs/heads/branch_name
```
