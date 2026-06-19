use crate::types::context::MediaSourceC;
use crate::types::context::{ContextDataC, ContextIdC, HostStateC};
use crate::types::i18n::TranslationPairC;
use crate::types::{PluginHandle, PluginResultC};
use std::ffi::c_char;

/// Host-side API table passed to plugins via [`PluginVTable::set_host_api`](crate::PluginVTable).
///
/// Plugins store this pointer during `set_host_api` and call through it
/// whenever they need to interact with the host (push context, close context,
/// query state, register translations). All functions are safe to call from any thread.
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

    /// Register translations for a language.
    ///
    /// Called during `on_load` to provide translated strings for the plugin's UI.
    /// Later registrations override earlier ones for the same key.
    /// The host copies the strings; the plugin may free them after the call returns.
    pub register_translations: unsafe extern "C" fn(
        PluginHandle,
        *const c_char,
        *const TranslationPairC,
        u32,
    ) -> PluginResultC,
}
