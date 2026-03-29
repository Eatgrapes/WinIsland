use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;
use windows::Win32::Globalization::GetUserDefaultLocaleName;

pub struct I18n {
    pub current_lang: String,
    translations: HashMap<String, String>,
}

static I18N: Lazy<Arc<RwLock<I18n>>> = Lazy::new(|| {
    let mut i18n = I18n {
        current_lang: "en".to_string(),
        translations: HashMap::new(),
    };
    i18n.load("en");
    Arc::new(RwLock::new(i18n))
});

fn lang_dir() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("resources")
        .join("in_app")
        .join("lang")
}

fn read_lang_file(lang: &str) -> Option<String> {
    let path = lang_dir().join(format!("{}.lang", lang));
    std::fs::read_to_string(path).ok()
}

const FALLBACK_EN: &str = include_str!("../../resources/in_app/lang/en.lang");
const FALLBACK_ZH: &str = include_str!("../../resources/in_app/lang/zh.lang");

impl I18n {
    pub fn load(&mut self, lang: &str) {
        let content = read_lang_file(lang).unwrap_or_else(|| {
            match lang {
                "zh" => FALLBACK_ZH.to_string(),
                _ => FALLBACK_EN.to_string(),
            }
        });
        self.current_lang = lang.to_string();
        self.translations.clear();
        for line in content.lines() {
            if let Some((k, v)) = line.split_once('=') {
                self.translations.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }

    pub fn get(&self, key: &str) -> String {
        self.translations.get(key).cloned().unwrap_or_else(|| key.to_string())
    }
}

pub fn init_i18n(config_lang: &str) {
    let mut target_lang = config_lang.to_string();
    if target_lang == "auto" {
        target_lang = get_system_lang();
    }
    I18N.write().unwrap().load(&target_lang);
}

pub fn set_lang(lang: &str) {
    I18N.write().unwrap().load(lang);
}

pub fn current_lang() -> String {
    I18N.read().unwrap().current_lang.clone()
}

pub fn tr(key: &str) -> String {
    I18N.read().unwrap().get(key)
}

fn get_system_lang() -> String {
    let mut buffer = [0u16; 128];
    unsafe {
        let len = GetUserDefaultLocaleName(&mut buffer);
        if len > 0 {
            let s = String::from_utf16_lossy(&buffer[..len as usize - 1]);
            if s.starts_with("zh") {
                return "zh".to_string();
            }
        }
    }
    "en".to_string()
}
