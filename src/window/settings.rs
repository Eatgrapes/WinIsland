use crate::core::config::{APP_AUTHOR, APP_HOMEPAGE, APP_VERSION, AppConfig, DockPosition};
use crate::core::i18n::{current_lang, init_i18n, set_lang, tr};
use crate::core::persistence::save_config;
use crate::utils::anim::AnimPool;
use crate::utils::autostart::set_autostart;
use crate::utils::color::*;
use crate::utils::font::{DrawTextCachedParams, FontManager};
use crate::utils::icon::get_app_icon;
use crate::utils::settings_ui::items::*;
use crate::utils::settings_ui::*;
use skia_safe::{Color, Paint, Rect, surfaces};
use softbuffer::{Context, Surface};
use std::sync::Arc;
use std::time::{Duration, Instant};
use windows::Win32::System::Threading::{MUTEX_ALL_ACCESS, OpenMutexW};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SW_RESTORE, SetForegroundWindow, ShowWindow,
};
use windows::core::w;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowButtons, WindowId};

const WIN_W: f32 = 666.0;
const WIN_H: f32 = 666.0;
const SIDEBAR_W: f32 = 180.0;
const SIDEBAR_ROW_H: f32 = 32.0;
const CONTENT_START_Y: f32 = 10.0;
const SUB_TAB_H: f32 = 40.0;
const SUB_TAB_START_Y: f32 = 50.0;

const POPUP_OPACITY_KEY: u64 = 1;
const SIDEBAR_KEY_BASE: u64 = 1_000;
const SCROLL_STIFFNESS: f32 = 55.0;
const SCROLL_DAMPING: f32 = 16.0;

#[derive(Clone, PartialEq)]
enum PopupKind {
    LyricsSource,
    Language,
    Monitor,
    IslandStyle,
    DockPositionPopup,
    SettingsTheme,
    MiniCoverShape,
    ExpandedCoverShape,
}

struct PopupState {
    kind: PopupKind,
    #[allow(dead_code)]
    button_rect: Rect,
    menu_rect: Rect,
    options: Vec<String>,
    values: Vec<String>,
    selected_idx: usize,
    hover_idx: Option<usize>,
}

impl PopupState {
    fn new(
        kind: PopupKind,
        button_rect: Rect,
        options: Vec<String>,
        values: Vec<String>,
        selected_idx: usize,
        _win_w: f32,
        win_h: f32,
    ) -> Self {
        let item_count = options.len() as f32;
        let menu_h = POPUP_MENU_PAD * 2.0 + item_count * POPUP_ITEM_H;

        let fm = FontManager::global();
        let mut max_text_w: f32 = button_rect.width();
        for opt in &options {
            let w = fm.measure_text_cached(opt, 12.0, skia_safe::FontStyle::normal());
            let needed = w + 36.0;
            if needed > max_text_w {
                max_text_w = needed;
            }
        }

        let menu_w = max_text_w;
        let right_edge = button_rect.right;

        let menu_x;
        let menu_y;

        let fits_below = button_rect.bottom + 2.0 + menu_h <= win_h;
        let fits_right = right_edge - menu_w >= 0.0;

        if fits_below {
            menu_y = button_rect.bottom + 2.0;
        } else {
            menu_y = (button_rect.top - menu_h - 2.0).max(0.0);
        }

        if fits_right {
            menu_x = right_edge - menu_w;
        } else {
            menu_x = button_rect.left;
        }

        let menu_rect = Rect::from_xywh(menu_x, menu_y, menu_w, menu_h);

        Self {
            kind,
            button_rect,
            menu_rect,
            options,
            values,
            selected_idx,
            hover_idx: None,
        }
    }

    fn menu_rect(&self) -> Rect {
        self.menu_rect
    }

    fn item_rect(&self, idx: usize) -> Rect {
        let menu = self.menu_rect;
        Rect::from_xywh(
            menu.left + POPUP_MENU_PAD,
            menu.top + POPUP_MENU_PAD + idx as f32 * POPUP_ITEM_H,
            menu.width() - POPUP_MENU_PAD * 2.0,
            POPUP_ITEM_H,
        )
    }

    fn hit_test_item(&self, mx: f32, my: f32) -> Option<usize> {
        let menu = self.menu_rect;
        if mx < menu.left || mx > menu.right || my < menu.top || my > menu.bottom {
            return None;
        }
        let inner_top = menu.top + POPUP_MENU_PAD;
        let inner_bottom = menu.bottom - POPUP_MENU_PAD;
        if my < inner_top || my > inner_bottom {
            return None;
        }
        let rel_y = my - inner_top;
        let idx = (rel_y / POPUP_ITEM_H).floor() as i32;
        if idx < 0 {
            return None;
        }
        let idx = idx as usize;
        if idx >= self.options.len() {
            return None;
        }
        Some(idx)
    }
}

pub struct SettingsApp {
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    config: AppConfig,
    active_page: usize,
    active_sub_page: usize,
    sub_tab_hover: i32,
    switch_anim: SwitchAnimator,
    anim: AnimPool,
    logical_mouse_pos: (f32, f32),
    last_hover_mouse_pos: (f32, f32),
    frame_count: u64,
    scroll_y: f32,
    target_scroll_y: f32,
    scroll_vel_y: f32,
    last_frame_time: Instant,
    detected_apps: Vec<String>,
    sidebar_hover: i32,
    popup: Option<PopupState>,
    hover_row: Option<usize>,
    total_rows: usize,
    is_light: bool,
    cached_items: Vec<SettingsItem>,
    items_dirty: bool,
    cached_content_height: f32,
    cached_max_scroll: f32,
    cached_row_tops: Vec<f32>,
    win_w: f32,
    win_h: f32,
    max_scroll: f32,
}

impl SettingsApp {
    pub fn new(config: AppConfig) -> Self {
        let switch_anim = SwitchAnimator::new(&[
            config.adaptive_border,
            config.motion_blur,
            config.cover_rotate,
            config.audio_gate,
            config.mini_controls,
            config.auto_start,
            config.auto_hide,
            config.check_for_updates,
            config.smtc_enabled,
            config.show_lyrics,
            config.lyrics_fallback,
            config.lyrics_scroll,
        ]);
        Self {
            window: None,
            surface: None,
            config,
            active_page: 0,
            active_sub_page: 0,
            sub_tab_hover: -1,
            switch_anim,
            anim: AnimPool::new(),
            logical_mouse_pos: (0.0, 0.0),
            last_hover_mouse_pos: (-1.0, -1.0),
            frame_count: 0,
            scroll_y: 0.0,
            target_scroll_y: 0.0,
            scroll_vel_y: 0.0,
            last_frame_time: Instant::now(),
            detected_apps: Vec::new(),
            sidebar_hover: -1,
            popup: None,
            hover_row: None,
            total_rows: 0,
            is_light: false,
            cached_items: Vec::new(),
            items_dirty: true,
            cached_content_height: 0.0,
            cached_max_scroll: 0.0,
            cached_row_tops: Vec::new(),
            win_w: WIN_W,
            win_h: WIN_H,
            max_scroll: 0.0,
        }
    }

    fn theme(&self) -> SettingsTheme {
        if self.is_light {
            light_settings_theme()
        } else {
            dark_settings_theme()
        }
    }

    fn update_theme(&mut self) {
        self.is_light = match self.config.settings_theme.as_str() {
            "light" => true,
            "dark" => false,
            _ => {
                if let Some(win) = &self.window {
                    win.theme() == Some(winit::window::Theme::Light)
                } else {
                    false
                }
            }
        };
        if let Some(win) = &self.window {
            win.request_redraw();
        }
    }

    fn build_general_items(&self) -> Vec<SettingsItem> {
        let mut items: Vec<SettingsItem> = vec![];

        match self.active_sub_page {
            0 => {
                items.push(SettingsItem::SectionHeader {
                    label: tr("section_appearance"),
                });
                items.push(SettingsItem::GroupStart);
                items.push(SettingsItem::RowStepper {
                    label: tr("global_scale"),
                    value: format!("{:.2}", self.config.global_scale),
                    enabled: true,
                });
                items.push(SettingsItem::RowStepper {
                    label: tr("base_width"),
                    value: self.config.base_width.to_string(),
                    enabled: true,
                });
                items.push(SettingsItem::RowStepper {
                    label: tr("base_height"),
                    value: self.config.base_height.to_string(),
                    enabled: true,
                });
                items.push(SettingsItem::RowStepper {
                    label: tr("expanded_width"),
                    value: self.config.expanded_width.to_string(),
                    enabled: true,
                });
                items.push(SettingsItem::RowStepper {
                    label: tr("expanded_height"),
                    value: self.config.expanded_height.to_string(),
                    enabled: true,
                });
                items.push(SettingsItem::RowStepper {
                    label: tr("position_x_offset"),
                    value: self.config.position_x_offset.to_string(),
                    enabled: true,
                });
                items.push(SettingsItem::RowStepper {
                    label: tr("position_y_offset"),
                    value: self.config.position_y_offset.to_string(),
                    enabled: true,
                });
                items.push(SettingsItem::RowStepper {
                    label: tr("font_size"),
                    value: format!("{:.0}", self.config.font_size),
                    enabled: true,
                });
                {
                    let monitors = self.get_monitor_list();
                    let selected_idx =
                        (self.config.monitor_index as usize).min(monitors.len().saturating_sub(1));
                    let options: Vec<(String, bool)> = monitors
                        .iter()
                        .enumerate()
                        .map(|(i, name)| (name.clone(), i == selected_idx))
                        .collect();
                    items.push(SettingsItem::RowSourceSelect {
                        label: tr("monitor"),
                        options,
                        enabled: true,
                    });
                }
                {
                    let dp = self.config.dock_position;
                    items.push(SettingsItem::RowSourceSelect {
                        label: tr("dock_position"),
                        options: vec![
                            (
                                tr("dock_position_top_center"),
                                dp == DockPosition::TopCenter,
                            ),
                            (tr("dock_position_top_left"), dp == DockPosition::TopLeft),
                            (tr("dock_position_top_right"), dp == DockPosition::TopRight),
                            (
                                tr("dock_position_bottom_center"),
                                dp == DockPosition::BottomCenter,
                            ),
                            (
                                tr("dock_position_bottom_left"),
                                dp == DockPosition::BottomLeft,
                            ),
                            (
                                tr("dock_position_bottom_right"),
                                dp == DockPosition::BottomRight,
                            ),
                        ],
                        enabled: true,
                    });
                }
                items.push(SettingsItem::GroupEnd);
            }
            1 => {
                items.push(SettingsItem::SectionHeader {
                    label: tr("section_effects"),
                });
                items.push(SettingsItem::GroupStart);
                items.push(SettingsItem::RowSourceSelect {
                    label: tr("settings_theme"),
                    options: vec![
                        (tr("theme_system"), self.config.settings_theme == "system"),
                        (tr("theme_light"), self.config.settings_theme == "light"),
                        (tr("theme_dark"), self.config.settings_theme == "dark"),
                    ],
                    enabled: true,
                });
                items.push(SettingsItem::RowSourceSelect {
                    label: tr("mini_cover_shape"),
                    options: vec![
                        (tr("shape_square"), self.config.mini_cover_shape == "square"),
                        (tr("shape_circle"), self.config.mini_cover_shape == "circle"),
                    ],
                    enabled: true,
                });
                items.push(SettingsItem::RowSourceSelect {
                    label: tr("expanded_cover_shape"),
                    options: vec![
                        (
                            tr("shape_square"),
                            self.config.expanded_cover_shape == "square",
                        ),
                        (
                            tr("shape_circle"),
                            self.config.expanded_cover_shape == "circle",
                        ),
                    ],
                    enabled: true,
                });
                items.push(SettingsItem::RowSwitch {
                    label: tr("adaptive_border"),
                    on: self.config.adaptive_border,
                    enabled: true,
                });
                items.push(SettingsItem::RowSwitch {
                    label: tr("motion_blur"),
                    on: self.config.motion_blur,
                    enabled: true,
                });
                items.push(SettingsItem::RowSwitch {
                    label: tr("cover_rotate"),
                    on: self.config.cover_rotate,
                    enabled: true,
                });
                items.push(SettingsItem::RowSwitch {
                    label: tr("audio_gate"),
                    on: self.config.audio_gate,
                    enabled: true,
                });
                items.push(SettingsItem::GroupEnd);
                items.push(SettingsItem::SectionHeader {
                    label: tr("section_experimental"),
                });
                items.push(SettingsItem::GroupStart);
                items.push(SettingsItem::RowSwitch {
                    label: tr("mini_controls"),
                    on: self.config.mini_controls,
                    enabled: true,
                });
                items.push(SettingsItem::GroupEnd);
                items.push(SettingsItem::RowSourceSelect {
                    label: tr("island_style"),
                    options: vec![
                        (tr("style_default"), self.config.island_style == "default"),
                        (tr("style_glass"), self.config.island_style == "glass"),
                        (tr("style_mica"), self.config.island_style == "mica"),
                        (tr("style_dynamic"), self.config.island_style == "dynamic"),
                    ],
                    enabled: true,
                });
                items.push(SettingsItem::RowFontPicker {
                    label: tr("custom_font"),
                    btn_label: tr("font_select"),
                    reset_label: if self.config.custom_font_path.is_some() {
                        Some(tr("font_reset"))
                    } else {
                        None
                    },
                });
                items.push(SettingsItem::FontPreview {
                    has_custom_font: self.config.custom_font_path.is_some(),
                });
                items.push(SettingsItem::GroupEnd);
            }
            2 => {
                items.push(SettingsItem::SectionHeader {
                    label: tr("section_behavior"),
                });
                items.push(SettingsItem::GroupStart);
                items.push(SettingsItem::RowSwitch {
                    label: tr("start_boot"),
                    on: self.config.auto_start,
                    enabled: true,
                });
                items.push(SettingsItem::RowSwitch {
                    label: tr("auto_hide"),
                    on: self.config.auto_hide,
                    enabled: true,
                });
                if self.config.auto_hide {
                    items.push(SettingsItem::RowStepper {
                        label: tr("hide_delay"),
                        value: format!("{:.0}", self.config.auto_hide_delay),
                        enabled: true,
                    });
                }
                items.push(SettingsItem::RowSourceSelect {
                    label: tr("language"),
                    options: vec![
                        ("English".to_string(), current_lang() == "en"),
                        ("中文".to_string(), current_lang() == "zh"),
                    ],
                    enabled: true,
                });
                items.push(SettingsItem::GroupEnd);

                items.push(SettingsItem::SectionHeader {
                    label: tr("section_updates"),
                });
                items.push(SettingsItem::GroupStart);
                items.push(SettingsItem::RowSwitch {
                    label: tr("check_updates"),
                    on: self.config.check_for_updates,
                    enabled: true,
                });
                if self.config.check_for_updates {
                    items.push(SettingsItem::RowStepper {
                        label: tr("update_interval"),
                        value: format!("{:.0}", self.config.update_check_interval),
                        enabled: true,
                    });
                }
                items.push(SettingsItem::GroupEnd);

                items.push(SettingsItem::Spacer { height: 10.0 });
                items.push(SettingsItem::CenterLink {
                    label: tr("reset_defaults"),
                    color: self.theme().danger,
                });
            }
            _ => {}
        }
        items
    }

    fn build_music_items(&self) -> Vec<SettingsItem> {
        let show_lyrics = self.config.show_lyrics;
        let enabled = self.config.smtc_enabled;
        let source = &self.config.lyrics_source;

        let mut items = vec![
            SettingsItem::PageTitle {
                text: tr("tab_music"),
            },
            SettingsItem::SectionHeader {
                label: tr("section_playback"),
            },
            SettingsItem::GroupStart,
            SettingsItem::RowSwitch {
                label: tr("smtc_control"),
                on: self.config.smtc_enabled,
                enabled: true,
            },
            SettingsItem::GroupEnd,
            SettingsItem::SectionHeader {
                label: tr("section_lyrics"),
            },
            SettingsItem::GroupStart,
            SettingsItem::RowSwitch {
                label: tr("show_lyrics"),
                on: self.config.show_lyrics,
                enabled: true,
            },
            SettingsItem::RowSourceSelect {
                label: tr("lyrics_source"),
                options: vec![
                    ("163".to_string(), source == "163"),
                    ("LRCLIB".to_string(), source == "lrclib"),
                ],
                enabled: show_lyrics,
            },
            SettingsItem::RowSwitch {
                label: tr("lyrics_fallback"),
                on: if show_lyrics {
                    self.config.lyrics_fallback
                } else {
                    false
                },
                enabled: show_lyrics,
            },
            SettingsItem::RowStepper {
                label: tr("lyrics_delay"),
                value: format!("{:.1}", self.config.lyrics_delay),
                enabled: show_lyrics,
            },
            SettingsItem::RowSwitch {
                label: tr("lyrics_scroll"),
                on: if show_lyrics {
                    self.config.lyrics_scroll
                } else {
                    false
                },
                enabled: show_lyrics,
            },
            SettingsItem::RowStepper {
                label: tr("lyrics_scroll_max_width"),
                value: format!("{}", self.config.lyrics_scroll_max_width as i32),
                enabled: show_lyrics && self.config.lyrics_scroll,
            },
            SettingsItem::GroupEnd,
            SettingsItem::SectionHeader {
                label: tr("media_apps"),
            },
            SettingsItem::GroupStart,
        ];

        if self.detected_apps.is_empty() {
            items.push(SettingsItem::RowLabel {
                label: tr("no_sessions"),
            });
        } else {
            for app in &self.detected_apps {
                let display_name = app.split('!').next().unwrap_or(app).to_string();
                let active = self.config.smtc_apps.contains(app);
                items.push(SettingsItem::RowAppItem {
                    label: display_name,
                    active,
                    enabled,
                });
            }
        }
        items.push(SettingsItem::GroupEnd);
        items
    }

    fn build_about_items(&self) -> Vec<SettingsItem> {
        let theme = self.theme();
        vec![
            SettingsItem::PageTitle {
                text: tr("tab_about"),
            },
            SettingsItem::Spacer { height: 20.0 },
            SettingsItem::CenterText {
                text: "WinIsland".to_string(),
                size: 28.0,
                color: theme.text_pri,
            },
            SettingsItem::CenterText {
                text: format!("Version {}", APP_VERSION),
                size: 14.0,
                color: theme.text_sec,
            },
            SettingsItem::CenterText {
                text: format!("{} {}", tr("created_by"), APP_AUTHOR),
                size: 14.0,
                color: theme.text_sec,
            },
            SettingsItem::Spacer { height: 10.0 },
            SettingsItem::CenterLink {
                label: tr("visit_homepage"),
                color: theme.accent,
            },
        ]
    }

    fn build_current_items(&self) -> Vec<SettingsItem> {
        match self.active_page {
            0 => self.build_general_items(),
            1 => self.build_music_items(),
            2 => self.build_about_items(),
            _ => vec![],
        }
    }

    fn rebuild_items_cache(&mut self) {
        self.cached_items = self.build_current_items();
        let content_start_y = if self.active_page == 0 {
            SUB_TAB_START_Y + SUB_TAB_H + CONTENT_START_Y
        } else {
            CONTENT_START_Y
        };
        self.cached_content_height = content_height(&self.cached_items, content_start_y);
        let scale = self
            .window
            .as_ref()
            .map(|w| w.scale_factor() as f32)
            .unwrap_or(1.0);
        let view_h = self.win_h / scale;
        self.cached_max_scroll = (self.cached_content_height - view_h + 20.0).max(0.0);
        self.cached_row_tops.clear();
        let mut y = content_start_y;
        for item in &self.cached_items {
            if item.is_row() {
                self.cached_row_tops.push(y);
            }
            y += item.height();
        }
        self.total_rows = self.cached_row_tops.len();
        self.items_dirty = false;
    }

    fn ensure_items_cache(&mut self) {
        if self.items_dirty {
            self.rebuild_items_cache();
        }
    }

    fn mark_items_dirty(&mut self) {
        self.items_dirty = true;
    }

    fn get_monitor_list(&self) -> Vec<String> {
        use windows::Win32::Graphics::Gdi::*;
        let mut monitors: Vec<String> = Vec::new();
        unsafe {
            let mut idx = 0u32;
            let mut active_count = 0;
            loop {
                let mut dd: DISPLAY_DEVICEW = std::mem::zeroed();
                dd.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
                if EnumDisplayDevicesW(None, idx, &mut dd, 0).as_bool() {
                    if (dd.StateFlags & DISPLAY_DEVICE_ACTIVE) != 0 {
                        active_count += 1;
                        let name = String::from_utf16_lossy(&dd.DeviceName)
                            .trim_end_matches('\0')
                            .to_string();
                        let mut dm: DISPLAY_DEVICEW = std::mem::zeroed();
                        dm.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
                        let mut label = if EnumDisplayDevicesW(
                            windows::core::PCWSTR(dd.DeviceName.as_ptr()),
                            0,
                            &mut dm,
                            0,
                        )
                        .as_bool()
                        {
                            let friendly = String::from_utf16_lossy(&dm.DeviceString)
                                .trim_end_matches('\0')
                                .to_string();
                            if friendly.is_empty() {
                                name.clone()
                            } else {
                                friendly
                            }
                        } else {
                            name.clone()
                        };
                        label = format!("Display {}: {}", active_count, label);
                        monitors.push(label);
                    }
                    idx += 1;
                } else {
                    break;
                }
            }
        }
        if monitors.is_empty() {
            monitors.push("Primary".to_string());
        }
        monitors
    }

    fn sync_switch_targets(&mut self) {
        self.switch_anim.set_target(0, self.config.adaptive_border);
        self.switch_anim.set_target(1, self.config.motion_blur);
        self.switch_anim.set_target(2, self.config.cover_rotate);
        self.switch_anim.set_target(3, self.config.audio_gate);
        self.switch_anim.set_target(4, self.config.mini_controls);
        self.switch_anim.set_target(5, self.config.auto_start);
        self.switch_anim.set_target(6, self.config.auto_hide);
        self.switch_anim
            .set_target(7, self.config.check_for_updates);
        self.switch_anim.set_target(8, self.config.smtc_enabled);
        self.switch_anim.set_target(9, self.config.show_lyrics);
        let fb_on = if self.config.show_lyrics {
            self.config.lyrics_fallback
        } else {
            false
        };
        self.switch_anim.set_target(10, fb_on);
        let fw_on = if self.config.show_lyrics {
            self.config.lyrics_scroll
        } else {
            false
        };
        self.switch_anim.set_target(11, fw_on);
    }

    fn update_detected_apps(&mut self) {
        use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager;
        let mut changed = false;
        if let Ok(manager_async) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        {
            if let Ok(manager) = manager_async.get() {
                if let Ok(sessions) = manager.GetSessions() {
                    if let Ok(size) = sessions.Size() {
                        for i in 0..size {
                            if let Ok(session) = sessions.GetAt(i) {
                                if let Ok(id) = session.SourceAppUserModelId() {
                                    let name = id.to_string();
                                    if !self.detected_apps.contains(&name) {
                                        self.detected_apps.push(name);
                                        changed = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        for app in &self.config.smtc_known_apps {
            if !self.detected_apps.contains(app) {
                self.detected_apps.push(app.clone());
                changed = true;
            }
        }
        if changed {
            self.items_dirty = true;
        }
    }

    fn draw(&mut self) {
        let (p_w, p_h, scale) = {
            let win = self.window.as_ref().unwrap();
            let size = win.inner_size();
            (
                size.width as i32,
                size.height as i32,
                win.scale_factor() as f32,
            )
        };
        if p_w <= 0 || p_h <= 0 {
            return;
        }

        self.ensure_items_cache();
        let theme = self.theme();
        let win_w = self.win_w / scale;
        let win_h = self.win_h / scale;
        let anim = self.get_page_anim();

        let mut surface = match self.surface.take() {
            Some(s) => s,
            None => return,
        };

        {
            let mut buffer = match surface.buffer_mut() {
                Ok(b) => b,
                Err(_) => {
                    self.surface = Some(surface);
                    return;
                }
            };
            let info = skia_safe::ImageInfo::new(
                skia_safe::ISize::new(p_w, p_h),
                skia_safe::ColorType::BGRA8888,
                skia_safe::AlphaType::Premul,
                None,
            );
            let dst_row_bytes = (p_w * 4) as usize;
            let u8_buffer: &mut [u8] = bytemuck::cast_slice_mut(&mut *buffer);
            let expected_size = (p_w * p_h * 4) as usize;
            let actual_size = u8_buffer.len();
            if actual_size != expected_size {
                return;
            }
            let mut sk_surface = match surfaces::wrap_pixels(&info, u8_buffer, dst_row_bytes, None)
            {
                Some(s) => s,
                None => {
                    return;
                }
            };

            let canvas = sk_surface.canvas();
            canvas.reset_matrix();
            canvas.clear(theme.win_bg);
            canvas.scale((scale, scale));

            self.draw_sidebar(canvas, &theme);

            let content_w = win_w - SIDEBAR_W;
            self.draw_sub_tabs(canvas, &theme, content_w);

            let content_start_y = if self.active_page == 0 {
                SUB_TAB_START_Y + SUB_TAB_H + CONTENT_START_Y
            } else {
                CONTENT_START_Y
            };

            self.max_scroll = self.cached_max_scroll;
            self.target_scroll_y = self.target_scroll_y.clamp(0.0, self.max_scroll);

            let clip_start_y = if self.active_page == 0 {
                SUB_TAB_START_Y + SUB_TAB_H
            } else {
                0.0
            };

            canvas.save();
            canvas.clip_rect(
                Rect::from_xywh(SIDEBAR_W, clip_start_y, content_w, win_h - clip_start_y),
                skia_safe::ClipOp::Intersect,
                true,
            );
            canvas.translate((SIDEBAR_W, -self.scroll_y));
            draw_items(DrawItemsParams {
                canvas,
                items: &self.cached_items,
                start_y: content_start_y,
                width: content_w,
                anims: &anim,
                hover_anims: &self.anim,
                theme: &theme,
                visible_min_y: self.scroll_y,
                visible_max_y: self.scroll_y + win_h,
            });
            canvas.restore();

            let ch = self.cached_content_height;
            let view_h = win_h;
            if ch > view_h {
                let bar_h = (view_h / ch) * view_h;
                let bar_y = (self.scroll_y / (ch - view_h)) * (view_h - bar_h);
                let mut p = Paint::default();
                p.set_anti_alias(true);
                p.set_color(Color::from_argb(60, 255, 255, 255));
                canvas.draw_round_rect(
                    Rect::from_xywh(win_w - 6.0, bar_y, 4.0, bar_h),
                    2.0,
                    2.0,
                    &p,
                );
            }

            self.draw_popup(canvas, &theme);
            let _ = buffer.present();
        }

        self.surface = Some(surface);
    }

    fn draw_sidebar(&self, canvas: &skia_safe::Canvas, theme: &SettingsTheme) {
        let fm = FontManager::global();
        let mut paint = Paint::default();
        paint.set_anti_alias(true);

        paint.set_color(theme.sidebar_bg);
        canvas.draw_rect(Rect::from_xywh(0.0, 0.0, SIDEBAR_W, self.win_h), &paint);

        let mut sep = Paint::default();
        sep.set_anti_alias(true);
        sep.set_color(theme.separator);
        sep.set_stroke_width(0.5);
        sep.set_style(skia_safe::paint::Style::Stroke);
        canvas.draw_line((SIDEBAR_W, 0.0), (SIDEBAR_W, self.win_h), &sep);

        let pages = [tr("tab_general"), tr("tab_music"), tr("tab_about")];
        let start_y = 20.0;

        for (i, label) in pages.iter().enumerate() {
            let row_y = start_y + i as f32 * (SIDEBAR_ROW_H + 2.0);
            let row_x = SIDEBAR_PAD;
            let row_w = SIDEBAR_W - SIDEBAR_PAD * 2.0;

            if self.active_page == i {
                paint.set_color(theme.sidebar_sel);
                canvas.draw_round_rect(
                    Rect::from_xywh(row_x, row_y, row_w, SIDEBAR_ROW_H),
                    SIDEBAR_SEL_RADIUS,
                    SIDEBAR_SEL_RADIUS,
                    &paint,
                );
                paint.set_color(theme.text_pri);
            } else {
                let hover_val = self.anim.get(SIDEBAR_KEY_BASE + i as u64);
                if hover_val > 0.005 {
                    let base = theme.sidebar_hover;
                    let alpha = (base.a() as f32 * hover_val) as u8;
                    paint.set_color(Color::from_argb(alpha, base.r(), base.g(), base.b()));
                    canvas.draw_round_rect(
                        Rect::from_xywh(row_x, row_y, row_w, SIDEBAR_ROW_H),
                        SIDEBAR_SEL_RADIUS,
                        SIDEBAR_SEL_RADIUS,
                        &paint,
                    );
                }
                paint.set_color(theme.text_sec);
            }

            fm.draw_text_cached(DrawTextCachedParams {
                canvas,
                text: label,
                x: row_x + 12.0,
                y: row_y + 21.0,
                size: 13.0,
                bold: false,
                paint: &paint,
            });
        }
    }

    fn draw_sub_tabs(&self, canvas: &skia_safe::Canvas, theme: &SettingsTheme, content_w: f32) {
        if self.active_page != 0 {
            return;
        }

        let fm = FontManager::global();
        let tabs = [
            tr("section_appearance"),
            tr("section_effects"),
            tr("section_behavior"),
        ];
        let tab_w = content_w / tabs.len() as f32;
        let start_x = SIDEBAR_W;

        let mut paint = Paint::default();
        paint.set_anti_alias(true);

        paint.set_color(theme.text_pri);
        fm.draw_text_cached(DrawTextCachedParams {
            canvas,
            text: &tr("tab_general"),
            x: SIDEBAR_W + CONTENT_PADDING,
            y: 35.0,
            size: 20.0,
            bold: true,
            paint: &paint,
        });

        let mut sep = Paint::default();
        sep.set_anti_alias(true);
        sep.set_color(theme.separator);
        sep.set_stroke_width(0.5);
        sep.set_style(skia_safe::paint::Style::Stroke);
        canvas.draw_line(
            (SIDEBAR_W, SUB_TAB_START_Y + SUB_TAB_H),
            (SIDEBAR_W + content_w, SUB_TAB_START_Y + SUB_TAB_H),
            &sep,
        );

        for (i, label) in tabs.iter().enumerate() {
            let tab_x = start_x + i as f32 * tab_w;
            let is_active = self.active_sub_page == i;
            let is_hover = self.sub_tab_hover == i as i32;

            if is_active {
                paint.set_color(theme.text_pri);
                let underline_x = tab_x + tab_w / 2.0 - 30.0;
                let underline_y = SUB_TAB_START_Y + SUB_TAB_H - 2.0;
                paint.set_style(skia_safe::paint::Style::Fill);
                canvas.draw_rect(Rect::from_xywh(underline_x, underline_y, 60.0, 2.0), &paint);
            } else if is_hover {
                paint.set_color(theme.text_pri);
            } else {
                paint.set_color(theme.text_sec);
            }

            let text_x = tab_x + tab_w / 2.0;
            let text_y = SUB_TAB_START_Y + SUB_TAB_H / 2.0 + 5.0;
            fm.draw_text_cached(DrawTextCachedParams {
                canvas,
                text: label,
                x: text_x - 30.0,
                y: text_y,
                size: 13.0,
                bold: false,
                paint: &paint,
            });
        }
    }

    fn draw_popup(&self, canvas: &skia_safe::Canvas, theme: &SettingsTheme) {
        let popup = match &self.popup {
            Some(p) => p,
            None => return,
        };
        let opacity = self.anim.get(POPUP_OPACITY_KEY);
        if opacity < 0.005 {
            return;
        }
        let fm = FontManager::global();
        let menu = popup.menu_rect();

        let mut shadow = Paint::default();
        shadow.set_anti_alias(true);
        shadow.set_color(Color::from_argb((60.0 * opacity) as u8, 0, 0, 0));
        canvas.draw_round_rect(
            Rect::from_xywh(
                menu.left - 1.0,
                menu.top + 2.0,
                menu.width() + 2.0,
                menu.height() + 2.0,
            ),
            POPUP_MENU_R,
            POPUP_MENU_R,
            &shadow,
        );

        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_color(Color::from_argb(
            (255.0 * opacity) as u8,
            theme.popup_bg.r(),
            theme.popup_bg.g(),
            theme.popup_bg.b(),
        ));
        canvas.draw_round_rect(menu, POPUP_MENU_R, POPUP_MENU_R, &paint);

        let mut border = Paint::default();
        border.set_anti_alias(true);
        border.set_color(Color::from_argb(
            (40.0 * opacity) as u8,
            theme.popup_border.r(),
            theme.popup_border.g(),
            theme.popup_border.b(),
        ));
        border.set_style(skia_safe::paint::Style::Stroke);
        border.set_stroke_width(0.5);
        canvas.draw_round_rect(menu, POPUP_MENU_R, POPUP_MENU_R, &border);

        let text_alpha = (255.0 * opacity) as u8;
        for (i, opt_label) in popup.options.iter().enumerate() {
            let item_rect = popup.item_rect(i);

            if popup.hover_idx == Some(i) {
                let a = theme.accent.a() as f32 * opacity;
                paint.set_color(Color::from_argb(
                    a as u8,
                    theme.accent.r(),
                    theme.accent.g(),
                    theme.accent.b(),
                ));
                paint.set_style(skia_safe::paint::Style::Fill);
                canvas.draw_round_rect(item_rect, 4.0, 4.0, &paint);
            }

            paint.set_color(Color::from_argb(
                text_alpha,
                theme.text_pri.r(),
                theme.text_pri.g(),
                theme.text_pri.b(),
            ));
            paint.set_style(skia_safe::paint::Style::Fill);
            fm.draw_text_cached(DrawTextCachedParams {
                canvas,
                text: opt_label,
                x: item_rect.left + 8.0,
                y: item_rect.top + 19.0,
                size: 12.0,
                bold: false,
                paint: &paint,
            });

            if i == popup.selected_idx {
                let check_base = if popup.hover_idx == Some(i) {
                    theme.text_pri
                } else {
                    theme.accent
                };
                paint.set_color(Color::from_argb(
                    text_alpha,
                    check_base.r(),
                    check_base.g(),
                    check_base.b(),
                ));
                paint.set_style(skia_safe::paint::Style::Stroke);
                paint.set_stroke_width(2.0);
                let cx = item_rect.right - 14.0;
                let cy = item_rect.top + POPUP_ITEM_H / 2.0;
                let svg = format!(
                    "M {} {} L {} {} L {} {}",
                    cx - 4.0,
                    cy,
                    cx - 1.0,
                    cy + 3.0,
                    cx + 4.0,
                    cy - 3.0,
                );
                if let Some(path) = skia_safe::Path::from_svg(&svg) {
                    canvas.draw_path(&path, &paint);
                }
                paint.set_style(skia_safe::paint::Style::Fill);
            }

            if i < popup.options.len() - 1 {
                let mut sep = Paint::default();
                sep.set_anti_alias(true);
                sep.set_color(Color::from_argb(
                    (30.0 * opacity) as u8,
                    theme.separator.r(),
                    theme.separator.g(),
                    theme.separator.b(),
                ));
                sep.set_stroke_width(0.5);
                sep.set_style(skia_safe::paint::Style::Stroke);
                canvas.draw_line(
                    (item_rect.left, item_rect.bottom),
                    (item_rect.right, item_rect.bottom),
                    &sep,
                );
            }
        }
    }

    fn get_page_anim(&self) -> SwitchAnimator {
        match self.active_page {
            0 => match self.active_sub_page {
                0 => SwitchAnimator::new(&[]),
                1 => SwitchAnimator::new_with_anims(&self.switch_anim, &[0, 1, 2, 3, 4]),
                2 => SwitchAnimator::new_with_anims(&self.switch_anim, &[5, 6, 7]),
                _ => SwitchAnimator::new(&[]),
            },
            1 => SwitchAnimator::new_with_anims(&self.switch_anim, &[8, 9, 10, 11]),
            _ => SwitchAnimator::new(&[]),
        }
    }

    fn handle_click(&mut self) {
        let (mx, my) = self.logical_mouse_pos;

        if let Some(popup) = &self.popup {
            if let Some(i) = popup.hit_test_item(mx, my) {
                let value = popup.values[i].clone();
                match popup.kind {
                    PopupKind::LyricsSource => {
                        self.config.lyrics_source = value;
                    }
                    PopupKind::Language => {
                        self.config.language = value;
                        set_lang(&self.config.language);
                    }
                    PopupKind::Monitor => {
                        self.config.monitor_index = value.parse::<i32>().unwrap_or(0);
                    }
                    PopupKind::IslandStyle => {
                        self.config.island_style = value;
                    }
                    PopupKind::DockPositionPopup => {
                        self.config.dock_position = value
                            .parse::<DockPosition>()
                            .unwrap_or(DockPosition::TopCenter);
                    }
                    PopupKind::SettingsTheme => {
                        self.config.settings_theme = value.clone();
                        self.update_theme();
                    }
                    PopupKind::MiniCoverShape => {
                        self.config.mini_cover_shape = value;
                    }
                    PopupKind::ExpandedCoverShape => {
                        self.config.expanded_cover_shape = value;
                    }
                }
                save_config(&self.config);
                self.mark_items_dirty();
            }
            self.popup = None;
            self.anim.set_with_speed(POPUP_OPACITY_KEY, 0.0, 0.3);
            if let Some(win) = &self.window {
                win.request_redraw();
            }
            return;
        }

        if mx < SIDEBAR_W {
            let pages = 3;
            let start_y = 20.0;
            for i in 0..pages {
                let row_y = start_y + i as f32 * (SIDEBAR_ROW_H + 2.0);
                if my >= row_y
                    && my <= row_y + SIDEBAR_ROW_H
                    && mx >= SIDEBAR_PAD
                    && mx <= SIDEBAR_W - SIDEBAR_PAD
                {
                    if self.active_page != i as usize {
                        self.active_page = i as usize;
                        self.scroll_y = 0.0;
                        self.target_scroll_y = 0.0;
                        self.scroll_vel_y = 0.0;
                        self.mark_items_dirty();
                        if let Some(win) = &self.window {
                            win.request_redraw();
                        }
                    }
                    return;
                }
            }
            return;
        }

        let scale = self.window.as_ref().unwrap().scale_factor() as f32;
        let content_w = self.win_w / scale - SIDEBAR_W;

        if self.active_page == 0 && my >= SUB_TAB_START_Y && my <= SUB_TAB_START_Y + SUB_TAB_H {
            let tab_count = 3usize;
            let tab_w = content_w / tab_count as f32;
            let rel_x = mx - SIDEBAR_W;
            let tab_idx = (rel_x / tab_w) as usize;
            if tab_idx < tab_count {
                if self.active_sub_page != tab_idx {
                    self.active_sub_page = tab_idx;
                    self.scroll_y = 0.0;
                    self.target_scroll_y = 0.0;
                    self.scroll_vel_y = 0.0;
                    self.mark_items_dirty();
                    if let Some(win) = &self.window {
                        win.request_redraw();
                    }
                }
            }
            return;
        }

        let content_x = mx - SIDEBAR_W;
        let content_start_y = if self.active_page == 0 {
            SUB_TAB_START_Y + SUB_TAB_H + CONTENT_START_Y
        } else {
            CONTENT_START_Y
        };
        let content_y = my + self.scroll_y;
        let items = self.build_current_items();

        match self.active_page {
            0 => {
                self.handle_general_click(&items, content_x, content_y, content_w, content_start_y)
            }
            1 => self.handle_music_click(&items, content_x, content_y, content_w, content_start_y),
            2 => self.handle_about_click(&items, content_x, content_y, content_w, content_start_y),
            _ => {}
        }
    }

    fn handle_general_click(
        &mut self,
        items: &[SettingsItem],
        mx: f32,
        my: f32,
        width: f32,
        start_y: f32,
    ) {
        let scale = self.window.as_ref().unwrap().scale_factor() as f32;
        let result = hit_test(items, mx, my, start_y, width);
        let mut changed = false;

        match result {
            ClickResult::StepperDec(idx) | ClickResult::StepperInc(idx) => {
                let is_dec = matches!(result, ClickResult::StepperDec(_));
                if let Some(item) = items.get(idx)
                    && let SettingsItem::RowStepper { label, .. } = item
                {
                    let l = label.clone();
                    if l == tr("global_scale") {
                        if is_dec {
                            self.config.global_scale =
                                ((self.config.global_scale - 0.05) * 100.0).round() / 100.0;
                            self.config.global_scale = self.config.global_scale.max(0.5);
                        } else {
                            self.config.global_scale =
                                ((self.config.global_scale + 0.05) * 100.0).round() / 100.0;
                            self.config.global_scale = self.config.global_scale.min(5.0);
                        }
                        changed = true;
                    } else if l == tr("base_width") {
                        if is_dec {
                            self.config.base_width -= 5.0;
                        } else {
                            self.config.base_width += 5.0;
                        }
                        changed = true;
                    } else if l == tr("base_height") {
                        if is_dec {
                            self.config.base_height -= 2.0;
                        } else {
                            self.config.base_height += 2.0;
                        }
                        changed = true;
                    } else if l == tr("expanded_width") {
                        if is_dec {
                            self.config.expanded_width -= 10.0;
                        } else {
                            self.config.expanded_width += 10.0;
                        }
                        changed = true;
                    } else if l == tr("expanded_height") {
                        if is_dec {
                            self.config.expanded_height -= 10.0;
                        } else {
                            self.config.expanded_height += 10.0;
                        }
                        changed = true;
                    } else if l == tr("position_x_offset") {
                        if is_dec {
                            self.config.position_x_offset -= 5;
                        } else {
                            self.config.position_x_offset += 5;
                        }
                        changed = true;
                    } else if l == tr("position_y_offset") {
                        if is_dec {
                            self.config.position_y_offset -= 5;
                        } else {
                            self.config.position_y_offset += 5;
                        }
                        changed = true;
                    } else if l == tr("font_size") {
                        if is_dec {
                            self.config.font_size = (self.config.font_size - 1.0).max(0.0);
                        } else {
                            self.config.font_size = (self.config.font_size + 1.0).min(30.0);
                        }
                        changed = true;
                    } else if l == tr("hide_delay") {
                        if is_dec {
                            self.config.auto_hide_delay =
                                (self.config.auto_hide_delay - 1.0).max(1.0);
                        } else {
                            self.config.auto_hide_delay =
                                (self.config.auto_hide_delay + 1.0).min(60.0);
                        }
                        changed = true;
                    } else if l == tr("update_interval") {
                        if is_dec {
                            self.config.update_check_interval =
                                (self.config.update_check_interval - 1.0).max(1.0);
                        } else {
                            self.config.update_check_interval =
                                (self.config.update_check_interval + 1.0).min(24.0);
                        }
                        changed = true;
                    }
                }
            }
            ClickResult::Switch(idx) => {
                let label = items
                    .iter()
                    .filter_map(|item| match item {
                        SettingsItem::RowSwitch { label, .. } => Some(label.clone()),
                        _ => None,
                    })
                    .nth(idx);
                if let Some(label) = label {
                    match label.as_str() {
                        l if l == tr("adaptive_border") => {
                            self.config.adaptive_border = !self.config.adaptive_border
                        }
                        l if l == tr("motion_blur") => {
                            self.config.motion_blur = !self.config.motion_blur
                        }
                        l if l == tr("cover_rotate") => {
                            self.config.cover_rotate = !self.config.cover_rotate
                        }
                        l if l == tr("audio_gate") => {
                            self.config.audio_gate = !self.config.audio_gate
                        }
                        l if l == tr("mini_controls") => {
                            self.config.mini_controls = !self.config.mini_controls
                        }
                        l if l == tr("start_boot") => {
                            self.config.auto_start = !self.config.auto_start;
                            let _ = set_autostart(self.config.auto_start);
                        }
                        l if l == tr("auto_hide") => {
                            self.config.auto_hide = !self.config.auto_hide
                        }
                        l if l == tr("check_updates") => {
                            self.config.check_for_updates = !self.config.check_for_updates
                        }
                        l if l == tr("smtc_control") => {
                            self.config.smtc_enabled = !self.config.smtc_enabled
                        }
                        l if l == tr("show_lyrics") => {
                            self.config.show_lyrics = !self.config.show_lyrics
                        }
                        l if l == tr("lyrics_fallback") => {
                            if self.config.show_lyrics {
                                self.config.lyrics_fallback = !self.config.lyrics_fallback
                            }
                        }
                        l if l == tr("lyrics_scroll") => {
                            if self.config.show_lyrics {
                                self.config.lyrics_scroll = !self.config.lyrics_scroll
                            }
                        }
                        _ => {
                            eprintln!("WinIsland: unhandled switch label: {}", label);
                        }
                    }
                }
                self.sync_switch_targets();
                changed = true;
            }
            ClickResult::FontSelect(_) => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Fonts", &["ttf", "otf"])
                    .pick_file()
                {
                    self.config.custom_font_path = Some(path.to_string_lossy().into_owned());
                    FontManager::global().refresh_custom_font();
                    changed = true;
                }
            }
            ClickResult::FontReset(_) => {
                self.config.custom_font_path = None;
                FontManager::global().refresh_custom_font();
                changed = true;
            }
            ClickResult::SourceButton(idx) => {
                let content_w = width;
                let mut btn_content_y = start_y;
                for item in items.iter().take(idx) {
                    btn_content_y += item.height();
                }
                let cy = btn_content_y + ROW_HEIGHT / 2.0;
                let btn_x = SIDEBAR_W + CONTENT_PADDING + content_w - GROUP_INNER_PAD - POPUP_BTN_W;
                let btn_y = cy - POPUP_BTN_H / 2.0 - self.scroll_y;

                if let Some(SettingsItem::RowSourceSelect { label, .. }) = items.get(idx) {
                    if label == &tr("monitor") {
                        let monitors = self.get_monitor_list();
                        let selected_idx = (self.config.monitor_index as usize)
                            .min(monitors.len().saturating_sub(1));
                        let values: Vec<String> =
                            (0..monitors.len()).map(|i| i.to_string()).collect();
                        self.popup = Some(PopupState::new(
                            PopupKind::Monitor,
                            Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            monitors,
                            values,
                            selected_idx,
                            self.win_w / scale,
                            self.win_h / scale,
                        ));
                    } else if label == &tr("island_style") {
                        let selected_idx = match self.config.island_style.as_str() {
                            "glass" => 1,
                            "mica" => 2,
                            "dynamic" => 3,
                            _ => 0,
                        };
                        self.popup = Some(PopupState::new(
                            PopupKind::IslandStyle,
                            Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            vec![
                                tr("style_default"),
                                tr("style_glass"),
                                tr("style_mica"),
                                tr("style_dynamic"),
                            ],
                            vec![
                                "default".to_string(),
                                "glass".to_string(),
                                "mica".to_string(),
                                "dynamic".to_string(),
                            ],
                            selected_idx,
                            self.win_w / scale,
                            self.win_h / scale,
                        ));
                    } else if label == &tr("dock_position") {
                        let dp = self.config.dock_position;
                        let selected_idx = match dp {
                            DockPosition::TopCenter => 0,
                            DockPosition::TopLeft => 1,
                            DockPosition::TopRight => 2,
                            DockPosition::BottomCenter => 3,
                            DockPosition::BottomLeft => 4,
                            DockPosition::BottomRight => 5,
                        };
                        self.popup = Some(PopupState::new(
                            PopupKind::DockPositionPopup,
                            Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            vec![
                                tr("dock_position_top_center"),
                                tr("dock_position_top_left"),
                                tr("dock_position_top_right"),
                                tr("dock_position_bottom_center"),
                                tr("dock_position_bottom_left"),
                                tr("dock_position_bottom_right"),
                            ],
                            vec![
                                "top_center".to_string(),
                                "top_left".to_string(),
                                "top_right".to_string(),
                                "bottom_center".to_string(),
                                "bottom_left".to_string(),
                                "bottom_right".to_string(),
                            ],
                            selected_idx,
                            self.win_w / scale,
                            self.win_h / scale,
                        ));
                    } else if label == &tr("settings_theme") {
                        let selected_idx = match self.config.settings_theme.as_str() {
                            "light" => 1,
                            "dark" => 2,
                            _ => 0,
                        };
                        self.popup = Some(PopupState::new(
                            PopupKind::SettingsTheme,
                            Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            vec![tr("theme_system"), tr("theme_light"), tr("theme_dark")],
                            vec![
                                "system".to_string(),
                                "light".to_string(),
                                "dark".to_string(),
                            ],
                            selected_idx,
                            self.win_w / scale,
                            self.win_h / scale,
                        ));
                    } else if label == &tr("mini_cover_shape") {
                        let selected_idx = if self.config.mini_cover_shape == "circle" {
                            1
                        } else {
                            0
                        };
                        self.popup = Some(PopupState::new(
                            PopupKind::MiniCoverShape,
                            Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            vec![tr("shape_square"), tr("shape_circle")],
                            vec!["square".to_string(), "circle".to_string()],
                            selected_idx,
                            self.win_w / scale,
                            self.win_h / scale,
                        ));
                    } else if label == &tr("expanded_cover_shape") {
                        let selected_idx = if self.config.expanded_cover_shape == "circle" {
                            1
                        } else {
                            0
                        };
                        self.popup = Some(PopupState::new(
                            PopupKind::ExpandedCoverShape,
                            Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            vec![tr("shape_square"), tr("shape_circle")],
                            vec!["square".to_string(), "circle".to_string()],
                            selected_idx,
                            self.win_w / scale,
                            self.win_h / scale,
                        ));
                    } else {
                        let lang = current_lang();
                        self.popup = Some(PopupState::new(
                            PopupKind::Language,
                            Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            vec!["English".to_string(), "中文".to_string()],
                            vec!["en".to_string(), "zh".to_string()],
                            if lang == "zh" { 1 } else { 0 },
                            self.win_w / scale,
                            self.win_h / scale,
                        ));
                    }
                    self.anim.set_with_speed(POPUP_OPACITY_KEY, 1.0, 0.25);
                    if let Some(win) = &self.window {
                        win.request_redraw();
                    }
                }
            }
            ClickResult::CenterLink(_) => {
                self.config = AppConfig::default();
                init_i18n(&self.config.language);
                FontManager::global().refresh_custom_font();
                self.switch_anim = SwitchAnimator::new(&[
                    self.config.adaptive_border,
                    self.config.motion_blur,
                    self.config.cover_rotate,
                    self.config.audio_gate,
                    self.config.auto_start,
                    self.config.auto_hide,
                    self.config.check_for_updates,
                    self.config.smtc_enabled,
                    self.config.show_lyrics,
                    self.config.lyrics_fallback,
                    self.config.lyrics_scroll,
                ]);
                changed = true;
            }
            _ => {}
        }

        if changed {
            self.mark_items_dirty();
            save_config(&self.config);
            if let Some(win) = &self.window {
                win.request_redraw();
            }
        }
    }

    fn handle_music_click(
        &mut self,
        items: &[SettingsItem],
        mx: f32,
        my: f32,
        width: f32,
        start_y: f32,
    ) {
        let scale = self.window.as_ref().unwrap().scale_factor() as f32;
        let result = hit_test(items, mx, my, start_y, width);
        let mut changed = false;

        match result {
            ClickResult::Switch(idx) => {
                let label = items
                    .iter()
                    .filter_map(|item| match item {
                        SettingsItem::RowSwitch { label, .. } => Some(label.clone()),
                        _ => None,
                    })
                    .nth(idx);
                if let Some(label) = label {
                    match label.as_str() {
                        l if l == tr("smtc_control") => {
                            self.config.smtc_enabled = !self.config.smtc_enabled
                        }
                        l if l == tr("show_lyrics") => {
                            self.config.show_lyrics = !self.config.show_lyrics
                        }
                        l if l == tr("lyrics_fallback") => {
                            if self.config.show_lyrics {
                                self.config.lyrics_fallback = !self.config.lyrics_fallback
                            }
                        }
                        l if l == tr("lyrics_scroll") => {
                            if self.config.show_lyrics {
                                self.config.lyrics_scroll = !self.config.lyrics_scroll
                            }
                        }
                        _ => {
                            eprintln!("WinIsland: unhandled switch label: {}", label);
                        }
                    }
                }
                self.sync_switch_targets();
                changed = true;
            }
            ClickResult::SourceButton(idx) => {
                let content_w = width;
                let mut btn_content_y = start_y;
                for item in items.iter().take(idx) {
                    btn_content_y += item.height();
                }
                let cy = btn_content_y + ROW_HEIGHT / 2.0;
                let btn_x = SIDEBAR_W + CONTENT_PADDING + content_w - GROUP_INNER_PAD - POPUP_BTN_W;
                let btn_y = cy - POPUP_BTN_H / 2.0 - self.scroll_y;

                let source = &self.config.lyrics_source;
                self.popup = Some(PopupState::new(
                    PopupKind::LyricsSource,
                    Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                    vec!["163".to_string(), "LRCLIB".to_string()],
                    vec!["163".to_string(), "lrclib".to_string()],
                    if source == "163" { 0 } else { 1 },
                    self.win_w / scale,
                    self.win_h / scale,
                ));
                self.anim.set_with_speed(POPUP_OPACITY_KEY, 1.0, 0.25);
                if let Some(win) = &self.window {
                    win.request_redraw();
                }
            }
            ClickResult::StepperDec(idx) | ClickResult::StepperInc(idx) => {
                let is_dec = matches!(result, ClickResult::StepperDec(_));
                if let Some(item) = items.get(idx)
                    && let SettingsItem::RowStepper { label, .. } = item
                {
                    if label == &tr("lyrics_delay") && self.config.show_lyrics {
                        if is_dec {
                            self.config.lyrics_delay =
                                ((self.config.lyrics_delay * 10.0 - 1.0).round() / 10.0).max(-10.0);
                        } else {
                            self.config.lyrics_delay =
                                ((self.config.lyrics_delay * 10.0 + 1.0).round() / 10.0).min(10.0);
                        }
                        changed = true;
                    } else if label == &tr("lyrics_scroll_max_width")
                        && self.config.show_lyrics
                        && self.config.lyrics_scroll
                    {
                        if is_dec {
                            self.config.lyrics_scroll_max_width =
                                (self.config.lyrics_scroll_max_width - 10.0).max(100.0);
                        } else {
                            self.config.lyrics_scroll_max_width =
                                (self.config.lyrics_scroll_max_width + 10.0).min(500.0);
                        }
                        changed = true;
                    }
                }
            }
            ClickResult::AppItem(idx)
                if self.config.smtc_enabled && !self.detected_apps.is_empty() =>
            {
                let app_start = items
                    .iter()
                    .position(|i| matches!(i, SettingsItem::RowAppItem { .. }))
                    .unwrap_or(items.len());
                let app_idx = idx - app_start;
                if app_idx < self.detected_apps.len() {
                    let app = &self.detected_apps[app_idx];
                    if self.config.smtc_apps.contains(app) {
                        self.config.smtc_apps.retain(|a| a != app);
                    } else {
                        self.config.smtc_apps.push(app.clone());
                        if !self.config.smtc_known_apps.contains(app) {
                            self.config.smtc_known_apps.push(app.clone());
                        }
                    }
                    changed = true;
                }
            }
            _ => {}
        }

        if changed {
            self.mark_items_dirty();
            save_config(&self.config);
            if let Some(win) = &self.window {
                win.request_redraw();
            }
        }
    }

    fn handle_about_click(
        &mut self,
        items: &[SettingsItem],
        mx: f32,
        my: f32,
        width: f32,
        start_y: f32,
    ) {
        let result = hit_test(items, mx, my, start_y, width);
        if let ClickResult::CenterLink(_) = result {
            let _ = open::that(APP_HOMEPAGE);
        }
    }

    fn get_hover_state(&mut self) -> bool {
        let (mx, my) = self.logical_mouse_pos;

        if let Some(popup) = &self.popup {
            let menu = popup.menu_rect();
            if mx >= menu.left && mx <= menu.right && my >= menu.top && my <= menu.bottom {
                return true;
            }
        }

        if mx < SIDEBAR_W {
            let start_y = 20.0;
            for i in 0..3 {
                let row_y = start_y + i as f32 * (SIDEBAR_ROW_H + 2.0);
                if my >= row_y
                    && my <= row_y + SIDEBAR_ROW_H
                    && mx >= SIDEBAR_PAD
                    && mx <= SIDEBAR_W - SIDEBAR_PAD
                {
                    return true;
                }
            }
            return false;
        }

        let scale = self.window.as_ref().unwrap().scale_factor() as f32;
        let content_w = self.win_w / scale - SIDEBAR_W;

        if self.active_page == 0 && my >= SUB_TAB_START_Y && my <= SUB_TAB_START_Y + SUB_TAB_H {
            return true;
        }

        let content_x = mx - SIDEBAR_W;
        let content_start_y = if self.active_page == 0 {
            SUB_TAB_START_Y + SUB_TAB_H + CONTENT_START_Y
        } else {
            CONTENT_START_Y
        };
        let content_y = my + self.scroll_y;
        self.ensure_items_cache();
        hover_test(
            &self.cached_items,
            content_x,
            content_y,
            content_start_y,
            content_w,
        )
    }
}

impl ApplicationHandler for SettingsApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let monitor = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next());
        let (win_x, win_y) = if let Some(m) = &monitor {
            let screen_size = m.size();
            let scale_factor = m.scale_factor();
            let screen_w = screen_size.width as f64 / scale_factor;
            let screen_h = screen_size.height as f64 / scale_factor;
            let win_w = WIN_W as f64;
            let win_h = WIN_H as f64;
            ((screen_w - win_w) / 2.0, (screen_h - win_h) / 2.0)
        } else {
            (100.0, 100.0)
        };

        let attrs = Window::default_attributes()
            .with_title("Settings")
            .with_inner_size(LogicalSize::new(WIN_W as f64, WIN_H as f64))
            .with_min_inner_size(LogicalSize::new(WIN_W as f64, WIN_H as f64))
            .with_position(LogicalPosition::new(win_x, win_y))
            .with_resizable(true)
            .with_enabled_buttons(WindowButtons::CLOSE | WindowButtons::MINIMIZE)
            .with_window_icon(get_app_icon());
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        self.window = Some(window.clone());
        let context = Context::new(window.clone()).unwrap();
        let mut surface = Surface::new(&context, window.clone()).unwrap();
        let size = window.inner_size();
        self.win_w = size.width as f32;
        self.win_h = size.height as f32;
        resize_surface(&mut surface, size.width, size.height);
        self.surface = Some(surface);
        self.update_theme();
        self.update_detected_apps();
    }

    fn window_event(&mut self, _el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => _el.exit(),
            WindowEvent::ThemeChanged(theme) => {
                if self.config.settings_theme == "system" {
                    self.is_light = theme == winit::window::Theme::Light;
                    if let Some(win) = &self.window {
                        win.request_redraw();
                    }
                }
            }
            WindowEvent::Resized(new_size) => {
                self.win_w = new_size.width as f32;
                self.win_h = new_size.height as f32;
                if let Some(surface) = &mut self.surface {
                    resize_surface(surface, new_size.width, new_size.height);
                    if let Some(win) = &self.window {
                        win.request_redraw();
                    }
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let (Some(win), Some(surface)) = (&self.window, &mut self.surface) {
                    let size = win.inner_size();
                    resize_surface(surface, size.width, size.height);
                    win.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    match event.logical_key {
                        Key::Named(NamedKey::F11) => {}
                        Key::Named(NamedKey::ArrowLeft) => {
                            if self.active_page == 0 {
                                if self.active_sub_page > 0 {
                                    self.active_sub_page -= 1;
                                    self.scroll_y = 0.0;
                                    self.target_scroll_y = 0.0;
                                    self.scroll_vel_y = 0.0;
                                    self.mark_items_dirty();
                                    if let Some(win) = &self.window {
                                        win.request_redraw();
                                    }
                                }
                            } else if self.active_page > 0 {
                                self.active_page -= 1;
                                self.scroll_y = 0.0;
                                self.target_scroll_y = 0.0;
                                self.scroll_vel_y = 0.0;
                                self.mark_items_dirty();
                                if let Some(win) = &self.window {
                                    win.request_redraw();
                                }
                            }
                        }
                        Key::Named(NamedKey::ArrowRight) => {
                            if self.active_page == 0 {
                                if self.active_sub_page < 2 {
                                    self.active_sub_page += 1;
                                    self.scroll_y = 0.0;
                                    self.target_scroll_y = 0.0;
                                    self.scroll_vel_y = 0.0;
                                    self.mark_items_dirty();
                                    if let Some(win) = &self.window {
                                        win.request_redraw();
                                    }
                                }
                            } else if self.active_page < 2 {
                                self.active_page += 1;
                                self.scroll_y = 0.0;
                                self.target_scroll_y = 0.0;
                                self.scroll_vel_y = 0.0;
                                self.mark_items_dirty();
                                if let Some(win) = &self.window {
                                    win.request_redraw();
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.window.as_ref().unwrap().scale_factor() as f32;
                let new_pos = (position.x as f32 / scale, position.y as f32 / scale);
                let mouse_moved = (new_pos.0 - self.last_hover_mouse_pos.0).abs() > 0.5
                    || (new_pos.1 - self.last_hover_mouse_pos.1).abs() > 0.5;
                self.logical_mouse_pos = new_pos;

                if let Some(popup) = &mut self.popup {
                    let (pmx, pmy) = self.logical_mouse_pos;
                    let new_hover = popup.hit_test_item(pmx, pmy);
                    if new_hover != popup.hover_idx {
                        popup.hover_idx = new_hover;
                        if let Some(win) = &self.window {
                            win.request_redraw();
                        }
                    }
                }

                if mouse_moved {
                    self.last_hover_mouse_pos = new_pos;
                    let (mx, my) = self.logical_mouse_pos;
                    let mut new_hover: i32 = -1;
                    if mx < SIDEBAR_W {
                        let start_y = 20.0;
                        for i in 0..3 {
                            let row_y = start_y + i as f32 * (SIDEBAR_ROW_H + 2.0);
                            if my >= row_y
                                && my <= row_y + SIDEBAR_ROW_H
                                && mx >= SIDEBAR_PAD
                                && mx <= SIDEBAR_W - SIDEBAR_PAD
                            {
                                new_hover = i;
                            }
                        }
                    }
                    if new_hover != self.sidebar_hover {
                        self.sidebar_hover = new_hover;
                        for idx in 0..3 {
                            if idx == new_hover as usize {
                                self.anim.set(SIDEBAR_KEY_BASE + idx as u64, 1.0);
                            } else {
                                self.anim.set(SIDEBAR_KEY_BASE + idx as u64, 0.0);
                            }
                        }
                        if let Some(win) = &self.window {
                            win.request_redraw();
                        }
                    }

                    let scale = self.window.as_ref().unwrap().scale_factor() as f32;
                    let content_w = self.win_w / scale - SIDEBAR_W;

                    if self.active_page == 0
                        && mx >= SIDEBAR_W
                        && my >= SUB_TAB_START_Y
                        && my <= SUB_TAB_START_Y + SUB_TAB_H
                    {
                        let tab_count = 3i32;
                        let tab_w = content_w / tab_count as f32;
                        let rel_x = mx - SIDEBAR_W;
                        let tab_idx = (rel_x / tab_w) as i32;
                        let new_sub_hover = if tab_idx >= 0 && tab_idx < tab_count {
                            tab_idx
                        } else {
                            -1
                        };
                        if new_sub_hover != self.sub_tab_hover {
                            self.sub_tab_hover = new_sub_hover;
                            if let Some(win) = &self.window {
                                win.request_redraw();
                            }
                        }
                    } else if self.sub_tab_hover != -1 {
                        self.sub_tab_hover = -1;
                        if let Some(win) = &self.window {
                            win.request_redraw();
                        }
                    }

                    if mx >= SIDEBAR_W {
                        let content_x = mx - SIDEBAR_W;
                        let content_y = my + self.scroll_y;
                        let mut new_row: Option<usize> = None;
                        self.ensure_items_cache();
                        if content_x >= CONTENT_PADDING && content_x <= content_w - CONTENT_PADDING
                        {
                            let idx = match self
                                .cached_row_tops
                                .binary_search_by(|y| y.partial_cmp(&content_y).unwrap())
                            {
                                Ok(i) => Some(i),
                                Err(0) => None,
                                Err(i) => Some(i - 1),
                            };
                            if let Some(i) = idx
                                && content_y <= self.cached_row_tops[i] + ROW_HEIGHT
                            {
                                new_row = Some(i);
                            }
                        }
                        if new_row != self.hover_row {
                            if let Some(old) = self.hover_row {
                                self.anim.set(HOVER_ROW_KEY_BASE + old as u64, 0.0);
                            }
                            if let Some(new) = new_row {
                                self.anim.set(HOVER_ROW_KEY_BASE + new as u64, 1.0);
                            }
                            self.hover_row = new_row;
                        }
                    } else if self.hover_row.is_some() {
                        if let Some(old) = self.hover_row {
                            self.anim.set(HOVER_ROW_KEY_BASE + old as u64, 0.0);
                        }
                        self.hover_row = None;
                    }
                }

                let cursor = if self.get_hover_state() {
                    winit::window::CursorIcon::Pointer
                } else {
                    winit::window::CursorIcon::Default
                };
                if let Some(win) = &self.window {
                    win.set_cursor(cursor);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if self.popup.is_some() {
                    self.popup = None;
                    self.anim.set_with_speed(POPUP_OPACITY_KEY, 0.0, 0.3);
                    if let Some(win) = &self.window {
                        win.request_redraw();
                    }
                    return;
                }
                let (mx, _) = self.logical_mouse_pos;
                if mx >= SIDEBAR_W {
                    let diff = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => y * 40.0,
                        winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                    };
                    self.target_scroll_y =
                        (self.target_scroll_y - diff).clamp(0.0, self.max_scroll);
                    if let Some(win) = &self.window {
                        win.request_redraw();
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.handle_click();
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {}
            WindowEvent::RedrawRequested => self.draw(),
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        if self.window.is_none() {
            return;
        }

        let frame_start = Instant::now();
        self.frame_count += 1;
        if self.frame_count.is_multiple_of(30) {
            unsafe {
                let h = OpenMutexW(
                    MUTEX_ALL_ACCESS,
                    false,
                    w!("Local\\WinIsland_SingleInstance_Mutex"),
                );
                if h.is_err() {
                    _el.exit();
                    return;
                }
                let _ = windows::Win32::Foundation::CloseHandle(h.unwrap());
            }
        }
        if self.frame_count.is_multiple_of(120) {
            self.update_detected_apps();
        }

        let has_anim = self.switch_anim.is_animating() || self.anim.is_animating();
        let has_popup = self.popup.is_some();
        let is_scrolling = (self.target_scroll_y - self.scroll_y).abs() > 0.1;

        if !has_anim && !has_popup && !is_scrolling {
            return;
        }

        let mut redraw = self.switch_anim.tick();
        if self.anim.tick() {
            redraw = true;
        }

        self.ensure_items_cache();
        let max_scroll = self.cached_max_scroll;
        self.target_scroll_y = self.target_scroll_y.clamp(0.0, max_scroll);

        let dt = self
            .last_frame_time
            .elapsed()
            .as_secs_f32()
            .clamp(0.001, 0.05);
        self.last_frame_time = Instant::now();

        let diff = self.target_scroll_y - self.scroll_y;
        let accel = diff * SCROLL_STIFFNESS - self.scroll_vel_y * SCROLL_DAMPING;
        self.scroll_vel_y += accel * dt;
        self.scroll_y += self.scroll_vel_y * dt;

        if self.scroll_y < 0.0 {
            self.scroll_y = 0.0;
            self.scroll_vel_y = 0.0;
        } else if self.scroll_y > max_scroll {
            self.scroll_y = max_scroll;
            self.scroll_vel_y = 0.0;
        }

        if diff.abs() > 0.05 || self.scroll_vel_y.abs() > 0.05 {
            redraw = true;
        } else if (self.scroll_y - self.target_scroll_y).abs() > f32::EPSILON {
            self.scroll_y = self.target_scroll_y;
            self.scroll_vel_y = 0.0;
        }

        if redraw {
            if let Some(win) = &self.window {
                win.request_redraw();
            }
            let target = Duration::from_millis(16);
            let elapsed = frame_start.elapsed();
            if elapsed < target {
                std::thread::sleep(target - elapsed);
            }
        }
    }
}

pub fn run_settings(config: AppConfig) {
    let el = EventLoop::new().unwrap();
    let mut app = SettingsApp::new(config);
    el.run_app(&mut app).unwrap();
}

pub fn bring_settings_to_front() {
    unsafe {
        let hwnd = FindWindowW(None, w!("Settings"));
        if let Ok(hwnd) = hwnd {
            if !hwnd.is_invalid() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
                let _ = SetForegroundWindow(hwnd);
            }
        }
    }
}

fn resize_surface(surface: &mut Surface<Arc<Window>, Arc<Window>>, width: u32, height: u32) {
    if let (Some(w), Some(h)) = (
        std::num::NonZeroU32::new(width),
        std::num::NonZeroU32::new(height),
    ) {
        let _ = surface.resize(w, h);
    }
}
