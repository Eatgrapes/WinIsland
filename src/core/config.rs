use serde::{Deserialize, Serialize};
pub const APP_VERSION: &str = "1.0.0";
pub const APP_AUTHOR: &str = "Eatgrapes";
pub const APP_HOMEPAGE: &str = "https://github.com/Eatgrapes/WinIsland";
pub const WINDOW_TITLE: &str = "WinIsland";
pub const TOP_OFFSET: i32 = 10;
pub const PADDING: f32 = 80.0;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(from = "String", into = "String")]
#[derive(Default)]
pub enum DockPosition {
    #[default]
    TopCenter,
    TopLeft,
    TopRight,
    BottomCenter,
    BottomLeft,
    BottomRight,
}

impl DockPosition {
    pub fn is_bottom(&self) -> bool {
        matches!(
            self,
            Self::BottomCenter | Self::BottomLeft | Self::BottomRight
        )
    }

    pub fn is_left(&self) -> bool {
        matches!(self, Self::TopLeft | Self::BottomLeft)
    }

    pub fn is_right(&self) -> bool {
        matches!(self, Self::TopRight | Self::BottomRight)
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::TopCenter => "top_center",
            Self::TopLeft => "top_left",
            Self::TopRight => "top_right",
            Self::BottomCenter => "bottom_center",
            Self::BottomLeft => "bottom_left",
            Self::BottomRight => "bottom_right",
        }
    }
}


impl std::fmt::Display for DockPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for DockPosition {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "top_center" => Ok(Self::TopCenter),
            "top_left" => Ok(Self::TopLeft),
            "top_right" => Ok(Self::TopRight),
            "bottom_center" => Ok(Self::BottomCenter),
            "bottom_left" => Ok(Self::BottomLeft),
            "bottom_right" => Ok(Self::BottomRight),
            _ => Err(()),
        }
    }
}

impl From<String> for DockPosition {
    fn from(value: String) -> Self {
        value.parse().unwrap_or_default()
    }
}

impl From<DockPosition> for String {
    fn from(value: DockPosition) -> Self {
        value.as_str().to_string()
    }
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AppConfig {
    pub global_scale: f32,
    pub base_width: f32,
    pub base_height: f32,
    pub expanded_width: f32,
    pub expanded_height: f32,
    pub adaptive_border: bool,
    pub motion_blur: bool,
    #[serde(default = "default_island_style")]
    pub island_style: String,
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
    #[serde(default = "default_lyrics_scroll")]
    pub lyrics_scroll: bool,
    #[serde(default = "default_lyrics_scroll_max_width")]
    pub lyrics_scroll_max_width: f32,
    #[serde(default = "default_position_x_offset")]
    pub position_x_offset: i32,
    #[serde(default = "default_position_y_offset")]
    pub position_y_offset: i32,
    #[serde(default = "default_dock_position")]
    pub dock_position: DockPosition,
    #[serde(default = "default_monitor_index")]
    pub monitor_index: i32,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
}

fn default_island_style() -> String {
    "default".to_string()
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

fn default_lyrics_scroll() -> bool {
    false
}

fn default_lyrics_scroll_max_width() -> f32 {
    300.0
}

fn default_position_x_offset() -> i32 {
    0
}

fn default_position_y_offset() -> i32 {
    0
}

fn default_dock_position() -> DockPosition {
    DockPosition::TopCenter
}

fn default_monitor_index() -> i32 {
    0
}

fn default_font_size() -> f32 {
    0.0
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            global_scale: 1.0,
            base_width: 120.0,
            base_height: 27.0,
            expanded_width: 360.0,
            expanded_height: 200.0,
            adaptive_border: false,
            motion_blur: true,
            island_style: "default".to_string(),
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
            lyrics_scroll: false,
            lyrics_scroll_max_width: 300.0,
            position_x_offset: 0,
            position_y_offset: 0,
            dock_position: DockPosition::TopCenter,
            monitor_index: 0,
            font_size: 0.0,
        }
    }
}
