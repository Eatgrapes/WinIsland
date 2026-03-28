use serde::{Deserialize, Serialize};
pub const APP_VERSION: &str = "1.0.0";
pub const APP_AUTHOR: &str = "Eatgrapes";
pub const APP_HOMEPAGE: &str = "https://github.com/Eatgrapes/WinIsland";
pub const WINDOW_TITLE: &str = "WinIsland";
pub const TOP_OFFSET: i32 = 10;
pub const PADDING: f32 = 80.0;
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AppConfig {
    pub global_scale: f32,
    pub base_width: f32,
    pub base_height: f32,
    pub expanded_width: f32,
    pub expanded_height: f32,
    pub adaptive_border: bool,
    pub motion_blur: bool,
    pub smtc_enabled: bool,
    pub smtc_apps: Vec<String>,
    #[serde(default = "default_show_lyrics")]
    pub show_lyrics: bool,
    #[serde(default = "default_custom_font")]
    pub custom_font_path: Option<String>,
    #[serde(default = "default_auto_start")]
    pub auto_start: bool,
    #[serde(default = "default_auto_hide")]
    pub auto_hide: bool,
    #[serde(default = "default_auto_hide_delay")]
    pub auto_hide_delay: f32,
    #[serde(default = "default_check_for_updates")]
    pub check_for_updates: bool,
    #[serde(default = "default_update_check_interval")]
    pub update_check_interval: f32,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_lyrics_source")]
    pub lyrics_source: String,
    #[serde(default = "default_lyrics_fallback")]
    pub lyrics_fallback: bool,
}

fn default_show_lyrics() -> bool {
    true
}

fn default_custom_font() -> Option<String> {
    None
}

fn default_auto_start() -> bool {
    false
}

fn default_auto_hide() -> bool {
    false
}

fn default_auto_hide_delay() -> f32 {
    5.0
}

fn default_check_for_updates() -> bool {
    true
}

fn default_update_check_interval() -> f32 {
    4.0
}

fn default_language() -> String {
    "auto".to_string()
}

fn default_lyrics_source() -> String {
    "163".to_string()
}

fn default_lyrics_fallback() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            global_scale: 1.0,
            base_width: 120.0,
            base_height: 27.0,
            expanded_width: 360.0,
            expanded_height: 190.0,
            adaptive_border: false,
            motion_blur: true,
            smtc_enabled: true,
            smtc_apps: Vec::new(),
            show_lyrics: true,
            custom_font_path: None,
            auto_start: false,
            auto_hide: false,
            auto_hide_delay: 5.0,
            check_for_updates: true,
            update_check_interval: 4.0,
            language: "auto".to_string(),
            lyrics_source: "163".to_string(),
            lyrics_fallback: true,
        }
    }
}
