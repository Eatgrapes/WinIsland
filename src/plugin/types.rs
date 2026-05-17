#![allow(dead_code)]

use serde::{Deserialize, Serialize};

pub use winisland_plugin_api::{
    AnimationConfigC, IslandContentC, PluginGetInstanceFn, PluginHandle, PluginInstanceC,
    PluginMetadataC, PluginResultC, PluginVTable, PluginType, ThemeColorsC,
    ISLAND_CONTENT_TAG_MUSIC, ISLAND_CONTENT_TAG_NOTIFICATION, ISLAND_CONTENT_TAG_STATUS,
};

fn read_c_str(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

fn read_opt_c_str(buf: &[u8]) -> Option<String> {
    let s = read_c_str(buf);
    if s.is_empty() { None } else { Some(s) }
}

/// 插件元信息（Host 端）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
}

impl From<&PluginMetadataC> for PluginMetadata {
    fn from(c: &PluginMetadataC) -> Self {
        Self {
            id: read_c_str(&c.id),
            name: read_c_str(&c.name),
            version: read_c_str(&c.version),
            author: read_c_str(&c.author),
            description: read_c_str(&c.description),
        }
    }
}

/// 岛屿内容枚举（Host 端）
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

impl From<&IslandContentC> for IslandContent {
    fn from(c: &IslandContentC) -> Self {
        match c.tag {
            ISLAND_CONTENT_TAG_MUSIC => IslandContent::Music {
                title: read_c_str(&c.title),
                artist: read_c_str(&c.artist),
                cover_url: read_opt_c_str(&c.cover_url),
                is_playing: c.is_playing,
            },
            ISLAND_CONTENT_TAG_NOTIFICATION => IslandContent::Notification {
                title: read_c_str(&c.title),
                message: read_c_str(&c.message),
                icon_url: read_opt_c_str(&c.cover_url),
            },
            ISLAND_CONTENT_TAG_STATUS => IslandContent::Status {
                label: read_c_str(&c.label),
                value: read_c_str(&c.value),
                icon: read_opt_c_str(&c.cover_url),
            },
            _ => IslandContent::Status {
                label: String::new(),
                value: String::new(),
                icon: None,
            },
        }
    }
}

/// 主题颜色（Host 端）
#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub primary: (u8, u8, u8, u8),
    pub secondary: (u8, u8, u8, u8),
    pub background: (u8, u8, u8, u8),
    pub text: (u8, u8, u8, u8),
    pub border: (u8, u8, u8, u8),
}

impl From<&ThemeColorsC> for ThemeColors {
    fn from(c: &ThemeColorsC) -> Self {
        Self {
            primary: (c.primary[0], c.primary[1], c.primary[2], c.primary[3]),
            secondary: (c.secondary[0], c.secondary[1], c.secondary[2], c.secondary[3]),
            background: (c.background[0], c.background[1], c.background[2], c.background[3]),
            text: (c.text[0], c.text[1], c.text[2], c.text[3]),
            border: (c.border[0], c.border[1], c.border[2], c.border[3]),
        }
    }
}

/// 动画配置（Host 端）
#[derive(Debug, Clone)]
pub struct AnimationConfig {
    pub expand_duration_ms: u32,
    pub collapse_duration_ms: u32,
    pub bounce_intensity: f32,
}

impl From<&AnimationConfigC> for AnimationConfig {
    fn from(c: &AnimationConfigC) -> Self {
        Self {
            expand_duration_ms: c.expand_duration_ms,
            collapse_duration_ms: c.collapse_duration_ms,
            bounce_intensity: c.bounce_intensity,
        }
    }
}

/// 快捷方式定义（Host 端）
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
// Host-side Plugin traits
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
