---
name: WinIsland PR Check
description: Build and lint the WinIsland Rust project. Use when the user wants to compile, check for errors, run clippy, format code, or execute tests. Trigger after every code change to verify correctness.
license: GPL-3.0
metadata:
  project: WinIsland
---

Run these commands from the project root in order. Stop and report any failures before proceeding.

## 1. Type-check

```bash
cargo check
```

This is the fastest way to detect compilation errors. Run this first.

## 2. Lint

```bash
cargo clippy --workspace -- -D warnings
```

Warnings are treated as errors. If clippy reports any, fix them before continuing.

## 3. Format

```bash
cargo fmt --all
```

Ensure all files are formatted. If the command makes changes, re-run step 1 and 2.

## 4. Release build (optional)

Only when the user explicitly asks for a release build:

```bash
cargo build --release
```

## Common issues

- **`link.exe not found`**: Ensure Visual Studio build tools or LLVM/clang are installed.
- **`ninja not found`**: Install via `choco install ninja` or ensure it's on PATH.
- **Skia linking errors**: The `skia-safe` crate bundles pre-built binaries; a network connection may be needed on first build.
