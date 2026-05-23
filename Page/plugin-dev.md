# Plugin Development Guide

Welcome! (´｡• ᵕ •｡`)♡ You're about to extend WinIsland with your own plugin.

> ⚠️ **Note: The plugin system is currently in a foundation stage.** The C ABI type definitions are ready, but the host-side trait interfaces (`ContentProvider`, `ThemeProvider`, `ShortcutProvider`) **are not yet wired into the render pipeline**. See [issue #55](https://github.com/Eatgrapes/WinIsland/issues/55) — we'd really love your input there!

## How Plugins Work

WinIsland uses a **C ABI vtable** pattern to load native `.dll` plugins safely. Think of it like this:

```
WinIsland.exe  ──libloading──▶  your_plugin.dll
   │                                  │
   │  PluginManager                   │  exports plugin_get_instance()
   │  └─ Vec<NativePlugin>            │  returns PluginInstanceC {
   │       ├─ metadata (id, name…)    │    handle: opaque ptr
   │       ├─ handle (opaque ptr)     │    vtable: function ptrs
   │       └─ vtable (fn ptrs)        │    metadata: PluginMetadataC
   │                                  │  }
   └── calls traits ──▶  through vtable ──▶  your code runs!
```

All data crossing the FFI boundary is `#[repr(C)]` — flat structs with no `Vec`, `String`, or trait objects. This means your plugin can be compiled with any Rust version and it'll still work (ﾉ◕ヮ◕)ﾉ*:･ﾟ✧

## Plugin Types (Planned)

| Type | Purpose | Status |
|------|---------|--------|
| **Content** (id=1) | Provide custom island content (weather, notifications, status…) | 🔲 API pending |
| **Theme** (id=2) | Override island colors and animation parameters | 🔲 API pending |
| **Shortcut** (id=3) | Register executable actions | 🔲 API pending |

## Project Setup

Create a new Rust library project:

```
cargo new --lib my-winisland-plugin
```

Edit `Cargo.toml`:

```toml
[package]
name = "my-winisland-plugin"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
winisland-plugin-api = { git = "https://github.com/Eatgrapes/WinIsland", branch = "master" }
```

## Packaging as ZIP (ﾉ◕ヮ◕)ﾉ

Your plugin must be packaged as `.zip` to be loaded by WinIsland. The ZIP must contain:

```
my-plugin.zip
├── plugin.yml    ← plugin manifest (required)
└── *.dll         ← plugin binary (required, multiple .dll OK — all will be loaded)
```

### plugin.yml

```yaml
name: example
author: xxx
version: 1.0.0
description: This is example plugin
github-link: example/example-plugin
```

**All 5 fields are required** — missing any will cause install to fail o(TヘTo)

## Installing ฅ^•ﻌ•^ฅ

Simply **drag the `.zip` file onto the island**! The plugin is extracted to `C:\Users\<YourName>\AppData\Roaming\WinIsland\plugins\<plugin-name>\` and auto-loaded.

A Windows notification dialog will confirm successful installation.

You can also manually place `.dll` files into subdirectories under `plugins\` — WinIsland scans them on startup.

## How to Verify Your Plugin Loaded?

Since the host-side API is still under development, plugins won't display anything on the Island UI yet.

**Verification:**
1. Press `F12` to open the WinIsland debug log window
2. Search for your plugin name — you should see something like `Loaded plugin: xxx (xxx)`
3. Dropping a ZIP triggers a Windows popup confirming success/failure

It's a rough skeleton, but it works! (｡･ω･｡)

## C ABI Type Reference

These types live in the `winisland-plugin-api` crate. Before API integration, they're only used for DLL loading validation — **plugin functionality is not yet visible in the UI**.

### PluginResultC

```rust
pub struct PluginResultC {
    pub ok: bool,
    pub error: [u8; 256],  // null-terminated UTF-8
}
```

Use `PluginResultC::ok()` for success, `PluginResultC::err("message")` for failure.

### PluginMetadataC

```rust
pub struct PluginMetadataC {
    pub id: [u8; 64],
    pub name: [u8; 128],
    pub version: [u8; 32],
    pub author: [u8; 128],
    pub description: [u8; 256],
}
```

### IslandContentC

```rust
pub struct IslandContentC {
    pub tag: u32,
    pub title: [u8; 256],
    pub artist: [u8; 256],
    pub cover_url: [u8; 512],
    pub is_playing: bool,
    pub message: [u8; 256],
    pub label: [u8; 128],
    pub value: [u8; 128],
}
```

### PluginVTable

```rust
pub struct PluginVTable {
    pub on_load: unsafe extern "C" fn(PluginHandle) -> PluginResultC,
    pub on_unload: unsafe extern "C" fn(PluginHandle) -> PluginResultC,
    pub destroy: unsafe extern "C" fn(PluginHandle),
    pub get_content: Option<unsafe extern "C" fn(PluginHandle) -> IslandContentC>,
    pub on_click: Option<unsafe extern "C" fn(PluginHandle)>,
    pub on_expanded: Option<unsafe extern "C" fn(PluginHandle, bool)>,
    pub supports_expand: Option<unsafe extern "C" fn(PluginHandle) -> bool>,
    pub get_colors: Option<unsafe extern "C" fn(PluginHandle) -> ThemeColorsC>,
    pub get_animations: Option<unsafe extern "C" fn(PluginHandle) -> AnimationConfigC>,
}
```

### PluginInstanceC

```rust
pub struct PluginInstanceC {
    pub handle: PluginHandle,
    pub metadata: PluginMetadataC,
    pub vtable: *const PluginVTable,
    pub plugin_type: u32, // 1=Content, 2=Theme, 3=Shortcut
}
```

### ThemeColorsC / AnimationConfigC

```rust
pub struct ThemeColorsC {
    pub primary: [u8; 4],    // RGBA
    pub secondary: [u8; 4],
    pub background: [u8; 4],
    pub text: [u8; 4],
    pub border: [u8; 4],
}

pub struct AnimationConfigC {
    pub expand_duration_ms: u32,
    pub collapse_duration_ms: u32,
    pub bounce_intensity: f32,
}
```

## Join the Discussion (づ｡◕‿‿◕｡)づ

Beyond hooking into the Island context, we don't have many concrete directions yet… **we're really short on inspiration QWQ**

Please join us at [#55](https://github.com/Eatgrapes/WinIsland/issues/55) to discuss what you'd like the plugin system to support!

---

Happy hacking! (づ｡◕‿‿◕｡)づ If you run into trouble, feel free to open an issue on [GitHub](https://github.com/Eatgrapes/WinIsland).
