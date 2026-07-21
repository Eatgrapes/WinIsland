# WinIsland Style Guide

## Language

All code, comments, and commit messages are in **English** unless the project context explicitly requires otherwise.

## General rules

1. **No unnecessary comments** — code should be self-documenting where possible. Exceptions: `// SAFETY:` for unsafe blocks, complex business logic.
2. **No emoji** in code, comments, or commit messages unless the user explicitly asks.
3. **Never commit changes** unless the user explicitly asks you to.
4. **Never create README or documentation files** unless the user asks.
5. Prefer editing existing files over creating new ones.

## Naming

| Category | Convention | Example |
|----------|-----------|---------|
| Types/structs/enums | PascalCase | `NativePlugin`, `PluginError` |
| Functions/methods | snake_case | `get_glass_background` |
| Variables | snake_case | `screen_x`, `cached_img` |
| Constants/statics | SCREAMING_SNAKE_CASE | `SKSL_SOURCE`, `MAX_FILENAME_COMPONENT` |
| Type aliases | PascalCase | `BgCacheEntry` |
| Thread-locals | SCREAMING_SNAKE_CASE | `GLASS_CACHE`, `EFFECT_CACHE` |
| Module/file names | snake_case | `glass.rs`, `zip_loader.rs` |

## Module responsibility

Each module has a single responsibility:
- `src/core/render.rs` — all Skia drawing, and only drawing
- `src/window/app.rs` — event loop, state, input handling
- `src/utils/glass.rs` — frosted glass effect (and nothing else)

**Do not** add unrelated logic to an existing module. Create a new module if the functionality is distinct.

## Unsafe code

Every `unsafe` block **MUST** have a `// SAFETY:` comment above it explaining why the operation is safe.

Good:
```rust
// SAFETY: hwnd was validated via find_window which checks is_invalid()
// before returning. PostMessageW sends a message through the HWND
// without accessing any memory through it.
unsafe {
    let _ = PostMessageW(hwnd, WM_CLOSE, None, None);
}
```

Bad (no comment):
```rust
unsafe {
    let _ = PostMessageW(hwnd, WM_CLOSE, None, None);
}
```

## Import style

Group imports in this order, separated by blank lines:
1. Standard library (`use std::...`)
2. External crates (`use skia_safe::...`, `use windows::...`)
3. Internal crate (`use crate::...`)

## Error handling

- Use `Option` for recoverable absence of a value
- Use `Result<T, String>` for errors where the message is user-facing
- Propagate errors with `?` where possible
- Log errors with `error!()` macro for unexpected failures

## Skia conventions

- Use cached `gpu::surfaces::render_target` surfaces for offscreen rendering
- `Paint::default()` then configure only what differs from defaults
- Anti-alias shape-drawing paints via `paint.set_anti_alias(true)`
- Use `image_filters::blur` for blur effects, not manual convolution
- Cache compiled `RuntimeEffect` (SKSL) in thread-locals

## Windows API conventions

- Use the `windows` crate, not `winapi` or raw FFI
- Always check handle validity with `.is_invalid()` after `GetDC`, `CreateCompatibleDC`, etc.
- Release resources in reverse acquisition order
- Use `_ = ...` to discard return values for fire-and-forget API calls

## Thread safety

- Thread-local storage (`thread_local!`) for caches that don't cross thread boundaries
- `AtomicUsize` / `AtomicBool` for simple cross-thread state
- `RwLock` for read-heavy shared state (e.g., `PluginManager`)
- `Arc<RwLock<>>` for long-lived shared state (e.g., `I18n` singleton)
