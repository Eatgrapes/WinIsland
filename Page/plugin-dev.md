# Plugin Development Guide

Welcome! You are about to extend WinIsland with your own plugin.

> **Note: The plugin system is currently in a foundation stage.** The C ABI type definitions are ready, and the push-based content system (`send_context`) works. However, the host-side trait interfaces (`ContentProvider`, `ThemeProvider`, `ShortcutProvider`) are not yet wired into the render pipeline. See [issue #55](https://github.com/Eatgrapes/WinIsland/issues/55) for details.

## How Plugins Work

WinIsland uses a **C ABI vtable** pattern to load native `.dll` plugins safely:

```
WinIsland.exe  ──libloading──>  your_plugin.dll
   |                                  |
   |  PluginManager                   |  exports plugin_get_instance()
   |  `-- Vec<NativePlugin>           |  returns PluginInstanceC {
   |       |-- metadata (id, name...) |    handle: opaque ptr
   |       |-- handle (opaque ptr)    |    vtable: function ptrs
   |       `-- vtable (fn ptrs)       |    metadata: PluginMetadataC
   |                                  |  }
   `-- calls traits --> through vtable --> your code runs!
```

All data crossing the FFI boundary is `#[repr(C)]` -- flat structs with no `Vec`, `String`, or trait objects. This means your plugin can be compiled with any Rust version and it will still work.

## Plugin Types

| Type | ID | Purpose | Status |
|------|----|---------|--------|
| **Content** | 1 | Push custom island content (notifications, status, etc.) via `send_context` | Working |
| **Theme** | 2 | Override island colors and animation parameters | API defined, not yet wired |
| **Shortcut** | 3 | Register executable actions | API defined, not yet wired |

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
winisland-plugin-api = "0.2"
```

## Implementing the Plugin

Create a minimal plugin that exports the required C ABI entry point.

**src/lib.rs:**

```rust
use winisland_plugin_api::*;

// The plugin instance is your plugin's state.
struct MyPlugin;

// The one and only entry point -- WinIsland calls this via libloading.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn plugin_get_instance() -> PluginInstanceC {
    let handle = Box::into_raw(Box::new(MyPlugin)) as PluginHandle;

    // The vtable is static -- it lives as long as the DLL is loaded.
    static VTABLE: PluginVTable = PluginVTable {
        on_load:    on_load,
        on_unload:  on_unload,
        destroy:    destroy,
        set_host_api: None,
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

### Pushing Content to the Island

Content-type plugins can export `plugin_set_host_api` to receive the host API table, then call `send_context`. The `PluginVTable::set_host_api` field is reserved for a future version and is not called by the current host.

```rust
struct MyPlugin {
    host_api: Option<*const HostApiC>,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn plugin_set_host_api(
    handle: PluginHandle,
    api: *const HostApiC,
) {
    let plugin = unsafe { &mut *(handle as *mut MyPlugin) };
    plugin.host_api = Some(api);
}

fn send_notification(handle: PluginHandle) -> Option<ContextIdC> {
    let plugin = unsafe { &*(handle as *const MyPlugin) };
    if let Some(api) = plugin.host_api {
        let ctx = ContextDataC {
            priority: PRIORITY_MEDIUM,
            title: str_to_fixed("Notification"),
            body: str_to_fixed("Hello from plugin!"),
            duration_sec: 5,
            mini_render: true,
            mini_text: str_to_fixed("New notification"),
        };
        return Some(unsafe { ((*api).send_context)(handle, ctx) });
    }
    None
}
```

## Packaging with One Command

The `winisland-plugin-api` crate comes with an optional **packager** module that automates release builds, signing, and ZIP packaging.

### 1. Add a packing script

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
winisland-plugin-api = { version = "0.2", features = ["packager"] }

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

The packager will:

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
|-- plugin.yml    (plugin manifest, required)
`-- *.dll         (plugin binary, required, multiple .dll OK)
```

#### plugin.yml

```yaml
name: example
author: xxx
version: 1.0.0
description: This is example plugin
github-link: example/example-plugin
```

**All 5 fields are required** -- missing any will cause install to fail.

## Digital Signing (Recommended)

The packager can optionally add an Ed25519 signature and DLL hashes to `plugin.yml`. The current host does not verify these fields yet, so signing should be treated as package metadata rather than an enforced trust boundary.

### Generate a signing key

```bash
openssl genpkey -algorithm ed25519 -out signing_key.pem
openssl pkey -in signing_key.pem -pubout -out public_key.pem
```

Keep private signing keys outside the repository and provide them to CI through a protected secret.

### Sign during packaging

```bash
cargo run --bin pack
```

Calling `signing_key_path` or `signing_key_env` makes the packager add the signature to `plugin.yml`:

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

## Installing

Simply **drag the `.zip` file onto the island**. The plugin is extracted in a background thread (so your island stays smooth and responsive) and loaded automatically.

A Windows notification dialog will confirm successful installation.

You can also manually place `.dll` files into subdirectories under the plugins folder -- WinIsland scans them on startup.

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
| **Package hashes** | The packager can record DLL hashes; host verification is not implemented yet |
| **Path traversal protection** | ZIP entries with `..`, `:`, or absolute paths are rejected |
| **Symlink rejection** | ZIP symlink entries are rejected |
| **Background extraction** | ZIP decompression runs in a background thread |
| **Poison handling** | Lock poisoning does not crash the host |
| **VTable validation** | Required function pointers checked for null before calling |

## How to Verify Your Plugin Loaded?

`send_context` content and plugin media sources are connected to the Island UI. Theme, shortcut, click, and expanded-state provider callbacks are not connected yet.

**Verification:**
1. Press `F12` to open the WinIsland debug log window
2. Search for your plugin name -- you should see something like `Loaded plugin: xxx (xxx)`
3. Dropping a ZIP triggers a Windows popup confirming success/failure

## C ABI Type Reference

These types live in the `winisland-plugin-api` crate.

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

### HostApiC

The host API table is passed to plugins through the exported `plugin_set_host_api` function. Plugins store this pointer and call through it to interact with the host.

```rust
pub struct HostApiC {
    pub send_context: unsafe extern "C" fn(PluginHandle, ContextDataC) -> ContextIdC,
    pub close_context: unsafe extern "C" fn(PluginHandle, *const c_char) -> PluginResultC,
    pub query_host_state: unsafe extern "C" fn(PluginHandle) -> HostStateC,
    pub set_media_source: unsafe extern "C" fn(PluginHandle, MediaSourceC) -> PluginResultC,
    pub clear_media_source: unsafe extern "C" fn(PluginHandle) -> PluginResultC,
    pub register_translations: unsafe extern "C" fn(
        PluginHandle,
        *const c_char,
        *const TranslationPairC,
        u32,
    ) -> PluginResultC,
}
```

### ContextDataC / ContextIdC / HostStateC / MediaSourceC

```rust
pub struct ContextDataC {
    pub priority: u32,       // PRIORITY_LOW, PRIORITY_MEDIUM, or PRIORITY_HIGH
    pub title: [u8; 256],    // shown in mini and expanded views
    pub body: [u8; 512],     // expanded body text
    pub duration_sec: u32,   // seconds before auto-collapse
    pub mini_render: bool,   // show mini summary after collapsing
    pub mini_text: [u8; 128],// mini summary text
}

pub struct ContextIdC {
    pub id: [u8; 128],       // encoded as "plugin_id:context_id"
}

pub struct HostStateC {
    pub media_title: [u8; 256],
    pub media_artist: [u8; 256],
    pub is_playing: bool,
    pub theme: [u8; 32],     // "light" or "dark"
}

pub struct MediaSourceC {
    pub title: [u8; 256],
    pub artist: [u8; 256],
    pub album: [u8; 256],
    pub duration_ms: u64,
    pub position_ms: u64,
    pub is_playing: bool,
    pub cover_data: *const u8,
    pub cover_len: u32,
}
```

### PluginVTable

```rust
pub struct PluginVTable {
    pub on_load: unsafe extern "C" fn(PluginHandle) -> PluginResultC,
    pub on_unload: unsafe extern "C" fn(PluginHandle) -> PluginResultC,
    pub destroy: unsafe extern "C" fn(PluginHandle),
    pub set_host_api: Option<unsafe extern "C" fn(PluginHandle, *const HostApiC)>, // reserved
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

### TranslationPairC

```rust
pub struct TranslationPairC {
    pub key: *const c_char,
    pub value: *const c_char,
}
```

## Join the Discussion

Beyond hooking into the Island context, we do not have many concrete directions yet. Please join us at [#55](https://github.com/Eatgrapes/WinIsland/issues/55) to discuss what you would like the plugin system to support.

---

If you run into trouble, feel free to open an issue on [GitHub](https://github.com/Eatgrapes/WinIsland).
