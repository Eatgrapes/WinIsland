# Changelog

All notable changes to `winisland-plugin-api` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## 0.2.0 - Jun 19, 2026

Added:

- `TranslationPairC` — FFI-safe translation key-value pair for plugin i18n
- `HostApiC::register_translations` — plugin registers translations during `on_load`;
  later registrations override earlier ones for the same key
- i18n overlay layer — `tr()` checks plugin-registered translations before `.lang` files

Changed:

- **Breaking**: `HostApiC` gains a new required field `register_translations`;
  all host implementations must provide this callback
- `lib.rs` split into modular files: `host.rs`, `vtable.rs`, `types/mod.rs`,
  `types/{metadata,content,context,theme,shortcut,i18n}.rs`
- All public types re-exported from crate root — import paths unchanged for plugin authors

## 0.1.3 - Jun 19, 2026

Added:

- `MediaSourceC` — plugin-injectable media source (title, artist, album, duration, position, cover art) #?
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
