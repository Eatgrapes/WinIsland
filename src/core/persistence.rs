use crate::core::config::AppConfig;
use std::fs;
use std::path::PathBuf;
pub fn get_config_path() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".winisland");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path.push("config.toml");
    path
}
pub fn load_config() -> AppConfig {
    let path = get_config_path();
    if let Ok(content) = fs::read_to_string(path)
        && let Ok(config) = toml::from_str(&content) {
            return config;
        }
    let default = AppConfig::default();
    save_config(&default);
    default
}
pub fn save_config(config: &AppConfig) {
    let path = get_config_path();
    if let Ok(content) = toml::to_string_pretty(config) {
        let _ = fs::write(path, content);
    }
}
