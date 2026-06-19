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
