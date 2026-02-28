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
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            global_scale: 1.0,
            base_width: 120.0,
            base_height: 27.0,
            expanded_width: 280.0,
            expanded_height: 160.0,
            adaptive_border: false,
        }
    }
}
