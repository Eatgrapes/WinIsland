use std::sync::Arc;
use skia_safe::Canvas;
use std::fmt;
use crate::core::smtc::MediaInfo;

pub type PluginResult<T = ()> = Result<T, PluginError>;

#[derive(Debug, Clone)]
pub enum PluginError {
    LoadError(String),
    InitError(String),
    ExecutionError(String),
    InvalidPlugin(String),
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PluginError::LoadError(s) => write!(f, "Load error: {}", s),
            PluginError::InitError(s) => write!(f, "Init error: {}", s),
            PluginError::ExecutionError(s) => write!(f, "Execution error: {}", s),
            PluginError::InvalidPlugin(s) => write!(f, "Invalid plugin: {}", s),
        }
    }
}

pub trait Plugin: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn author(&self) -> &str;
    fn description(&self) -> &str;
    
    fn init(&mut self) -> PluginResult { Ok(()) }
    
    fn on_media_change(&self, _media: &MediaInfo) -> PluginResult { Ok(()) }
    fn on_playback_state_change(&self, _is_playing: bool) -> PluginResult { Ok(()) }
    fn on_audio_spectrum(&self, _spectrum: [f32; 6]) -> PluginResult { Ok(()) }
    fn on_render(&self, _canvas: &mut Canvas, _x: f32, _y: f32, _w: f32, _h: f32) -> PluginResult { Ok(()) }
    fn on_tick(&self, _delta_time: f32) -> PluginResult { Ok(()) }
    fn on_key_press(&self, _key: &str) -> PluginResult<bool> { Ok(false) }
    
    fn on_shutdown(&self) -> PluginResult { Ok(()) }
}

pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
}

impl From<&dyn Plugin> for PluginInfo {
    fn from(plugin: &dyn Plugin) -> Self {
        Self {
            name: plugin.name().to_string(),
            version: plugin.version().to_string(),
            author: plugin.author().to_string(),
            description: plugin.description().to_string(),
        }
    }
}

pub struct PluginContext {
    pub config: Arc<crate::core::config::AppConfig>,
}

impl PluginContext {
    pub fn new(config: Arc<crate::core::config::AppConfig>) -> Self {
        Self { config }
    }
}

#[macro_export]
macro_rules! declare_plugin {
    ($struct_name:ident, $name:expr, $version:expr, $author:expr, $description:expr) => {
        pub struct $struct_name;
        
        impl Plugin for $struct_name {
            fn name(&self) -> &str { $name }
            fn version(&self) -> &str { $version }
            fn author(&self) -> &str { $author }
            fn description(&self) -> &str { $description }
        }
        
        #[no_mangle]
        pub extern "C" fn winisland_plugin_create() -> *mut dyn Plugin {
            Box::into_raw(Box::new($struct_name))
        }
    };
}
