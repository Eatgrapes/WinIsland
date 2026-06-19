pub mod content;
pub mod context;
pub mod i18n;
pub mod metadata;
pub mod shortcut;
pub mod theme;

/// Opaque handle to a plugin instance, passed through every vtable call.
pub type PluginHandle = *mut std::ffi::c_void;

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
