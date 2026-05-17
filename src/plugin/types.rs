#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// 插件元信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
}

/// 插件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginType {
    ContentProvider,
    ThemeProvider,
    ShortcutProvider,
}

/// 岛屿内容枚举
#[derive(Debug, Clone)]
pub enum IslandContent {
    Music {
        title: String,
        artist: String,
        cover_url: Option<String>,
        is_playing: bool,
    },
    Notification {
        title: String,
        message: String,
        icon_url: Option<String>,
    },
    Status {
        label: String,
        value: String,
        icon: Option<String>,
    },
    Shortcut {
        name: String,
        icon: Option<String>,
        action_id: String,
    },
    Custom(serde_json::Value),
}

/// 主题颜色
#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub primary: (u8, u8, u8, u8),
    pub secondary: (u8, u8, u8, u8),
    pub background: (u8, u8, u8, u8),
    pub text: (u8, u8, u8, u8),
    pub border: (u8, u8, u8, u8),
}

/// 动画配置
#[derive(Debug, Clone)]
pub struct AnimationConfig {
    pub expand_duration_ms: u32,
    pub collapse_duration_ms: u32,
    pub bounce_intensity: f32,
}

/// 快捷方式定义
#[derive(Debug, Clone)]
pub struct Shortcut {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
    pub hotkey: Option<String>,
}

/// 插件错误
#[derive(Debug)]
pub enum PluginError {
    NotFound(String),
    LoadFailed(String),
    InvalidPlugin(String),
    ExecutionError(String),
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "Plugin not found: {}", msg),
            Self::LoadFailed(msg) => write!(f, "Failed to load plugin: {}", msg),
            Self::InvalidPlugin(msg) => write!(f, "Invalid plugin: {}", msg),
            Self::ExecutionError(msg) => write!(f, "Plugin execution error: {}", msg),
        }
    }
}

impl std::error::Error for PluginError {}

// ---------------------------------------------------------------------------
// Host-side Plugin trait — what the application works with
// ---------------------------------------------------------------------------

pub trait Plugin: Send + Sync {
    fn metadata(&self) -> &PluginMetadata;
    fn plugin_type(&self) -> PluginType;
}

pub trait ContentProvider: Plugin {
    fn get_content(&self) -> Option<IslandContent>;
    fn on_click(&mut self);
    fn on_expanded(&mut self, expanded: bool);
    fn supports_expand(&self) -> bool;
}

pub trait ThemeProvider: Plugin {
    fn get_colors(&self) -> ThemeColors;
    fn get_animations(&self) -> AnimationConfig;
}

pub trait ShortcutProvider: Plugin {
    fn get_shortcuts(&self) -> Vec<Shortcut>;
    fn execute(&mut self, shortcut_id: &str) -> Result<(), String>;
}

// ---------------------------------------------------------------------------
// C ABI types — stable across compiler versions, safe to pass across FFI
// ---------------------------------------------------------------------------

pub type PluginHandle = *mut std::ffi::c_void;

#[repr(C)]
pub struct PluginResultC {
    pub ok: bool,
    pub error: [u8; 256],
}

impl PluginResultC {
    pub fn ok() -> Self {
        Self {
            ok: true,
            error: [0u8; 256],
        }
    }

    pub fn err(msg: &str) -> Self {
        let mut error = [0u8; 256];
        let bytes = msg.as_bytes();
        let len = bytes.len().min(255);
        error[..len].copy_from_slice(&bytes[..len]);
        Self { ok: false, error }
    }

    pub fn to_result(self) -> Result<(), String> {
        if self.ok {
            Ok(())
        } else {
            let end = self.error.iter().position(|&b| b == 0).unwrap_or(256);
            Err(String::from_utf8_lossy(&self.error[..end]).into_owned())
        }
    }
}

#[repr(C)]
pub struct PluginMetadataC {
    pub id: [u8; 64],
    pub name: [u8; 128],
    pub version: [u8; 32],
    pub author: [u8; 128],
    pub description: [u8; 256],
}

impl PluginMetadataC {
    fn read_str(buf: &[u8]) -> String {
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        String::from_utf8_lossy(&buf[..end]).into_owned()
    }

    pub fn to_metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: Self::read_str(&self.id),
            name: Self::read_str(&self.name),
            version: Self::read_str(&self.version),
            author: Self::read_str(&self.author),
            description: Self::read_str(&self.description),
        }
    }
}

#[repr(C)]
pub struct IslandContentC {
    pub tag: u32,
    pub title: [u8; 256],
    pub artist: [u8; 256],
    pub cover_url: [u8; 512],
    pub is_playing: bool,
    pub message: [u8; 256],
    pub label: [u8; 128],
    pub value: [u8; 128],
}

impl IslandContentC {
    fn read_str(buf: &[u8]) -> String {
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        String::from_utf8_lossy(&buf[..end]).into_owned()
    }

    pub fn to_content(&self) -> Option<IslandContent> {
        match self.tag {
            1 => Some(IslandContent::Music {
                title: Self::read_str(&self.title),
                artist: Self::read_str(&self.artist),
                cover_url: {
                    let s = Self::read_str(&self.cover_url);
                    if s.is_empty() { None } else { Some(s) }
                },
                is_playing: self.is_playing,
            }),
            2 => Some(IslandContent::Notification {
                title: Self::read_str(&self.title),
                message: Self::read_str(&self.message),
                icon_url: {
                    let s = Self::read_str(&self.cover_url);
                    if s.is_empty() { None } else { Some(s) }
                },
            }),
            3 => Some(IslandContent::Status {
                label: Self::read_str(&self.label),
                value: Self::read_str(&self.value),
                icon: {
                    let s = Self::read_str(&self.cover_url);
                    if s.is_empty() { None } else { Some(s) }
                },
            }),
            _ => None,
        }
    }
}

#[repr(C)]
pub struct ThemeColorsC {
    pub primary: [u8; 4],
    pub secondary: [u8; 4],
    pub background: [u8; 4],
    pub text: [u8; 4],
    pub border: [u8; 4],
}

impl ThemeColorsC {
    pub fn to_colors(&self) -> ThemeColors {
        ThemeColors {
            primary: (
                self.primary[0],
                self.primary[1],
                self.primary[2],
                self.primary[3],
            ),
            secondary: (
                self.secondary[0],
                self.secondary[1],
                self.secondary[2],
                self.secondary[3],
            ),
            background: (
                self.background[0],
                self.background[1],
                self.background[2],
                self.background[3],
            ),
            text: (self.text[0], self.text[1], self.text[2], self.text[3]),
            border: (
                self.border[0],
                self.border[1],
                self.border[2],
                self.border[3],
            ),
        }
    }
}

#[repr(C)]
pub struct AnimationConfigC {
    pub expand_duration_ms: u32,
    pub collapse_duration_ms: u32,
    pub bounce_intensity: f32,
}

impl AnimationConfigC {
    pub fn to_config(&self) -> AnimationConfig {
        AnimationConfig {
            expand_duration_ms: self.expand_duration_ms,
            collapse_duration_ms: self.collapse_duration_ms,
            bounce_intensity: self.bounce_intensity,
        }
    }
}

// ---------------------------------------------------------------------------
// VTable — C ABI function pointer table
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct PluginVTable {
    pub on_load: unsafe extern "C" fn(PluginHandle) -> PluginResultC,
    pub on_unload: unsafe extern "C" fn(PluginHandle) -> PluginResultC,
    pub destroy: unsafe extern "C" fn(PluginHandle),
    pub get_content: Option<unsafe extern "C" fn(PluginHandle) -> IslandContentC>,
    pub on_click: Option<unsafe extern "C" fn(PluginHandle)>,
    pub on_expanded: Option<unsafe extern "C" fn(PluginHandle, bool)>,
    pub supports_expand: Option<unsafe extern "C" fn(PluginHandle) -> bool>,
    pub get_colors: Option<unsafe extern "C" fn(PluginHandle) -> ThemeColorsC>,
    pub get_animations: Option<unsafe extern "C" fn(PluginHandle) -> AnimationConfigC>,
}

#[repr(C)]
pub struct PluginInstanceC {
    pub handle: PluginHandle,
    pub metadata: PluginMetadataC,
    pub vtable: *const PluginVTable,
    pub plugin_type: u32,
}

impl PluginInstanceC {
    pub fn plugin_type_enum(&self) -> PluginType {
        match self.plugin_type {
            1 => PluginType::ContentProvider,
            2 => PluginType::ThemeProvider,
            3 => PluginType::ShortcutProvider,
            _ => PluginType::ContentProvider,
        }
    }
}

/// Expected DLL export signature
pub type PluginGetInstanceFn = unsafe extern "C" fn() -> PluginInstanceC;
