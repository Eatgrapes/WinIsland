#![allow(dead_code)]

use super::types::{
    AnimationConfig, ContentProvider, IslandContent, Plugin, PluginError, PluginGetInstanceFn,
    PluginHandle, PluginInstanceC, PluginMetadata, PluginResultC, PluginType, Shortcut, ShortcutC,
    ShortcutProvider, ThemeColors, ThemeProvider,
};
use libloading::Library;
use std::path::Path;

/// Wrapper around a native DLL plugin. Implements host-side traits by
/// calling through the C ABI vtable, avoiding any trait-object crossing
/// the FFI boundary.
pub struct NativePlugin {
    metadata: PluginMetadata,
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

        let metadata = PluginMetadata::from(&instance.metadata);
        let plugin_type = PluginType::from_u32(instance.plugin_type).ok_or_else(|| {
            PluginError::InvalidPlugin(format!(
                "Plugin '{}' has unknown plugin_type: {}",
                path.display(),
                instance.plugin_type
            ))
        })?;

        let plugin = Self {
            metadata,
            plugin_type,
            handle: instance.handle,
            vtable: instance.vtable,
            _lib: lib,
        };

        // SAFETY: vtable pointer was validated non-null above and is 'static for the DLL's lifetime.
        let vtable = unsafe { &*plugin.vtable };
        // SAFETY: calling the plugin's on_load via vtable with its own handle.
        let result: PluginResultC = unsafe { (vtable.on_load)(plugin.handle) };
        result.into_result().map_err(|e| {
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
    fn metadata(&self) -> &PluginMetadata {
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
            let c = unsafe { f(self.handle) };
            IslandContent::from(&c)
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
                ThemeColors::from(&unsafe { f(self.handle) })
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
                AnimationConfig::from(&unsafe { f(self.handle) })
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
        let vtable = self.vtable();
        let count = match vtable.get_shortcuts_count {
            // SAFETY: calling through vtable with the opaque handle from the same DLL.
            Some(f) => unsafe { f(self.handle) },
            None => return Vec::new(),
        };
        let get_at = match vtable.get_shortcut_at {
            Some(f) => f,
            None => return Vec::new(),
        };
        let mut shortcuts = Vec::with_capacity(count as usize);
        for i in 0..count {
            let mut c = ShortcutC {
                id: [0u8; 64],
                name: [0u8; 128],
                description: [0u8; 256],
                icon: [0u8; 256],
                hotkey: [0u8; 32],
            };
            // SAFETY: calling through vtable with opaque handle; &mut c is a valid
            // pointer to a stack-allocated ShortcutC struct for the DLL to fill.
            unsafe { get_at(self.handle, i, &mut c) };
            shortcuts.push(Shortcut {
                id: super::types::read_c_str(&c.id),
                name: super::types::read_c_str(&c.name),
                description: super::types::read_c_str(&c.description),
                icon: super::types::read_opt_c_str(&c.icon),
                hotkey: super::types::read_opt_c_str(&c.hotkey),
            });
        }
        shortcuts
    }

    fn execute(&mut self, shortcut_id: &str) -> Result<(), String> {
        let vtable = self.vtable();
        match vtable.execute_shortcut {
            Some(f) => {
                let mut id_bytes = [0i8; 128];
                let bytes = shortcut_id.as_bytes();
                let len = bytes.len().min(127);
                if bytes.len() > 127 {
                    log::warn!("shortcut_id '{}' truncated to 127 bytes", shortcut_id);
                }
                for (i, &b) in bytes[..len].iter().enumerate() {
                    id_bytes[i] = b as i8;
                }
                unsafe { f(self.handle, id_bytes.as_ptr()).into_result() }
            }
            None => Err("Plugin does not support execute_shortcut".into()),
        }
    }
}

impl Drop for NativePlugin {
    fn drop(&mut self) {
        let vtable = self.vtable();
        // SAFETY: on_unload and destroy are called with the plugin's own handle
        // during drop, which is the correct lifecycle point for cleanup.
        // The vtable and handle are from the same DLL and remain valid.
        unsafe {
            let _ = (vtable.on_unload)(self.handle);
            (vtable.destroy)(self.handle);
        }
    }
}
