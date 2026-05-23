use std::ffi::c_char;

pub type PluginHandle = *mut std::ffi::c_void;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginType {
    Content = 1,
    Theme = 2,
    Shortcut = 3,
}

impl PluginType {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            1 => Some(Self::Content),
            2 => Some(Self::Theme),
            3 => Some(Self::Shortcut),
            _ => None,
        }
    }
}

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

    pub fn into_result(self) -> Result<(), String> {
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

pub const ISLAND_CONTENT_TAG_MUSIC: u32 = 1;
pub const ISLAND_CONTENT_TAG_NOTIFICATION: u32 = 2;
pub const ISLAND_CONTENT_TAG_STATUS: u32 = 3;

#[repr(C)]
pub struct ThemeColorsC {
    pub primary: [u8; 4],
    pub secondary: [u8; 4],
    pub background: [u8; 4],
    pub text: [u8; 4],
    pub border: [u8; 4],
}

#[repr(C)]
pub struct AnimationConfigC {
    pub expand_duration_ms: u32,
    pub collapse_duration_ms: u32,
    pub bounce_intensity: f32,
}

#[repr(C)]
pub struct ShortcutC {
    pub id: [u8; 64],
    pub name: [u8; 128],
    pub description: [u8; 256],
    pub icon: [u8; 256],
    pub hotkey: [u8; 32],
}

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
    pub get_shortcuts_count: Option<unsafe extern "C" fn(PluginHandle) -> u32>,
    pub get_shortcut_at: Option<unsafe extern "C" fn(PluginHandle, i: u32, out: *mut ShortcutC)>,
    pub execute_shortcut:
        Option<unsafe extern "C" fn(PluginHandle, id: *const c_char) -> PluginResultC>,
}

#[repr(C)]
pub struct PluginInstanceC {
    pub handle: PluginHandle,
    pub metadata: PluginMetadataC,
    pub vtable: *const PluginVTable,
    pub plugin_type: u32,
}

pub type PluginGetInstanceFn = unsafe extern "C" fn() -> PluginInstanceC;
