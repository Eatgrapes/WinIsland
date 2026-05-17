#![allow(dead_code)]

use super::loader::NativePlugin;
use super::types::{
    ContentProvider, Plugin, PluginError, PluginType, ShortcutProvider, ThemeProvider,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

pub struct PluginManager {
    entries: Arc<RwLock<Vec<NativePlugin>>>,
    plugin_dir: PathBuf,
}

impl PluginManager {
    pub fn new<P: AsRef<Path>>(plugin_dir: P) -> Self {
        let plugin_dir = plugin_dir.as_ref().to_path_buf();
        let _ = std::fs::create_dir_all(&plugin_dir);

        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            plugin_dir,
        }
    }

    pub fn load_all(&self) {
        let dlls = discover_plugins(&self.plugin_dir);
        for dll_path in dlls {
            match NativePlugin::load(&dll_path) {
                Ok(native) => {
                    log::info!(
                        "Loaded plugin: {} ({})",
                        native.metadata().name,
                        native.metadata().id
                    );
                    if let Ok(mut entries) = self.entries.write() {
                        entries.push(native);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to load plugin '{}': {}", dll_path.display(), e);
                }
            }
        }
    }

    pub fn unload(&self, plugin_id: &str) -> Result<(), PluginError> {
        let mut entries = self.entries.write().unwrap();
        let idx = entries
            .iter()
            .position(|p| p.metadata().id == plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;
        entries.remove(idx);
        Ok(())
    }

    pub fn list_content_providers(&self) -> Vec<String> {
        self.entries
            .read()
            .unwrap()
            .iter()
            .filter(|p| p.plugin_type() == PluginType::ContentProvider)
            .map(|p| p.metadata().id.clone())
            .collect()
    }

    pub fn list_theme_providers(&self) -> Vec<String> {
        self.entries
            .read()
            .unwrap()
            .iter()
            .filter(|p| p.plugin_type() == PluginType::ThemeProvider)
            .map(|p| p.metadata().id.clone())
            .collect()
    }

    pub fn list_shortcut_providers(&self) -> Vec<String> {
        self.entries
            .read()
            .unwrap()
            .iter()
            .filter(|p| p.plugin_type() == PluginType::ShortcutProvider)
            .map(|p| p.metadata().id.clone())
            .collect()
    }

    pub fn with_content<F, R>(&self, plugin_id: &str, f: F) -> Result<R, PluginError>
    where
        F: FnOnce(&dyn ContentProvider) -> R,
    {
        let entries = self.entries.read().unwrap();
        let entry = entries
            .iter()
            .find(|p| p.metadata().id == plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        if entry.plugin_type() != PluginType::ContentProvider {
            return Err(PluginError::InvalidPlugin(format!(
                "Plugin '{}' is not a ContentProvider",
                plugin_id
            )));
        }

        Ok(f(entry))
    }

    pub fn with_content_mut<F, R>(&self, plugin_id: &str, f: F) -> Result<R, PluginError>
    where
        F: FnOnce(&mut dyn ContentProvider) -> R,
    {
        let mut entries = self.entries.write().unwrap();
        let entry = entries
            .iter_mut()
            .find(|p| p.metadata().id == plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        if entry.plugin_type() != PluginType::ContentProvider {
            return Err(PluginError::InvalidPlugin(format!(
                "Plugin '{}' is not a ContentProvider",
                plugin_id
            )));
        }

        Ok(f(entry))
    }

    pub fn with_theme<F, R>(&self, plugin_id: &str, f: F) -> Result<R, PluginError>
    where
        F: FnOnce(&dyn ThemeProvider) -> R,
    {
        let entries = self.entries.read().unwrap();
        let entry = entries
            .iter()
            .find(|p| p.metadata().id == plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        if entry.plugin_type() != PluginType::ThemeProvider {
            return Err(PluginError::InvalidPlugin(format!(
                "Plugin '{}' is not a ThemeProvider",
                plugin_id
            )));
        }

        Ok(f(entry))
    }

    pub fn with_shortcut_mut<F, R>(&self, plugin_id: &str, f: F) -> Result<R, PluginError>
    where
        F: FnOnce(&mut dyn ShortcutProvider) -> R,
    {
        let mut entries = self.entries.write().unwrap();
        let entry = entries
            .iter_mut()
            .find(|p| p.metadata().id == plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        if entry.plugin_type() != PluginType::ShortcutProvider {
            return Err(PluginError::InvalidPlugin(format!(
                "Plugin '{}' is not a ShortcutProvider",
                plugin_id
            )));
        }

        Ok(f(entry))
    }

    pub fn plugin_dir(&self) -> &Path {
        &self.plugin_dir
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        let dir = dirs::config_dir()
            .unwrap_or_default()
            .join("WinIsland")
            .join("plugins");
        Self::new(dir)
    }
}

fn discover_plugins(plugin_dir: &Path) -> Vec<PathBuf> {
    if !plugin_dir.exists() {
        return Vec::new();
    }

    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(plugin_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "dll") {
                result.push(path);
            }
        }
    }
    result
}
