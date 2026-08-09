# Plugin API Changelog

This page lists published `winisland-plugin-api` releases only. Release notes are added when a version is published; there is no `Unreleased` section.

## 0.3.0 - Aug 9, 2026

Added:

- Native DLL ABI v1 through `winisland_plugin_entry_v1() -> *const PluginDescriptorV1`
- Host-issued `PluginToken` and `ResourceId` values for identity and ownership
- Capability declarations for Context, Media, i18n, and Host State
- Versioned service discovery through `HostApiV1::query_interface`
- Context create, update, release, priority, compact text, and timeout support
- Media sources with cover art, playback position, available controls, and optional callbacks
- Releasable translation bundles and current media/theme Host State snapshots
- Single-entry plugin packages with `id`, `abi-version`, and `entry` manifest fields

Changed:

- **Breaking**: removed the 0.2 `PluginVTable`, `PluginType`, `PluginInstanceC`,
  `HostApiC`, `plugin_get_instance`, and `plugin_set_host_api` interfaces
- **Breaking**: removed the unfinished Theme and Shortcut interfaces
- Lifecycle is strictly `create -> shutdown -> destroy`; DLL unload requires successful shutdown
- Context identifiers are host-issued numeric resources instead of plugin-defined strings
- Plugin Media is independent of the SMTC setting and only renders declared controls
- Worker-thread resource changes wake the WinIsland event loop
- Packager metadata now reads Cargo `repository` and `[lib].name`

Fixed:

- Descriptor size, ABI version, capability, metadata, and lifecycle callback validation
- Token-bound resource ownership, per-plugin count limits, and memory limits
- Media callback reentrancy and unload synchronization
- UTF-8-safe fixed-buffer truncation and bounded borrowed-slice copying
- Context update/release event coalescing and media seek source binding
- Translation bundle cleanup and host event-loop wake coalescing
- Bounded ZIP extraction, Windows path-collision checks, staging activation, backup, and rollback

## 0.2.0 - Jun 19, 2026

Added:

- `TranslationPairC` — FFI-safe translation key-value pair for plugin i18n
- `HostApiC::register_translations` — plugin registers translations during `on_load`;
  later registrations override earlier ones for the same key
- i18n overlay layer — `tr()` checks plugin-registered translations before `.lang` files

Changed:

- **Breaking**: `HostApiC` gains a new required field, `register_translations`;
  all host implementations must provide this callback
- The crate is split into focused host, vtable, and type modules
- All public types remain re-exported from the crate root

## 0.1.3 - Jun 19, 2026

Added:

- `MediaSourceC` — plugin-injectable media source (title, artist, album, duration, position, cover art)
- `HostApiC::set_media_source` — replace SMTC with plugin-provided media data
- `HostApiC::clear_media_source` — restore SMTC as the active media source

Changed:

- `HostApiC` derives `Clone`, `Copy` for safe FFI usage
- `PluginResultC` derives `Debug`, `Clone`, `Copy`
- `ContextDataC`, `ContextIdC`, `HostStateC` — new push-based context types
- `PluginVTable::set_host_api` — optional slot for plugin to receive `HostApiC` pointer

## 0.1.2 - Jun 17, 2026

Added:

- README.md with crate-level documentation, usage examples and feature flags

## 0.1.1 - Jun 16, 2026

Added:

- `packager` feature: `PluginPackager` for building, signing and zipping plugins
- Cargo.toml metadata for crates.io publishing (repository, homepage, license, keywords, categories)
- `docs.rs` configuration with `packager` feature enabled

Changed:

- Use `str_to_fixed` helper for byte-buffer initialization, replacing manual padding loops
- Packager validates `manifest.yaml` during `build()`; checks for missing fields and oversized buffers
- `github_link` field in `Manifest` is now required (non-empty) to satisfy host validation

Fixed:

- `plugin_get_instance` doc example uses proper `#[no_mangle]` export, no extraneous `fn main`
- Broken doc links in packager module docs
- `BG_CACHE` size check in signing flow

## 0.1.0 - Jun 15, 2026

Added:

- Initial release — C ABI types extracted from the WinIsland host into a standalone crate
- Core types: `PluginInstanceC`, `PluginVTable`, `PluginMetadataC`, `IslandContentC`, `ThemeColorsC`, `AnimationConfigC`, `ShortcutC`, `PluginResultC`
- `PluginType` enum with `from_u32` conversion
- `PluginGetInstanceFn` — entry-point signature for plugin DLLs
- `str_to_fixed` / `read_c_str` / `read_opt_c_str` helpers for FFI byte-buffer handling
- Priority constants: `PRIORITY_LOW`, `PRIORITY_MEDIUM`, `PRIORITY_HIGH`
- Content tag constants: `ISLAND_CONTENT_TAG_MUSIC`, `ISLAND_CONTENT_TAG_NOTIFICATION`, `ISLAND_CONTENT_TAG_STATUS`
