# Plugin Development Guide

Welcome! (´｡• ᵕ •｡`)♡ You're about to extend WinIsland with your own plugin. This guide will walk you through everything you need to know.

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

## Plugin Types

You can write three kinds of plugins:

| Type | What it does | VTable field |
|------|-------------|-------------|
| **Content** (id=1) | Provide custom island content (weather, notifications, status…) | `get_content`, `on_click`, `on_expanded`, `supports_expand` |
| **Theme** (id=2) | Override island colors and animation parameters | `get_colors`, `get_animations` |
| **Shortcut** (id=3) | Register executable actions | _(not yet exposed in vtable)_ |

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
winisland-plugin-api = { git = "https://github.com/oneachina/WinIsland", branch = "feat/plugin-system" }
```

## Packaging as ZIP (ﾉ◕ヮ◕)ﾉ

Your plugin must be packaged as `.zip` to be loaded by WinIsland. The ZIP must contain:

```
my-plugin.zip
├── plugin.yml    ← plugin manifest (required)
└── *.dll         ← plugin binary (required, multiple .dll OK)
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

Simply **drag the `.zip` file onto the island**! While hovering it shows "📦 放入 zip~ 以加载插件", release to auto-extract and load, then see "✅ 已加载 {name}~"

Plugins are extracted to `C:\Users\<YourName>\AppData\Roaming\WinIsland\plugins\<plugin-name>\`.

## Writing a ContentProvider Plugin

Here's a minimal "Hello World" plugin that shows a status message (｡･ω･｡):

```rust
use std::ffi::c_void;
use winisland_plugin_api::*;

struct HelloPlugin {
    clicks: u32,
}

// ── VTable entries ──

extern "C" fn hello_on_load(_handle: PluginHandle) -> PluginResultC {
    PluginResultC::ok()
}

extern "C" fn hello_on_unload(_handle: PluginHandle) -> PluginResultC {
    PluginResultC::ok()
}

extern "C" fn hello_destroy(handle: PluginHandle) {
    unsafe { drop(Box::from_raw(handle as *mut HelloPlugin)); }
}

extern "C" fn hello_get_content(handle: PluginHandle) -> IslandContentC {
    let plugin = unsafe { &mut *(handle as *mut HelloPlugin) };
    let mut label = [0u8; 128];
    let msg = format!("Clicked {} times", plugin.clicks);
    let bytes = msg.as_bytes();
    let len = bytes.len().min(127);
    label[..len].copy_from_slice(&bytes[..len]);

    IslandContentC {
        tag: ISLAND_CONTENT_TAG_STATUS,
        label,
        ..zero_content()
    }
}

extern "C" fn hello_on_click(handle: PluginHandle) {
    let plugin = unsafe { &mut *(handle as *mut HelloPlugin) };
    plugin.clicks += 1;
}

extern "C" fn hello_supports_expand(_handle: PluginHandle) -> bool {
    false
}

// ── VTable ──

static VTABLE: PluginVTable = PluginVTable {
    on_load: hello_on_load,
    on_unload: hello_on_unload,
    destroy: hello_destroy,
    get_content: Some(hello_get_content),
    on_click: Some(hello_on_click),
    on_expanded: None,
    supports_expand: Some(hello_supports_expand),
    get_colors: None,
    get_animations: None,
};

// ── Metadata ──

fn fill_metadata() -> PluginMetadataC {
    let mut id = [0u8; 64];
    let mut name = [0u8; 128];
    let mut version = [0u8; 32];
    let mut author = [0u8; 128];
    let mut description = [0u8; 256];

    write_str(&mut id, "hello_plugin");
    write_str(&mut name, "Hello Plugin");
    write_str(&mut version, "0.1.0");
    write_str(&mut author, "You! o(TヘTo)");
    write_str(&mut description, "A friendly example plugin");

    PluginMetadataC { id, name, version, author, description }
}

fn write_str(buf: &mut [u8], s: &str) {
    let bytes = s.as_bytes();
    let len = bytes.len().min(buf.len() - 1);
    buf[..len].copy_from_slice(&bytes[..len]);
}

fn zero_content() -> IslandContentC {
    IslandContentC {
        tag: 0,
        title: [0u8; 256],
        artist: [0u8; 256],
        cover_url: [0u8; 512],
        is_playing: false,
        message: [0u8; 256],
        label: [0u8; 128],
        value: [0u8; 128],
    }
}

// ── Entry point ──

#[unsafe(no_mangle)]
pub extern "C" fn plugin_get_instance() -> PluginInstanceC {
    let plugin = Box::new(HelloPlugin { clicks: 0 });
    PluginInstanceC {
        handle: Box::into_raw(plugin) as PluginHandle,
        metadata: fill_metadata(),
        vtable: &VTABLE,
        plugin_type: 1, // Content
    }
}
```

### Key Points

1. **Only export `plugin_get_instance`** — this is the only symbol WinIsland looks for.
2. **`handle` is opaque** — you own its type; the host never touches it directly.
3. **Fill all C struct fields** — uninitialized bytes are UB. Use `zero_content()` or `[0u8; N]`.
4. **`destroy` must free the handle** — `Box::from_raw(handle)` gives ownership back so `drop` runs.
5. **Null vtable entries are fine** — fill `None` for features your plugin type doesn't support.

## IslandContentC Tags

When returning `IslandContentC`, set `tag` to one of:

| Tag constant | Value | Meaning |
|---|---|---|
| `ISLAND_CONTENT_TAG_MUSIC` | 1 | Fill `title`, `artist`, `cover_url`, `is_playing` |
| `ISLAND_CONTENT_TAG_NOTIFICATION` | 2 | Fill `title`, `message`, `cover_url` (as icon) |
| `ISLAND_CONTENT_TAG_STATUS` | 3 | Fill `label`, `value`, `cover_url` (as icon) |

## C ABI Type Reference

These types live in the `winisland-plugin-api` crate. All are `#[repr(C)]`.

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

All strings are null-terminated UTF-8.

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

Which fields are used depends on `tag`. Unused fields should be zeroed.

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

## Troubleshooting (╥﹏╥)

| Problem | Check |
|---|---|
| Plugin not loaded | Is the `.dll` in the right directory? Check WinIsland logs for "Failed to load plugin" |
| Null handle / null vtable | `plugin_get_instance()` must fill all fields |
| Crash on startup | Make sure `zero_content()` or `= [0u8; N]` is used for all struct fields |
| `get_content` returns wrong data | Double-check `tag` value and which fields you filled |

---

Happy hacking! (づ｡◕‿‿◕｡)づ If you run into trouble, feel free to open an issue on [GitHub](https://github.com/Eatgrapes/WinIsland).
