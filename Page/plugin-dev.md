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
winisland-plugin-api = "0.1"
```

## Implementing the Plugin

Create a minimal plugin that exports the required C ABI entry point.

**src/lib.rs:**

```rust
use winisland_plugin_api::*;

// The plugin instance is your plugin's state.
struct MyPlugin;

// The one and only entry point — WinIsland calls this via libloading.
#[no_mangle]
pub extern "C" fn plugin_get_instance() -> PluginInstanceC {
    let handle = Box::into_raw(Box::new(MyPlugin)) as PluginHandle;

    // The vtable is static — it lives as long as the DLL is loaded.
    static VTABLE: PluginVTable = PluginVTable {
        on_load:    on_load,
        on_unload:  on_unload,
        destroy:    destroy,
        get_content: None,
        on_click:   None,
        on_expanded: None,
        supports_expand: None,
        get_colors: None,
        get_animations: None,
        get_shortcuts_count: None,
        get_shortcut_at: None,
        execute_shortcut: None,
    };

    PluginInstanceC {
        handle,
        metadata: PluginMetadataC {
            id:          str_to_fixed("my-plugin"),
            name:        str_to_fixed("My Plugin"),
            version:     str_to_fixed("0.1.0"),
            author:      str_to_fixed("you"),
            description: str_to_fixed("A minimal WinIsland plugin"),
        },
        vtable: &VTABLE,
        plugin_type: PluginType::Content as u32,
    }
}

unsafe extern "C" fn on_load(_handle: PluginHandle) -> PluginResultC {
    PluginResultC::ok()
}

unsafe extern "C" fn on_unload(_handle: PluginHandle) -> PluginResultC {
    PluginResultC::ok()
}

unsafe extern "C" fn destroy(handle: PluginHandle) {
    drop(unsafe { Box::from_raw(handle as *mut MyPlugin) });
}
```

## Packaging with One Command (ﾉ◕ヮ◕)ﾉ

The `winisland-plugin-api` crate comes with an optional **packager** module that automates release builds, signing, and ZIP packaging.

### 1. Add a packing script

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
winisland-plugin-api = { version = "0.1", features = ["packager"] }

[[bin]]
name = "pack"
path = "package.rs"
```

Create `package.rs` at the project root:

```rust
fn main() {
    winisland_plugin_api::packager::PluginPackager::from_cargo()
        .unwrap()
        .signing_key_path("signing_key.pem")  // optional
        .include_dir("assets")                 // optional
        .build()
        .unwrap();
}
```

### 2. Build everything

```bash
# This single command compiles, signs (if key provided), and packages into a ZIP:
cargo run --bin pack
# Output: target/my-winisland-plugin-0.1.0.zip
```

That's it! The packager will:

1. Run `cargo build --release` to compile your DLL
2. Find the built `.dll` in `target/release/`
3. Copy any extra directories (like `assets/`)
4. Compute SHA-256 hashes of all DLLs
5. Sign the manifest with your Ed25519 key (if provided)
6. Generate `plugin.yml` with all metadata
7. Pack everything into `<name>-<version>.zip`

### Without the packager (manual ZIP)

Your plugin must be packaged as `.zip` to be loaded by WinIsland. The ZIP must contain:

```
my-plugin.zip
├── plugin.yml    ← plugin manifest (required)
└── *.dll         ← plugin binary (required, multiple .dll OK)
```

#### plugin.yml

```yaml
name: example
author: xxx
version: 1.0.0
description: This is example plugin
github-link: example/example-plugin
```

**All 5 fields are required** — missing any will cause install to fail o(TヘTo)

## Digital Signing (Recommended)

Plugins can be **optionally** signed with Ed25519 to verify authenticity. Signed plugins are verified before loading — unsigned ones still work but show a warning.

### Generate a signing key

```bash
openssl genpkey -algorithm ed25519 -out signing_key.pem
openssl pkey -in signing_key.pem -pubout -out public_key.pem
```

The `public_key.pem` is embedded into the WinIsland binary by the project maintainers. If you're a plugin developer, you'll receive the signing key from the WinIsland team through a secure channel.

### Sign during packaging

```bash
cargo run --bin pack
```

If `signing_key.pem` is present in the project root, the packager automatically signs the plugin. The signature is embedded in `plugin.yml`:

```yaml
name: my-plugin
author: you
version: 1.0.0
description: My awesome plugin
github-link: you/my-plugin
signature: "abc123deadbeef..."    # Ed25519 signature (64 bytes hex)
dll_hashes:
  - "sha256hashofdll..."
```

### CI signing with environment variable

```yaml
# .github/workflows/release.yml
- run: cargo run --bin pack
  env:
    PLUGIN_SIGNING_KEY: ${{ secrets.PLUGIN_SIGNING_KEY }}
```

```rust
// package.rs
PluginPackager::from_cargo()
    .unwrap()
    .signing_key_env("PLUGIN_SIGNING_KEY")
    .build()
    .unwrap();
```

## Installing ฅ^•ﻌ•^ฅ

Simply **drag the `.zip` file onto the island**! The plugin is extracted in a background thread (so your island stays smooth and responsive) and loaded automatically.

A Windows notification dialog will confirm successful installation.

You can also manually place `.dll` files into subdirectories under the plugins folder — WinIsland scans them on startup.

### Plugin storage location

```
C:\Users\<YourName>\AppData\Roaming\WinIsland\plugins\<plugin-name>\*.dll
```

## Security

WinIsland applies several security measures when loading plugins:

| Protection | Details |
|-----------|---------|
| **Plugin ID validation** | IDs must match `[a-zA-Z0-9_-]+` only |
| **ID conflict detection** | Duplicate plugin IDs are rejected |
| **Signature verification** | Ed25519 signature checked before loading (if present) |
| **Path traversal protection** | ZIP entries with `..`, `:`, or absolute paths are rejected |
| **Symlink rejection** | ZIP symlink entries are rejected |
| **Background extraction** | ZIP decompression runs in a background thread |
| **Poison handling** | Lock poisoning doesn't crash the host |
| **VTable validation** | Required function pointers checked for null before calling |

## How to Verify Your Plugin Loaded?

Since the host-side API is still under development, plugins won't display anything on the Island UI yet.

**Verification:**
1. Press `F12` to open the WinIsland debug log window
2. Search for your plugin name — you should see something like `Loaded plugin: xxx (xxx)`
3. Dropping a ZIP triggers a Windows popup confirming success/failure

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
    pub get_shortcuts_count: Option<unsafe extern "C" fn(PluginHandle) -> u32>,
    pub get_shortcut_at: Option<unsafe extern "C" fn(PluginHandle, i: u32, out: *mut ShortcutC)>,
    pub execute_shortcut: Option<unsafe extern "C" fn(PluginHandle, id: *const c_char) -> PluginResultC>,
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

### ThemeColorsC / AnimationConfigC / ShortcutC

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

pub struct ShortcutC {
    pub id: [u8; 64],
    pub name: [u8; 128],
    pub description: [u8; 256],
    pub icon: [u8; 256],
    pub hotkey: [u8; 32],
}
```

## Join the Discussion (づ｡◕‿‿◕｡)づ

Beyond hooking into the Island context, we don't have many concrete directions yet… **we're really short on inspiration QWQ**

Please join us at [#55](https://github.com/Eatgrapes/WinIsland/issues/55) to discuss what you'd like the plugin system to support!

---

Happy hacking! (づ｡◕‿‿◕｡)づ If you run into trouble, feel free to open an issue on [GitHub](https://github.com/Eatgrapes/WinIsland).
