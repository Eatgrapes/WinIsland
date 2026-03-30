use crate::core::config::WINDOW_TITLE;
use tray_icon::menu::{Menu, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub struct TrayManager {
    tray: TrayIcon,
    toggle_item: MenuItem,
    settings_item: MenuItem,
    music_item: MenuItem,
    notification_item: MenuItem,
    quit_item: MenuItem,
    is_light: bool,
}

impl TrayManager {
    pub fn new(is_light: bool) -> Self {
        let menu = Menu::new();
        let toggle_item = MenuItem::new("Hide", true, None);
        let settings_item = MenuItem::new("Settings", true, None);
        let music_item = MenuItem::new("Music Settings", true, None);
        let notification_item = MenuItem::new("Notification Settings", true, None);
        let quit_item = MenuItem::new("Exit", true, None);
        let _ = menu.append(&toggle_item);
        let _ = menu.append(&settings_item);
        let _ = menu.append(&music_item);
        let _ = menu.append(&notification_item);
        let _ = menu.append(&quit_item);
        
        let tray = TrayIconBuilder::new()
            .with_tooltip(WINDOW_TITLE)
            .with_menu(Box::new(menu))
            .with_icon(Self::load_tray_icon(is_light))
            .build()
            .unwrap();
        Self {
            tray,
            toggle_item,
            settings_item,
            music_item,
            notification_item,
            quit_item,
            is_light,
        }
    }

    pub fn update_theme(&mut self, is_light: bool) {
        if self.is_light != is_light {
            self.is_light = is_light;
            let _ = self.tray.set_icon(Some(Self::load_tray_icon(is_light)));
        }
    }

    pub fn update_item_text(&self, visible: bool) {
        if visible {
            self.toggle_item.set_text("Hide");
        } else {
            self.toggle_item.set_text("Show");
        }
    }

    fn load_tray_icon(is_light: bool) -> Icon {
        let icon_bytes: &[u8] = if is_light {
            include_bytes!("../../resources/icon-dark.png")
        } else {
            include_bytes!("../../resources/icon.png")
        };
        let image = image::load_from_memory(icon_bytes).expect("Failed to load icon from resources");
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();
        let rgba_vec = rgba.into_raw();
        Icon::from_rgba(rgba_vec, width, height).expect("Failed to create tray icon from RGBA data")
    }
}
impl TrayAction {
    pub fn from_id(id: tray_icon::menu::MenuId, tray: &TrayManager) -> Option<Self> {
        if id == tray.toggle_item.id() {
            Some(TrayAction::ToggleVisibility)
        } else if id == tray.settings_item.id() {
            Some(TrayAction::OpenSettings)
        } else if id == tray.music_item.id() {
            Some(TrayAction::OpenMusicSettings)
        } else if id == tray.notification_item.id() {
            Some(TrayAction::OpenNotificationSettings)
        } else if id == tray.quit_item.id() {
            Some(TrayAction::Exit)
        } else {
            None
        }
    }
}
pub enum TrayAction {
    ToggleVisibility,
    OpenSettings,
    OpenMusicSettings,
    OpenNotificationSettings,
    Exit,
}

