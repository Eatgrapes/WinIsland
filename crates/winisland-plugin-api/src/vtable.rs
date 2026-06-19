use crate::HostApiC;
use crate::types::content::IslandContentC;
use crate::types::metadata::PluginMetadataC;
use crate::types::shortcut::ShortcutC;
use crate::types::theme::{AnimationConfigC, ThemeColorsC};
use crate::types::{PluginHandle, PluginResultC};

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
        Option<unsafe extern "C" fn(PluginHandle, id: *const std::ffi::c_char) -> PluginResultC>,

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
