#![allow(dead_code)]

use super::types::{
    AnimationConfig, ContentProvider, IslandContent, Plugin, PluginError, PluginGetInstanceFn,
    PluginHandle, PluginInstanceC, PluginResultC, PluginType, Shortcut, ShortcutProvider,
    ThemeColors, ThemeProvider,
};
use libloading::Library;
use std::path::Path;

/// Wrapper around a native DLL plugin. Implements host-side traits by
/// calling through the C ABI vtable, avoiding any trait-object crossing
/// the FFI boundary.
pub struct NativePlugin {
    metadata: super::types::PluginMetadata,
    plugin_type: PluginType,
    handle: PluginHandle,
    vtable: *const super::types::PluginVTable,
    _lib: Library,
}

// SAFETY: PluginHandle is Send+Sync by convention; the DLL must be thread-safe.
unsafe impl Send for NativePlugin {}
unsafe impl Sync for NativePlugin {}

impl NativePlugin {
    /// Load a native plugin from a DLL file.
    ///
    /// The DLL must export a `plugin_get_instance` symbol with signature:
    /// `unsafe extern "C" fn() -> PluginInstanceC`
    pub fn load(path: &Path) -> Result<Self, PluginError> {
        // SAFETY: libloading loads a DLL; we assume the provided path is trustworthy.
        let lib = unsafe {
            Library::new(path).map_err(|e| {
                PluginError::LoadFailed(format!(
                    "Failed to load library '{}': {}",
                    path.display(),
                    e
                ))
            })?
        };

        // SAFETY: we call the exported symbol with the expected C ABI signature.
        // The DLL author is responsible for returning a valid PluginInstanceC.
        let get_instance: libloading::Symbol<PluginGetInstanceFn> = unsafe {
            lib.get(b"plugin_get_instance").map_err(|e| {
                PluginError::InvalidPlugin(format!(
                    "Plugin '{}' does not export 'plugin_get_instance': {}",
                    path.display(),
                    e
                ))
            })?
        };

        let instance: PluginInstanceC = unsafe { get_instance() };

        if instance.handle.is_null() {
            return Err(PluginError::LoadFailed(format!(
                "Plugin '{}' returned null handle",
                path.display()
            )));
        }

        if instance.vtable.is_null() {
            return Err(PluginError::InvalidPlugin(format!(
                "Plugin '{}' returned null vtable",
                path.display()
            )));
        }

        let metadata = instance.metadata.to_metadata();
        let plugin_type = instance.plugin_type_enum();

        let plugin = Self {
            metadata,
            plugin_type,
            handle: instance.handle,
            vtable: instance.vtable,
            _lib: lib,
        };

        let vtable = unsafe { &*plugin.vtable };
        let result: PluginResultC = unsafe { (vtable.on_load)(plugin.handle) };
        result.to_result().map_err(|e| {
            PluginError::ExecutionError(format!(
                "Plugin '{}' on_load failed: {}",
                plugin.metadata.id, e
            ))
        })?;

        Ok(plugin)
    }

    fn vtable(&self) -> &super::types::PluginVTable {
        // SAFETY: vtable was validated on construction and is 'static in the DLL.
        unsafe { &*self.vtable }
    }
}

impl Plugin for NativePlugin {
    fn metadata(&self) -> &super::types::PluginMetadata {
        &self.metadata
    }

    fn plugin_type(&self) -> PluginType {
        self.plugin_type
    }
}

impl ContentProvider for NativePlugin {
    fn get_content(&self) -> Option<IslandContent> {
        let vtable = self.vtable();
        vtable.get_content.map(|f| {
            // SAFETY: calling through vtable with the opaque handle from the same DLL.
            let c: super::types::IslandContentC = unsafe { f(self.handle) };
            c.to_content().unwrap_or(IslandContent::Status {
                label: String::new(),
                value: String::new(),
                icon: None,
            })
        })
    }

    fn on_click(&mut self) {
        let vtable = self.vtable();
        if let Some(f) = vtable.on_click {
            // SAFETY: calling through vtable with the opaque handle from the same DLL.
            unsafe { f(self.handle) };
        }
    }

    fn on_expanded(&mut self, expanded: bool) {
        let vtable = self.vtable();
        if let Some(f) = vtable.on_expanded {
            // SAFETY: calling through vtable with the opaque handle from the same DLL.
            unsafe { f(self.handle, expanded) };
        }
    }

    fn supports_expand(&self) -> bool {
        let vtable = self.vtable();
        vtable
            .supports_expand
            .map(|f| {
                // SAFETY: calling through vtable with the opaque handle from the same DLL.
                unsafe { f(self.handle) }
            })
            .unwrap_or(false)
    }
}

impl ThemeProvider for NativePlugin {
    fn get_colors(&self) -> ThemeColors {
        let vtable = self.vtable();
        vtable
            .get_colors
            .map(|f| {
                // SAFETY: calling through vtable with the opaque handle from the same DLL.
                unsafe { f(self.handle) }.to_colors()
            })
            .unwrap_or(ThemeColors {
                primary: (255, 255, 255, 255),
                secondary: (200, 200, 200, 255),
                background: (30, 30, 30, 255),
                text: (255, 255, 255, 255),
                border: (100, 100, 100, 255),
            })
    }

    fn get_animations(&self) -> AnimationConfig {
        let vtable = self.vtable();
        vtable
            .get_animations
            .map(|f| {
                // SAFETY: calling through vtable with the opaque handle from the same DLL.
                unsafe { f(self.handle) }.to_config()
            })
            .unwrap_or(AnimationConfig {
                expand_duration_ms: 300,
                collapse_duration_ms: 300,
                bounce_intensity: 0.5,
            })
    }
}

impl ShortcutProvider for NativePlugin {
    fn get_shortcuts(&self) -> Vec<Shortcut> {
        Vec::new()
    }

    fn execute(&mut self, _shortcut_id: &str) -> Result<(), String> {
        Ok(())
    }
}

impl Drop for NativePlugin {
    fn drop(&mut self) {
        let vtable = self.vtable();
        // SAFETY: destroy frees the plugin state. Called exactly once in Drop.
        // Failure to invoke this is a plugin bug.
        unsafe { (vtable.destroy)(self.handle) };
    }
}
