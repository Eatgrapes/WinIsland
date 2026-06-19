//! # WinIsland Plugin API
//!
//! C ABI types and tooling for developing [WinIsland](https://github.com/Eatgrapes/WinIsland) plugins.
//!
//! Plugins are native DLLs that communicate with the WinIsland host via a
//! C-compatible vtable interface — no serialization, no IPC, straight FFI.
//!
//! ## Usage modes
//!
//! ### 1. Writing a plugin (core C ABI types, zero extra dependencies)
//!
//! ```toml
//! [dependencies]
//! winisland-plugin-api = "0.1"
//! ```
//!
//! Implement the C ABI by exporting a `plugin_get_instance` function:
//!
//! ```rust,no_run
//! use winisland_plugin_api::*;
//!
//! #[no_mangle]
//! pub unsafe extern "C" fn plugin_get_instance() -> PluginInstanceC {
//!     // See the crate docs for a full plugin example.
//!     unimplemented!()
//! }
//! ```
//!
//! ### 2. Packaging a plugin (requires `packager` feature)
//!
//! ```toml
//! [dev-dependencies]
//! winisland-plugin-api = { version = "0.1", features = ["packager"] }
//! ```
//!
//! Add a `package.rs` binary that builds, signs and zips the plugin:
//!
//! ```rust,no_run
//! winisland_plugin_api::packager::PluginPackager::from_cargo()
//!     .unwrap()
//!     .signing_key_path("signing_key.pem")
//!     .build()
//!     .unwrap();
//! ```
//!
//! Then run `cargo run --bin pack` to produce a signed `.zip` distributable.

pub mod host;
pub mod types;
pub mod vtable;

#[cfg(feature = "packager")]
pub mod packager;

// ---------------------------------------------------------------------------
// Public re-exports — flat import for plugin authors
// ---------------------------------------------------------------------------

pub use host::HostApiC;
pub use types::content::{
    ISLAND_CONTENT_TAG_MUSIC, ISLAND_CONTENT_TAG_NOTIFICATION, ISLAND_CONTENT_TAG_STATUS,
    IslandContentC,
};
pub use types::context::{
    ContextDataC, ContextIdC, HostStateC, MediaSourceC, PRIORITY_HIGH, PRIORITY_LOW,
    PRIORITY_MEDIUM,
};
pub use types::i18n::TranslationPairC;
pub use types::metadata::PluginMetadataC;
pub use types::shortcut::ShortcutC;
pub use types::theme::{AnimationConfigC, ThemeColorsC};
pub use types::{PluginHandle, PluginResultC, PluginType, str_to_fixed};
pub use vtable::{PluginGetInstanceFn, PluginInstanceC, PluginVTable};
