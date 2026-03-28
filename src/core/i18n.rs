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

const LANG_EN: &str = r#"
tab_general=General
tab_about=About
global_scale=Global Scale
base_width=Base Width
base_height=Base Height
expanded_width=Expanded Width
expanded_height=Expanded Height
adaptive_border=Adaptive Border
motion_blur=Motion Blur
custom_font=Custom Font
font_select=Select
font_reset=Reset
start_boot=Start at Boot
auto_hide=Auto Hide
check_updates=Check for Updates
update_interval=Update Interval (h)
language=Language
lang_name=English
hide_delay=Hide Delay (s)
reset_defaults=Reset to Defaults
visit_homepage=Visit Project Homepage
created_by=Created by
music_settings_title=Music Settings
smtc_control=SMTC Control
show_lyrics=Show Lyrics
lyrics_source=Lyrics Source
lyrics_fallback=Fallback Source
media_apps=MEDIA APPLICATIONS
scan_apps=Scan Apps
no_sessions=No sessions detected
delete=Delete
update_available_title=Update Available
update_available_desc=A new version of WinIsland is available (Released: {}). Would you like to update now?
update_failed_title=Update Failed
update_failed_dl=Failed to download the new version.
update_failed_save=Failed to save the new version.
"#;

const LANG_ZH: &str = r#"
tab_general=常规设置
tab_about=关于
global_scale=全局缩放
base_width=基础宽度
base_height=基础高度
expanded_width=展开宽度
expanded_height=展开高度
adaptive_border=自适应边框
motion_blur=动态模糊
custom_font=自定义字体
font_select=选择
font_reset=重置
start_boot=开机启动
auto_hide=自动隐藏
check_updates=检查更新
update_interval=检查更新间隔 (h)
language=语言
lang_name=中文
hide_delay=隐藏延迟 (s)
reset_defaults=恢复默认设置
visit_homepage=访问项目主页
created_by=作者
music_settings_title=音乐设置
smtc_control=SMTC 控制
show_lyrics=显示歌词
lyrics_source=歌词来源
lyrics_fallback=备选来源
media_apps=媒体应用程序
scan_apps=扫描应用
no_sessions=未检测到运行中的媒体
delete=删除
update_available_title=发现新版本
update_available_desc=WinIsland 有新版本可用 (发布时间: {})。是否现在更新？
update_failed_title=更新失败
update_failed_dl=无法下载新版本。
update_failed_save=无法保存新版本文件。
"#;

impl I18n {
    pub fn load(&mut self, lang: &str) {
        let content = match lang {
            "zh" => LANG_ZH,
            _ => LANG_EN,
        };
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
