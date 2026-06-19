# winisland-plugin-api

C ABI types and tooling for developing **WinIsland** plugins.

WinIsland is a Dynamic Island emulator for Windows. Plugins are native DLLs that communicate with the host via a C-compatible vtable interface — no serialization, no IPC, straight FFI.

## Usage

```toml
[dependencies]
winisland-plugin-api = "0.1.3"
```

Then export a `plugin_get_instance` function from your `cdylib`:

```rust
use winisland_plugin_api::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn plugin_get_instance() -> PluginInstanceC {
    PluginInstanceC {
        handle: std::ptr::null_mut(),
        metadata: PluginMetadataC {
            id: str_to_fixed("my-plugin"),
            name: str_to_fixed("My Plugin"),
            version: str_to_fixed("1.0.0"),
            author: str_to_fixed("Me"),
            description: str_to_fixed("Does cool stuff"),
        },
        vtable: &VTABLE,
        plugin_type: PluginType::Content as u32,
    }
}
```

## Plugin Types

| Type | Capability |
|------|-----------|
| **Content** | Display text/status on the Dynamic Island |
| **Theme** | Provide custom colours and animation config |
| **Shortcut** | Expose keyboard shortcuts / quick actions |

## Features

| Feature | Description |
|---------|-------------|
| *(default)* | Core C ABI types only — zero extra dependencies |
| `packager` | Build-time packaging, ZIP archiving and Ed25519 signing tools |

## Links

- [GitHub](https://github.com/Eatgrapes/WinIsland)
- [Plugin Development Guide](https://github.com/Eatgrapes/WinIsland/blob/master/Page/plugin-dev.md)
