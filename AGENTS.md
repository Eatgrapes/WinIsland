# WinIsland — AI Agent Guide

WinIsland is a **Dynamic Island for Windows**, written in Rust. It replicates the iPhone Dynamic Island experience on Windows, integrating with System Media Transport Controls (SMTC) to display now-playing media, real-time lyrics, audio spectrum visualization, and more.

## Quick start

```bash
cargo check          # Fast type-check
cargo clippy --workspace -- -D warnings   # Lint (warnings as errors)
cargo fmt --all      # Format all files
cargo build --release   # Production build
cargo test           # Run tests
```

## Key files to read

| File | What it contains |
|------|-----------------|
| [.agents/ARCHITECTURE.md](.agents/ARCHITECTURE.md) | Project structure, module responsibilities, rendering pipeline, plugin system, Windows API usage |
| [.agents/STYLE-GUIDE.md](.agents/STYLE-GUIDE.md) | Coding conventions, naming, unsafe safety comments, Skia patterns |
| [.agents/skills/build-winisland/SKILL.md](.agents/skills/pr-check/SKILL.md) | Build & test automation for AI agents |
| [.agents/skills/commit-winisland/SKILL.md](.agents/skills/commit-push/SKILL.md) | Commit message generation for AI agents |

## Agent behavior rules

1. **Never commit changes** unless the user explicitly asks you to.
2. **Never create README or documentation files** unless the user asks.
3. **Do not use emoji** unless the user explicitly requests it.
4. **Always look at existing code conventions** before writing new code — mimic imports, patterns, and styles from neighboring files.
5. **Check dependencies exist** before using a library — look at Cargo.toml or neighboring imports first.
6. **Follow security best practices** — never log or commit secrets/keys.
7. **No unnecessary comments** — do NOT add comments in code unless asked.
8. Prefer editing existing files over creating new ones.
9. **WDA_EXCLUDEFROMCAPTURE is intentionally NOT set** — see glass.rs doc comments for rationale.
