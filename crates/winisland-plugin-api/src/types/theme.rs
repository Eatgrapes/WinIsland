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
