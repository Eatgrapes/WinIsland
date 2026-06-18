---
name: commit-push
description: Generate a Conventional Commit message from the current git diff and commit. Use when the user says 'commit', '/commit', 'commit changes', or asks to create a git commit.
license: GPL-3.0
metadata:
  project: WinIsland
---

Analyze the staged and unstaged changes, then generate a commit message following the project's existing style. The project uses **Conventional Commits** with optional scopes matching module names.

## Commit message format

```
<type>(<scope>): <short description>

<body (optional)>
```

### Types

| Type | When to use |
|------|------------|
| `feat` | A new feature |
| `fix` | A bug fix |
| `refactor` | Code restructuring without feature/bug changes |
| `perf` | Performance improvement |
| `style` | Formatting, whitespace, etc. (no logic change) |
| `docs` | Documentation changes |
| `chore` | Maintenance, config, dependencies |

### Scopes

Match the scope to the module or subsystem affected:

| Scope | Module |
|-------|--------|
| `lyrics` | `src/core/lyrics.rs` or lyrics-related changes |
| `smtc` | `src/core/smtc.rs` — SMTC/media integration |
| `render` | `src/core/render.rs` — rendering |
| `glass` | `src/utils/glass.rs` — glass effect |
| `plugin` | `src/plugin/` or `crates/winisland-plugin-api/` |
| `settings` | `src/window/settings/` or settings UI |
| `packager` | `crates/winisland-plugin-api/src/packager/` |
| `window` | `src/window/` — window management |
| (none) | Use no scope for cross-cutting changes |

### Examples from the project

```
feat(lyrics): prevent unrelated lyrics for browser video sessions and add local lyrics
fix: filter empty path in RowFolderPicker clear_label and current_path
refactor: restructure rendering pipeline, optimize code quality, fix plugin security issues
perf(glass): capture background once, cache by position, reduce blur CPU usage
style: fix fmt formatting in packager mod.rs
```

## Procedure

1. **Stage files logically**: Group related changes together. Do NOT stage everything at once if changes span unrelated areas.

   ```bash
   # Stage only the files relevant to this commit
   git add <file1> <file2>
   ```

2. **Generate the message**: Based on the staged diff, write a commit message following the format above.

3. **Commit**:

   ```bash
   git commit -m "<type>(<scope>): <description>"
   ```

4. **If the user has a fork + PR setup**: Ask whether to push. If yes, push to the fork's branch:

   ```bash
   git push
   ```

## Rules

- Short description: imperative mood, lowercase, no period at end, max ~72 chars
- Body (if needed): wrap at 72 chars, explain *why* not *what*
- If the change fixes an issue, reference it in the body: `Fixes #123`
- Never commit unless the user explicitly asks
