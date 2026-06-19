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

use std::ffi::c_char;

/// Opaque handle to a plugin instance, passed through every vtable call.
pub type PluginHandle = *mut std::ffi::c_void;

#[cfg(feature = "packager")]
pub mod packager;

/// Identifies what capability a plugin provides to the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginType {
    /// Plugin provides island content (music, notification, status, etc.)
    Content = 1,
    /// Plugin provides theme colours and animation configuration.
    Theme = 2,
    /// Plugin provides keyboard shortcuts / quick actions.
    Shortcut = 3,
}

impl PluginType {
    /// Convert a raw `u32` discriminant from the C ABI into a `PluginType`.
    ///
    /// Returns `None` for unknown values.
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            1 => Some(Self::Content),
            2 => Some(Self::Theme),
            3 => Some(Self::Shortcut),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Priority constants
// ---------------------------------------------------------------------------

/// Playback — media players, podcasts, videos (lowest).
pub const PRIORITY_LOW: u32 = 0;
/// Activity — ongoing short-lived activities like timers, screen recording.
pub const PRIORITY_MEDIUM: u32 = 1;
/// Alert — notifications that need immediate attention (highest).
pub const PRIORITY_HIGH: u32 = 2;

// ---------------------------------------------------------------------------
// Result type
// ---------------------------------------------------------------------------

/// The return type for fallible plugin host calls.
///
/// This is a C-compatible equivalent of `Result<(), String>`.
#[repr(C)]
pub struct PluginResultC {
    /// `true` for success, `false` for failure.
    pub ok: bool,
    /// Null-terminated UTF-8 error message (max 255 bytes + NUL).
    pub error: [u8; 256],
}

impl PluginResultC {
    /// Construct a success result.
    pub fn ok() -> Self {
        Self {
            ok: true,
            error: [0u8; 256],
        }
    }

    /// Construct an error result with the given message.
    ///
    /// The message is truncated to 255 bytes if it exceeds the buffer.
    pub fn err(msg: &str) -> Self {
        let mut error = [0u8; 256];
        let bytes = msg.as_bytes();
        let len = bytes.len().min(255);
        error[..len].copy_from_slice(&bytes[..len]);
        Self { ok: false, error }
    }

    /// Convert back into a Rust `Result`.
    pub fn into_result(self) -> Result<(), String> {
        if self.ok {
            Ok(())
        } else {
            let end = self.error.iter().position(|&b| b == 0).unwrap_or(256);
            Err(String::from_utf8_lossy(&self.error[..end]).into_owned())
        }
    }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Fill a fixed-size byte buffer with a string, zeroing the rest.
///
/// Useful for initialising `#[repr(C)]` struct fields with a
/// null-terminated string. The string is truncated if it doesn't fit.
///
/// ```rust
/// use winisland_plugin_api::str_to_fixed;
/// let buf: [u8; 64] = str_to_fixed("hello");
/// assert_eq!(&buf[..6], b"hello\0");
/// assert_eq!(buf[6..].iter().all(|&b| b == 0), true);
/// ```
pub fn str_to_fixed<const N: usize>(s: &str) -> [u8; N] {
    let mut buf = [0u8; N];
    let len = s.len().min(N - 1);
    buf[..len].copy_from_slice(&s.as_bytes()[..len]);
    buf
}

// ---------------------------------------------------------------------------
// Core ABI types
// ---------------------------------------------------------------------------

/// Fixed-width plugin metadata exchanged over FFI.
///
/// Every field is a fixed-size byte buffer. The host reads them via
/// [`read_c_str`] and [`read_opt_c_str`] helpers.
#[repr(C)]
pub struct PluginMetadataC {
    /// Unique identifier (e.g. `"my-awesome-plugin"`). Max 63 bytes + NUL.
    pub id: [u8; 64],
    /// Human-readable name. Max 127 bytes + NUL.
    pub name: [u8; 128],
    /// Semver version string (e.g. `"1.0.0"`). Max 31 bytes + NUL.
    pub version: [u8; 32],
    /// Author name. Max 127 bytes + NUL.
    pub author: [u8; 128],
    /// Description. Max 255 bytes + NUL.
    pub description: [u8; 256],
}

/// Content data pushed to the WinIsland island display.
///
/// The `tag` field selects which content variant the host should render.
#[repr(C)]
pub struct IslandContentC {
    /// Discriminant: [`ISLAND_CONTENT_TAG_MUSIC`], [`ISLAND_CONTENT_TAG_NOTIFICATION`],
    /// or [`ISLAND_CONTENT_TAG_STATUS`].
    pub tag: u32,
    /// Title text (e.g. song title, notification subject). Max 255 bytes + NUL.
    pub title: [u8; 256],
    /// Artist / subtitle text. Max 255 bytes + NUL.
    pub artist: [u8; 256],
    /// URL to cover album art or notification icon. Max 511 bytes + NUL.
    pub cover_url: [u8; 512],
    /// Playback state.
    pub is_playing: bool,
    /// Notification body / extra message. Max 255 bytes + NUL.
    pub message: [u8; 256],
    /// Status metric label (e.g. "CPU"). Max 127 bytes + NUL.
    pub label: [u8; 128],
    /// Status metric value (e.g. "45%"). Max 127 bytes + NUL.
    pub value: [u8; 128],
}

/// Content variant: music playback info with cover art.
pub const ISLAND_CONTENT_TAG_MUSIC: u32 = 1;
/// Content variant: system / app notification.
pub const ISLAND_CONTENT_TAG_NOTIFICATION: u32 = 2;
/// Content variant: status metric (CPU, memory, etc.).
pub const ISLAND_CONTENT_TAG_STATUS: u32 = 3;

/// Colour palette returned by a theme plugin.
///
/// Each colour is `[R, G, B, A]` in sRGB.
#[repr(C)]
pub struct ThemeColorsC {
    pub primary: [u8; 4],
    pub secondary: [u8; 4],
    pub background: [u8; 4],
    pub text: [u8; 4],
    pub border: [u8; 4],
}

/// Animation timing configuration for island transitions.
#[repr(C)]
pub struct AnimationConfigC {
    /// Expand transition duration in milliseconds.
    pub expand_duration_ms: u32,
    /// Collapse transition duration in milliseconds.
    pub collapse_duration_ms: u32,
    /// Spring bounce intensity (0.0 = no bounce, typical range 0.3–0.8).
    pub bounce_intensity: f32,
}

/// A keyboard shortcut or quick action exposed by a plugin.
#[repr(C)]
pub struct ShortcutC {
    /// Stable identifier used in `execute_shortcut` calls. Max 63 bytes + NUL.
    pub id: [u8; 64],
    /// Display name shown in the shortcut palette. Max 127 bytes + NUL.
    pub name: [u8; 128],
    /// One-line description of what the shortcut does. Max 255 bytes + NUL.
    pub description: [u8; 256],
    /// Optional icon hint. Max 255 bytes + NUL.
    pub icon: [u8; 256],
    /// Optional hotkey binding (e.g. `"Ctrl+Shift+M"`). Max 31 bytes + NUL.
    pub hotkey: [u8; 32],
}

// ---------------------------------------------------------------------------
// Context (push-based) types — added in 0.2
// ---------------------------------------------------------------------------

/// Context data sent by a plugin to display on the Dynamic Island.
#[repr(C)]
pub struct ContextDataC {
    /// Priority: [`PRIORITY_LOW`], [`PRIORITY_MEDIUM`], [`PRIORITY_HIGH`].
    /// Defaults to `PRIORITY_MEDIUM` when zero.
    pub priority: u32,
    /// Title text shown in mini and expanded views. Max 255 bytes + NUL.
    pub title: [u8; 256],
    /// Expanded body text. Max 511 bytes + NUL.
    pub body: [u8; 512],
    /// How many seconds the expanded view stays open before collapsing.
    pub duration_sec: u32,
    /// Whether to show a mini summary after collapsing.
    pub mini_render: bool,
    /// Mini summary text (used when `mini_render` is true). Max 127 bytes + NUL.
    pub mini_text: [u8; 128],
}

/// Opaque context identifier returned by `send_context`.
#[repr(C)]
pub struct ContextIdC {
    /// Encoded as `"plugin_id:uuid"`. Max 127 bytes + NUL.
    pub id: [u8; 128],
}

/// Snapshot of the current host state that a plugin can query.
#[repr(C)]
pub struct HostStateC {
    /// Currently playing media title. Max 255 bytes + NUL.
    pub media_title: [u8; 256],
    /// Currently playing media artist. Max 255 bytes + NUL.
    pub media_artist: [u8; 256],
    /// Whether media is currently playing.
    pub is_playing: bool,
    /// Current theme: `"light"` or `"dark"`. Max 31 bytes + NUL.
    pub theme: [u8; 32],
}

/// Media source data pushed by a plugin to replace SMTC playback info.
///
/// When a plugin calls `set_media_source`, the host uses this data
/// instead of the current SMTC session for the entire media UI
/// (progress bar, cover art, title/artist, playback controls).
/// Call `clear_media_source` to restore SMTC as the source.
#[repr(C)]
pub struct MediaSourceC {
    /// Track title. Max 255 bytes + NUL.
    pub title: [u8; 256],
    /// Artist name. Max 255 bytes + NUL.
    pub artist: [u8; 256],
    /// Album name. Max 255 bytes + NUL.
    pub album: [u8; 256],
    /// Total duration in milliseconds.
    pub duration_ms: u64,
    /// Current playback position in milliseconds.
    pub position_ms: u64,
    /// Whether the media is currently playing.
    pub is_playing: bool,
    /// Raw cover art bytes (JPEG/PNG). Null pointer if no cover.
    pub cover_data: *const u8,
    /// Length of `cover_data` in bytes. 0 if no cover.
    pub cover_len: u32,
}

/// Host-side API table passed to plugins via [`PluginVTable::set_host_api`].
///
/// Plugins store this pointer during `set_host_api` and call through it
/// whenever they need to interact with the host (push context, close context,
/// query state). All functions are safe to call from any thread.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HostApiC {
    /// Push a new context to the Dynamic Island.
    ///
    /// Returns a [`ContextIdC`] that can be used later to close or update.
    pub send_context: unsafe extern "C" fn(PluginHandle, ContextDataC) -> ContextIdC,

    /// Close a previously sent context by its ID.
    pub close_context: unsafe extern "C" fn(PluginHandle, *const c_char) -> PluginResultC,

    /// Query the current host state.
    pub query_host_state: unsafe extern "C" fn(PluginHandle) -> HostStateC,

    /// Replace SMTC with plugin-provided media source.
    ///
    /// The host will use this data for the entire media UI. Returns an
    /// error if `title` is empty. Call [`clear_media_source`] to restore SMTC.
    pub set_media_source: unsafe extern "C" fn(PluginHandle, MediaSourceC) -> PluginResultC,

    /// Restore SMTC as the active media source and stop using plugin data.
    pub clear_media_source: unsafe extern "C" fn(PluginHandle) -> PluginResultC,
}

// ---------------------------------------------------------------------------
// Plugin vtable
// ---------------------------------------------------------------------------

/// Virtual function table that every plugin DLL must expose.
///
/// This is the **core of the plugin ABI**: the host calls through these
/// function pointers on the plugin's handle. Required fields (`on_load`,
/// `on_unload`, `destroy`) must always be non-null. Optional fields
/// may be `None` if the plugin doesn't support that capability.
#[repr(C)]
pub struct PluginVTable {
    /// Called when the plugin is first loaded. Perform one-time initialisation.
    ///
    /// **Must be non-null.** Return `PluginResultC::ok()` on success.
    pub on_load: unsafe extern "C" fn(PluginHandle) -> PluginResultC,

    /// Called when the plugin is about to be unloaded. Release resources.
    ///
    /// **Must be non-null.** The return value is logged but does not
    /// prevent unloading.
    pub on_unload: unsafe extern "C" fn(PluginHandle) -> PluginResultC,

    /// Final destructor called after `on_unload`. Free the `handle`.
    ///
    /// **Must be non-null.** After this returns, the handle pointer
    /// becomes invalid.
    pub destroy: unsafe extern "C" fn(PluginHandle),

    /// Return the current island content to display.
    ///
    /// Required for [`PluginType::Content`] plugins. The host polls this
    /// at a regular interval.
    pub get_content: Option<unsafe extern "C" fn(PluginHandle) -> IslandContentC>,

    /// Called when the user clicks on the plugin's content area.
    pub on_click: Option<unsafe extern "C" fn(PluginHandle)>,

    /// Called when the island expands / collapses.
    ///
    /// `true` = expanded, `false` = collapsed.
    pub on_expanded: Option<unsafe extern "C" fn(PluginHandle, bool)>,

    /// Whether this content plugin supports an expanded view.
    ///
    /// If `None`, the host assumes no expanded view.
    pub supports_expand: Option<unsafe extern "C" fn(PluginHandle) -> bool>,

    /// Return the current theme colours.
    ///
    /// Required for [`PluginType::Theme`] plugins.
    pub get_colors: Option<unsafe extern "C" fn(PluginHandle) -> ThemeColorsC>,

    /// Return animation timing configuration.
    ///
    /// Required for [`PluginType::Theme`] plugins.
    pub get_animations: Option<unsafe extern "C" fn(PluginHandle) -> AnimationConfigC>,

    /// Number of shortcuts exposed by this plugin.
    ///
    /// Required for [`PluginType::Shortcut`] plugins.
    pub get_shortcuts_count: Option<unsafe extern "C" fn(PluginHandle) -> u32>,

    /// Write the `i`-th shortcut (0-indexed) into the output buffer.
    ///
    /// Required for [`PluginType::Shortcut`] plugins.
    pub get_shortcut_at: Option<unsafe extern "C" fn(PluginHandle, i: u32, out: *mut ShortcutC)>,

    /// Execute the shortcut identified by `id`.
    ///
    /// Required for [`PluginType::Shortcut`] plugins.
    pub execute_shortcut:
        Option<unsafe extern "C" fn(PluginHandle, id: *const c_char) -> PluginResultC>,

    /// Give the plugin a pointer to the host API table.
    ///
    /// Called after `on_load`. The plugin should store this pointer
    /// and use it to call `send_context`, `close_context`, etc.
    /// May be `None` for plugins that don't need host interaction.
    pub set_host_api: Option<unsafe extern "C" fn(PluginHandle, *const HostApiC)>,
}

/// The complete plugin instance returned by the DLL's entry point.
///
/// Every plugin DLL must export a `plugin_get_instance` function
/// returning one of these:
///
/// ```rust,no_run
/// # use winisland_plugin_api::*;
/// #[no_mangle]
/// pub unsafe extern "C" fn plugin_get_instance() -> PluginInstanceC {
///     PluginInstanceC {
///         handle: std::ptr::null_mut(),
///         metadata: PluginMetadataC {
///             id: str_to_fixed("my-plugin"),
///             name: str_to_fixed("My Plugin"),
///             version: str_to_fixed("1.0.0"),
///             author: str_to_fixed("Me"),
///             description: str_to_fixed("Does cool stuff"),
///         },
///         vtable: &VTABLE,
///         plugin_type: PluginType::Content as u32,
///     }
/// }
/// ```
#[repr(C)]
pub struct PluginInstanceC {
    /// Opaque handle passed back to every vtable call.
    pub handle: PluginHandle,
    /// Plugin identity metadata.
    pub metadata: PluginMetadataC,
    /// Pointer to the virtual function table.
    ///
    /// The vtable must remain valid for the lifetime of the handle.
    pub vtable: *const PluginVTable,
    /// Plugin type discriminant ([`PluginType`]).
    pub plugin_type: u32,
}

/// Entry-point function signature that every plugin DLL must export.
///
/// ```ignore
/// #[no_mangle]
/// pub unsafe extern "C" fn plugin_get_instance() -> PluginInstanceC;
/// ```
pub type PluginGetInstanceFn = unsafe extern "C" fn() -> PluginInstanceC;
