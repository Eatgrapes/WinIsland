#![allow(dead_code)]

use serde::{Deserialize, Serialize};

pub use winisland_plugin_api::{
    AnimationConfigC, ContextDataC, ContextIdC, HostApiC, HostStateC, MediaSourceC, PRIORITY_HIGH,
    PRIORITY_LOW, PRIORITY_MEDIUM, PluginGetInstanceFn, PluginHandle, PluginInstanceC,
    PluginMetadataC, PluginResultC, PluginType, PluginVTable, ShortcutC, ThemeColorsC,
    TranslationPairC,
};

pub fn read_c_str(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

pub fn read_opt_c_str(buf: &[u8]) -> Option<String> {
    let s = read_c_str(buf);
    if s.is_empty() { None } else { Some(s) }
}

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
            secondary: (
                c.secondary[0],
                c.secondary[1],
                c.secondary[2],
                c.secondary[3],
            ),
            background: (
                c.background[0],
                c.background[1],
                c.background[2],
                c.background[3],
            ),
            text: (c.text[0], c.text[1], c.text[2], c.text[3]),
            border: (c.border[0], c.border[1], c.border[2], c.border[3]),
        }
    }
}

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

#[derive(Debug, Clone)]
pub struct Shortcut {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
    pub hotkey: Option<String>,
}

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
// Context types (push-based, Host side)
// ---------------------------------------------------------------------------

/// Host state snapshot returned by [`HostApiC::query_host_state`].
#[derive(Debug, Clone, Default)]
pub struct HostState {
    pub media_title: String,
    pub media_artist: String,
    pub is_playing: bool,
    pub theme: String,
}

impl From<&HostStateC> for HostState {
    fn from(c: &HostStateC) -> Self {
        Self {
            media_title: read_c_str(&c.media_title),
            media_artist: read_c_str(&c.media_artist),
            is_playing: c.is_playing,
            theme: read_c_str(&c.theme),
        }
    }
}

impl From<&HostState> for HostStateC {
    fn from(s: &HostState) -> Self {
        fn fill<const N: usize>(buf: &mut [u8; N], val: &str) {
            let len = val.len().min(N - 1);
            buf[..len].copy_from_slice(&val.as_bytes()[..len]);
        }
        let mut c = HostStateC {
            media_title: [0u8; 256],
            media_artist: [0u8; 256],
            is_playing: s.is_playing,
            theme: [0u8; 32],
        };
        fill(&mut c.media_title, &s.media_title);
        fill(&mut c.media_artist, &s.media_artist);
        fill(&mut c.theme, &s.theme);
        c
    }
}

/// Convert a C ABI [`ContextDataC`] into a [`HostState`] (host-side context data).
///
/// Note: The priority field is mapped directly from the C-level constant.
impl From<&ContextDataC> for crate::core::context::PluginContext {
    fn from(c: &ContextDataC) -> Self {
        let priority = match c.priority {
            PRIORITY_LOW => crate::core::context::Priority::Low,
            PRIORITY_MEDIUM => crate::core::context::Priority::Medium,
            PRIORITY_HIGH => crate::core::context::Priority::High,
            _ => crate::core::context::Priority::Medium,
        };
        Self {
            id: crate::core::context::ContextId {
                source: String::new(), // filled in by the caller
                uuid: String::new(),   // filled in by ContextManager::push_context
            },
            priority,
            title: read_c_str(&c.title),
            body: read_c_str(&c.body),
            icon: Vec::new(), // filled from plugin metadata later
            duration_sec: c.duration_sec,
            mini_render: c.mini_render,
            mini_text: read_c_str(&c.mini_text),
            created_at: std::time::Instant::now(),
            expanded_started_at: None,
            collapsed_at: None,
            mini_timeout_start: None,
        }
    }
}
