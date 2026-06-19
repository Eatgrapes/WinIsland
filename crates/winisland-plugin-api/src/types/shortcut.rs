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
