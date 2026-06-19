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
