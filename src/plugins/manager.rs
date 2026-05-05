use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::path::{Path, PathBuf};
use libloading::{Library, Symbol};
use crate::core::config::AppConfig;
use super::api::{Plugin, PluginResult, PluginError, PluginInfo, PluginContext};

type CreatePlugin = unsafe extern "C" fn() -> *mut dyn Plugin;

pub struct PluginManager {
    plugins: RwLock<HashMap<String, Arc<Box<dyn Plugin>>>>,
    libraries: Mutex<HashMap<String, Library>>,
    context: PluginContext,
    plugins_path: PathBuf,
}

impl PluginManager {
    pub fn new(config: Arc<AppConfig>) -> Self {
        let mut plugins_path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        plugins_path.push("WinIsland");
        plugins_path.push("plugins");
        
        std::fs::create_dir_all(&plugins_path).ok();
        
        Self {
            plugins: RwLock::new(HashMap::new()),
            libraries: Mutex::new(HashMap::new()),
            context: PluginContext::new(config),
            plugins_path,
        }
    }
    
    pub fn load_all_plugins(&self) -> Vec<Result<PluginInfo, PluginError>> {
        let mut results = Vec::new();
        
        if !self.plugins_path.exists() {
            return results;
        }
        
        if let Ok(entries) = std::fs::read_dir(&self.plugins_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map(|e| e == "dll").unwrap_or(false) {
                    match self.load_plugin(&path) {
                        Ok(info) => results.push(Ok(info)),
                        Err(e) => results.push(Err(e)),
                    }
                }
            }
        }
        
        results
    }
    
    pub fn load_plugin(&self, path: &Path) -> PluginResult<PluginInfo> {
        let lib = unsafe { Library::new(path) }
            .map_err(|e| PluginError::LoadError(format!("Failed to load library: {}", e)))?;
        
        let create: Symbol<CreatePlugin> = unsafe {
            lib.get(b"winisland_plugin_create")
                .map_err(|e| PluginError::InvalidPlugin(format!("Missing symbol: {}", e)))?
        };
        
        let plugin_ptr = unsafe { create() };
        let mut plugin = unsafe { Box::from_raw(plugin_ptr) };
        
        match plugin.init() {
            Ok(_) => {},
            Err(e) => return Err(PluginError::InitError(format!("{}", e))),
        }
        
        let name = plugin.name().to_string();
        let info = PluginInfo {
            name: plugin.name().to_string(),
            version: plugin.version().to_string(),
            author: plugin.author().to_string(),
            description: plugin.description().to_string(),
        };
        
        self.plugins.write().unwrap().insert(name.clone(), Arc::new(plugin));
        self.libraries.lock().unwrap().insert(name, lib);
        
        Ok(info)
    }
    
    pub fn unload_plugin(&self, name: &str) -> PluginResult {
        if let Some(plugin) = self.plugins.write().unwrap().remove(name) {
            plugin.on_shutdown()?;
        }
        
        self.libraries.lock().unwrap().remove(name);
        
        Ok(())
    }
    
    pub fn get_plugins(&self) -> Vec<PluginInfo> {
        self.plugins.read().unwrap()
            .values()
            .map(|p| PluginInfo {
                name: p.name().to_string(),
                version: p.version().to_string(),
                author: p.author().to_string(),
                description: p.description().to_string(),
            })
            .collect()
    }
    
    pub fn get_plugin_by_name(&self, name: &str) -> Option<Arc<Box<dyn Plugin>>> {
        self.plugins.read().unwrap().get(name).cloned()
    }
    
    pub fn call_on_media_change(&self, media: &crate::core::smtc::MediaInfo) {
        for plugin in self.plugins.read().unwrap().values() {
            let _ = plugin.on_media_change(media);
        }
    }
    
    pub fn call_on_playback_state_change(&self, is_playing: bool) {
        for plugin in self.plugins.read().unwrap().values() {
            let _ = plugin.on_playback_state_change(is_playing);
        }
    }
    
    pub fn call_on_audio_spectrum(&self, spectrum: [f32; 6]) {
        for plugin in self.plugins.read().unwrap().values() {
            let _ = plugin.on_audio_spectrum(spectrum);
        }
    }
    
    pub fn call_on_render(&self, canvas: &mut skia_safe::Canvas, x: f32, y: f32, w: f32, h: f32) {
        for plugin in self.plugins.read().unwrap().values() {
            let _ = plugin.on_render(canvas, x, y, w, h);
        }
    }
    
    pub fn call_on_tick(&self, delta_time: f32) {
        for plugin in self.plugins.read().unwrap().values() {
            let _ = plugin.on_tick(delta_time);
        }
    }
    
    pub fn call_on_key_press(&self, key: &str) -> bool {
        for plugin in self.plugins.read().unwrap().values() {
            if let Ok(handled) = plugin.on_key_press(key) {
                if handled {
                    return true;
                }
            }
        }
        false
    }
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        for plugin in self.plugins.write().unwrap().values() {
            let _ = plugin.on_shutdown();
        }
    }
}
