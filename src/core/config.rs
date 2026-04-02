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
    #[serde(default = "default_smtc_known_apps")]
    pub smtc_known_apps: Vec<String>,
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
    #[serde(default = "default_lyrics_delay")]
    pub lyrics_delay: f64,
    #[serde(default = "default_position_x_offset")]
    pub position_x_offset: i32,
    #[serde(default = "default_position_y_offset")]
    pub position_y_offset: i32,
    #[serde(default = "default_notification_enabled")]
    pub notification_enabled: bool,
    #[serde(default = "default_notification_show_app_name")]
    pub notification_show_app_name: bool,
    #[serde(default = "default_notification_duration")]
    pub notification_duration: f32,
    #[serde(default = "default_notification_excluded_apps")]
    pub notification_excluded_apps: Vec<String>,
    #[serde(default = "default_notification_show_overview")]
    pub notification_show_overview: bool,
}

fn default_notification_enabled() -> bool {
    false
}

fn default_notification_show_app_name() -> bool {
    true
}

fn default_notification_duration() -> f32 {
    5.0
}

fn default_notification_excluded_apps() -> Vec<String> {
    Vec::new()
}

fn default_notification_show_overview() -> bool {
    false
}

fn default_show_lyrics() -> bool {
    true
}

fn default_smtc_known_apps() -> Vec<String> {
    Vec::new()
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

fn default_lyrics_delay() -> f64 {
    0.0
}

fn default_position_x_offset() -> i32 {
    0
}

fn default_position_y_offset() -> i32 {
    0
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
            smtc_known_apps: Vec::new(),
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
            lyrics_delay: 0.0,
            position_x_offset: 0,
            position_y_offset: 0,
            notification_enabled: false,
            notification_show_app_name: true,
            notification_duration: 5.0,
            notification_excluded_apps: Vec::new(),
            notification_show_overview: false,
        }
    }
}
