# winisland-plugin-api

Versioned C ABI types and packaging tools for trusted native WinIsland plugins.

Plugins are loaded as in-process Windows DLLs. They are not sandboxed: install only plugins you trust. ABI v1 is published by crate version `0.3` and does not support the old `0.2` vtable ABI.

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

## Minimal context plugin

```rust
use std::ffi::c_void;
use winisland_plugin_api::*;

struct Instance {
    token: PluginToken,
    context_api: ContextApiV1,
    context_id: ResourceId,
}

static DESCRIPTOR: PluginDescriptorV1 = PluginDescriptorV1 {
    struct_size: std::mem::size_of::<PluginDescriptorV1>() as u32,
    abi_version: ABI_VERSION_1,
    capabilities: CAPABILITY_CONTEXT,
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

unsafe extern "C" fn create(
    create_info: *const PluginCreateInfoV1,
    out_handle: *mut PluginHandle,
) -> PluginResultC {
    if create_info.is_null() || out_handle.is_null() {
        return PluginResultC::err("null create argument");
    }
    // SAFETY: WinIsland supplies a complete ABI v1 create-info structure.
    let info = unsafe { &*create_info };
    if info.struct_size < std::mem::size_of::<PluginCreateInfoV1>() as u32
        || info.abi_version != ABI_VERSION_1
        || info.host_api.is_null()
    {
        return PluginResultC::err("unsupported create info");
    }
    // SAFETY: The host API pointer remains valid for the process lifetime.
    let host = unsafe { &*info.host_api };
    // SAFETY: `host` was supplied by WinIsland and validated above.
    let Some(context_api) = (unsafe { host.context_api() }) else {
        return PluginResultC::err("context API is unavailable");
    };
    let Some(create_context) = context_api.create else {
        return PluginResultC::err("context create is unavailable");
    };

    let context = ContextDataV1 {
        title: str_to_fixed("Hello WinIsland"),
        body: str_to_fixed("ABI v1 plugin is running"),
        compact_text: str_to_fixed("Hello"),
        ..Default::default()
    };
    let mut context_id = INVALID_ID;
    // SAFETY: The input and output pointers remain valid for this call.
    let result = unsafe { create_context(info.plugin_token, &context, &mut context_id) };
    if result.status != 0 {
        return result;
    }

    let instance = Box::new(Instance {
        token: info.plugin_token,
        context_api,
        context_id,
    });
    // SAFETY: WinIsland owns this opaque handle until `destroy`.
    unsafe { out_handle.write(Box::into_raw(instance).cast::<c_void>()) };
    PluginResultC::ok()
}

unsafe extern "C" fn shutdown(handle: PluginHandle) -> PluginResultC {
    if handle.is_null() {
        return PluginResultC::ok();
    }
    // SAFETY: `handle` was created from `Box<Instance>` in `create`.
    let instance = unsafe { &mut *handle.cast::<Instance>() };
    if instance.context_id != INVALID_ID {
        if let Some(release) = instance.context_api.release {
            // SAFETY: This resource belongs to the same plugin token.
            let result = unsafe { release(instance.token, instance.context_id) };
            if result.status != 0 {
                return result;
            }
        }
        instance.context_id = INVALID_ID;
    }
    PluginResultC::ok()
}

unsafe extern "C" fn destroy(handle: PluginHandle) {
    if !handle.is_null() {
        // SAFETY: `destroy` is called once after successful shutdown.
        unsafe { drop(Box::from_raw(handle.cast::<Instance>())) };
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// WinIsland calls this function using the documented ABI v1 signature.
pub unsafe extern "C" fn winisland_plugin_entry_v1() -> *const PluginDescriptorV1 {
    &DESCRIPTOR
}
```

Lifecycle is always `create -> shutdown -> destroy`. `shutdown` must stop and join every thread that can execute plugin code or call a host service. WinIsland destroys the handle and unloads the DLL only after `shutdown` succeeds.

## Host services

Declare each service in `PluginDescriptorV1.capabilities`, then query it from `HostApiV1` during `create`:

| Capability | Query method | Resource operations |
|---|---|---|
| `CAPABILITY_CONTEXT` | `context_api()` | create, update, release |
| `CAPABILITY_MEDIA` | `media_api()` | create, update, release, UI command callback |
| `CAPABILITY_I18N` | `i18n_api()` | register and release translation bundles |
| `CAPABILITY_HOST_STATE` | `host_state_api()` | read the current media/theme snapshot |

All created resources belong to the host-issued `PluginToken`. A plugin cannot update or release another plugin's resources. WinIsland automatically revokes remaining resources after successful shutdown.

Borrowed slices (`ByteSliceV1`, `Utf8SliceV1`) only need to remain valid until the host call returns. `MediaSourceDataV1.callback_data` is different: it must remain valid until the media resource is successfully released. Media callbacks run synchronously on the WinIsland event-loop thread and may call host services. Update or release of that media resource returns an error while its callback is executing.

Every pointer passed across the ABI must be non-null when required, correctly aligned for its declared type, and readable or writable for the complete call. All public ABI structs use `#[repr(C)]` and start with `struct_size` where versioned extension is supported.

## Packaging

Enable the optional packager:

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

`from_cargo()` reads package metadata, `repository`, and `[lib].name`. The package metadata must exactly match `PluginMetadataC`; use builder methods such as `.name(...)` or `.id(...)` when overriding it.

The generated `plugin.yml` identifies one entry DLL:

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

Additional DLLs and asset directories may be included as dependencies, but WinIsland only loads `entry` as the plugin. The packager can write hashes and an Ed25519 signature; host-side signature verification is not implemented yet.

## Features

| Feature | Description |
|---|---|
| default | Core ABI v1 types with no extra dependencies |
| `packager` | Build, ZIP, hash, and optional Ed25519 signing tools |

See the [WinIsland plugin development guide](https://tanikaze.icu/WinIsland/) for all services and installation details.
