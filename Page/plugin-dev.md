# Plugin Development Guide

WinIsland plugin API `0.3` defines native ABI v1. Plugins are trusted Windows DLLs loaded into the WinIsland process. There is no sandbox or crash isolation, so users must only install plugins they trust.

The old `0.2` `PluginVTable`, `PluginType`, `plugin_get_instance`, and `plugin_set_host_api` interfaces are not supported.

## Architecture

```text
plugin DLL exports winisland_plugin_entry_v1()
    -> PluginDescriptorV1
    -> WinIsland validates ABI, capabilities, and metadata
    -> WinIsland issues PluginToken and calls create(PluginCreateInfoV1)
    -> plugin queries versioned Context/Media/I18n/HostState services
    -> plugin creates host-owned resources identified by ResourceId
    -> WinIsland calls shutdown(handle)
    -> WinIsland revokes remaining resources
    -> WinIsland calls destroy(handle) and unloads the DLL
```

Lifecycle is strictly `create -> shutdown -> destroy`. `shutdown` must synchronously stop and join every plugin thread that can execute plugin code or call a host service. WinIsland only destroys the handle and unloads the DLL after `shutdown` returns success.

## Project setup

```toml
[package]
name = "hello-winisland-plugin"
version = "0.1.0"
edition = "2024"
authors = ["Example Author"]
description = "Minimal WinIsland ABI v1 plugin"
repository = "https://github.com/example/hello-winisland-plugin"

[lib]
name = "hello_winisland_plugin"
crate-type = ["cdylib"]

[dependencies]
winisland-plugin-api = "0.3"
```

The crate README contains a complete Context plugin that can be used as the initial `src/lib.rs` implementation.

## Entry descriptor

Every plugin exports one entry point:

```rust
#[unsafe(no_mangle)]
/// # Safety
/// WinIsland calls this function using the documented ABI v1 signature.
pub unsafe extern "C" fn winisland_plugin_entry_v1() -> *const PluginDescriptorV1 {
    &DESCRIPTOR
}
```

The descriptor is static:

```rust
static DESCRIPTOR: PluginDescriptorV1 = PluginDescriptorV1 {
    struct_size: std::mem::size_of::<PluginDescriptorV1>() as u32,
    abi_version: ABI_VERSION_1,
    capabilities: CAPABILITY_CONTEXT | CAPABILITY_HOST_STATE,
    metadata: PluginMetadataC::new(
        "hello-winisland-plugin",
        "hello-winisland-plugin",
        "0.1.0",
        "Example Author",
        "Minimal WinIsland ABI v1 plugin",
    ),
    create: Some(create),
    shutdown: Some(shutdown),
    destroy: Some(destroy),
};
```

Plugin ID must match `[a-zA-Z0-9_-]+`. Package metadata and DLL descriptor metadata must match exactly.

## Querying services

During `create`, validate `PluginCreateInfoV1`, retain its `plugin_token`, and query declared services:

```rust
let info = unsafe { &*create_info };
let host = unsafe { &*info.host_api };
let context_api = unsafe { host.context_api() }
    .ok_or_else(|| PluginResultC::err("context API unavailable"));
```

The available services are:

| Capability | Query | Purpose |
|---|---|---|
| `CAPABILITY_CONTEXT` | `HostApiV1::context_api()` | Compact island text/status resources |
| `CAPABILITY_MEDIA` | `HostApiV1::media_api()` | Media UI data and optional controls |
| `CAPABILITY_I18N` | `HostApiV1::i18n_api()` | Translation bundles |
| `CAPABILITY_HOST_STATE` | `HostApiV1::host_state_api()` | Current media and light/dark theme snapshot |

Calling a service without declaring its capability returns an error.

## Context service

Context resources support create, update, and release:

```rust
let data = ContextDataV1 {
    priority: PRIORITY_MEDIUM,
    flags: CONTEXT_FLAG_SHOW_COMPACT,
    timeout_ms: 5_000,
    title: str_to_fixed("Build complete"),
    body: str_to_fixed("Release package is ready"),
    compact_text: str_to_fixed("Build ready"),
    ..Default::default()
};

let mut id = INVALID_ID;
let result = unsafe { context_api.create.unwrap()(token, &data, &mut id) };
```

`timeout_ms = 0` means persistent. A timeout hides the context; the plugin still owns the resource and should release it. Updating a resource refreshes its display order and timeout.

## Media service

Media resources replace SMTC while active. The most recently created or updated media resource is displayed. Releasing the active resource selects the next plugin resource or restores SMTC.

```rust
let cover = std::fs::read("cover.png").unwrap_or_default();
let media = MediaSourceDataV1 {
    flags: MEDIA_FLAG_PLAYING,
    title: str_to_fixed("Plugin Track"),
    artist: str_to_fixed("Plugin Artist"),
    duration_ms: 180_000,
    position_ms: 12_000,
    cover: ByteSliceV1::from_slice(&cover),
    ..Default::default()
};
```

The host copies cover bytes before returning. Cover data only needs to remain valid during the call.

To enable media controls, declare the corresponding `MEDIA_CONTROL_*` flags and provide `on_command`. The callback runs synchronously on the WinIsland event-loop thread:

```rust
media.available_controls = MEDIA_CONTROL_TOGGLE_PLAY | MEDIA_CONTROL_SEEK;
media.on_command = Some(on_media_command);
media.callback_data = state_pointer;
```

`callback_data` must remain valid until the media resource is successfully released. The callback may call host services. Update or release of the same media resource returns an error while its callback is executing.

## Translation service

Translation strings are borrowed UTF-8 slices and are copied during registration:

```rust
let pairs = [TranslationPairV1 {
    key: Utf8SliceV1::borrowed("hello.title"),
    value: Utf8SliceV1::borrowed("Hello"),
}];
let mut bundle_id = INVALID_ID;
let result = unsafe {
    i18n_api.register_bundle.unwrap()(
        token,
        Utf8SliceV1::borrowed("en_us"),
        pairs.as_ptr(),
        pairs.len() as u32,
        &mut bundle_id,
    )
};
```

Supported built-in language codes currently include `en_us`, `zh_cn`, and `es_es`. Retain and release every bundle `ResourceId` during shutdown.

## Host state

Use `HostStateV1::default()` before calling `get`; the host validates `struct_size`:

```rust
let mut state = HostStateV1::default();
let result = unsafe { host_state_api.get.unwrap()(token, &mut state) };
if result.status == 0 {
    let playing = state.is_playing != 0;
}
```

The snapshot reports the media currently displayed by WinIsland, including plugin media, plus `light` or `dark` theme text.

## Resource ownership

- `PluginToken` is issued by WinIsland. Never invent or share one.
- `ResourceId` is issued by a service create/register call.
- Update and release require the token that owns the resource.
- WinIsland revokes remaining resources after successful shutdown.
- Plugins should still release resources explicitly during shutdown.
- Context, Media, and translation resources have per-plugin count and memory limits.
- Host services may be called from plugin worker threads; resource changes wake the WinIsland event loop.

## FFI rules

- All ABI structs are `#[repr(C)]`.
- Versioned structs begin with `struct_size`; initialize them with `Default` where available.
- Required pointers must be non-null, correctly aligned, and readable/writable for the complete call.
- `ByteSliceV1` and `Utf8SliceV1` are `(ptr, len)` borrowed slices, not NUL-terminated strings.
- The host copies borrowed slices before returning unless a field explicitly documents a longer lifetime.
- Do not let panic unwind across an `extern "C"` boundary.
- Native plugins are trusted and execute with the WinIsland process permissions.

## Packaging

Add the packager as a dev dependency:

```toml
[dev-dependencies]
winisland-plugin-api = { version = "0.3", features = ["packager"] }

[[example]]
name = "pack"
path = "package.rs"
```

```rust
fn main() {
    winisland_plugin_api::packager::PluginPackager::from_cargo()
        .unwrap()
        .build()
        .unwrap();
}
```

Run `cargo run --example pack`.

`from_cargo()` reads package name, version, author, description, repository, and `[lib].name`. Builder methods can override metadata, but generated `plugin.yml` metadata must still match `PluginMetadataC` exactly.

The package has one entry DLL:

```yaml
id: hello-winisland-plugin
name: hello-winisland-plugin
author: Example Author
version: 0.1.0
description: Minimal WinIsland ABI v1 plugin
github-link: https://github.com/example/hello-winisland-plugin
abi-version: 1
entry: hello_winisland_plugin.dll
```

Drag the ZIP onto WinIsland to install it. Extraction uses size/path limits and a staging directory. Updates validate the new descriptor before stopping the old plugin, then use backup-and-rollback directory replacement. Additional DLLs can be packaged as dependencies, but only `entry` is loaded as the plugin.

The packager can add DLL hashes and an Ed25519 signature. WinIsland does not enforce signature verification yet.
