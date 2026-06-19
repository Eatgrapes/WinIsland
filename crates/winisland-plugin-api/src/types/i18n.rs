use std::ffi::c_char;

/// A translation key-value pair registered by a plugin via
/// [`HostApiC::register_translations`](crate::HostApiC).
///
/// Both `key` and `value` are null-terminated UTF-8 strings.
/// The host copies the strings immediately; the plugin may free
/// them after the call returns.
#[repr(C)]
pub struct TranslationPairC {
    /// Translation key (e.g. `"greeting"`).
    pub key: *const c_char,
    /// Translation value (e.g. `"Bonjour"`).
    pub value: *const c_char,
}
