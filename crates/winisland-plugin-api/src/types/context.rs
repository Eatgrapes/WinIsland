/// Playback — media players, podcasts, videos (lowest).
pub const PRIORITY_LOW: u32 = 0;
/// Activity — ongoing short-lived activities like timers, screen recording.
pub const PRIORITY_MEDIUM: u32 = 1;
/// Alert — notifications that need immediate attention (highest).
pub const PRIORITY_HIGH: u32 = 2;

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
