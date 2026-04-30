use crate::core::config::{AppConfig, APP_AUTHOR, APP_HOMEPAGE, APP_VERSION};
use crate::core::persistence::save_config;
use crate::core::i18n::{tr, set_lang, current_lang};
use crate::utils::anim::AnimPool;
use crate::utils::color::*;
use crate::utils::font::FontManager;
use crate::utils::settings_ui::*;
use crate::utils::settings_ui::items::*;
use skia_safe::{surfaces, Color, Paint, Rect};
use softbuffer::{Context, Surface};
use std::sync::Arc;
use std::time::Duration;
use windows::core::w;
use windows::Win32::System::Threading::{OpenMutexW, MUTEX_ALL_ACCESS};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, LogicalPosition};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId, WindowButtons};
use winit::keyboard::{Key, NamedKey};
use crate::utils::icon::get_app_icon;
use crate::utils::autostart::set_autostart;

const WIN_W: f32 = 666.0;
const WIN_H: f32 = 666.0;
const SIDEBAR_W: f32 = 180.0;
const SIDEBAR_ROW_H: f32 = 32.0;
const CONTENT_START_Y: f32 = 10.0;
const SUB_TAB_H: f32 = 40.0;
const SUB_TAB_START_Y: f32 = 50.0;

#[derive(Clone, PartialEq)]
enum PopupKind {
    LyricsSource,
    Language,
    Monitor,
    IslandStyle,
    SettingsTheme,
    MiniCoverShape,
    ExpandedCoverShape,
}

struct PopupState {
    kind: PopupKind,
    button_rect: Rect,
    options: Vec<String>,
    values: Vec<String>,
    selected_idx: usize,
    hover_idx: Option<usize>,
}

impl PopupState {
    fn menu_rect(&self) -> Rect {
        let item_count = self.options.len() as f32;
        let menu_h = POPUP_MENU_PAD * 2.0 + item_count * POPUP_ITEM_H;
        let fm = FontManager::global();
        let mut max_text_w: f32 = self.button_rect.width();
        for opt in &self.options {
            let (w, _) = fm.measure(opt, 12.0, false);
            let needed = w + 36.0;
            if needed > max_text_w { max_text_w = needed; }
        }
        let menu_w = max_text_w;
        let right_edge = self.button_rect.right;
        let menu_x = right_edge - menu_w;
        Rect::from_xywh(
            menu_x,
            self.button_rect.bottom + 2.0,
            menu_w,
            menu_h,
        )
    }

    fn item_rect(&self, idx: usize) -> Rect {
        let menu = self.menu_rect();
        Rect::from_xywh(
            menu.left + POPUP_MENU_PAD,
            menu.top + POPUP_MENU_PAD + idx as f32 * POPUP_ITEM_H,
            menu.width() - POPUP_MENU_PAD * 2.0,
            POPUP_ITEM_H,
        )
    }
}

pub struct SettingsApp {
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    sk_surface: Option<skia_safe::Surface>,
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
    detected_apps: Vec<String>,
    sidebar_hover: i32,
    popup: Option<PopupState>,
    hover_row: Option<usize>,
    total_rows: usize,
    is_light: bool,
    cached_items: Option<Vec<SettingsItem>>,
    items_dirty: bool,
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
            sk_surface: None,
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
            detected_apps: Vec::new(),
            sidebar_hover: -1,
            popup: None,
            hover_row: None,
            total_rows: 0,
            is_light: false,
            cached_items: None,
            items_dirty: true,
            win_w: WIN_W,
            win_h: WIN_H,
            max_scroll: 0.0,
        }
    }

    fn theme(&self) -> SettingsTheme {
        if self.is_light { light_settings_theme() } else { dark_settings_theme() }
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
        if let Some(win) = &self.window { win.request_redraw(); }
    }

    fn build_general_items(&self) -> Vec<SettingsItem> {
        let mut items: Vec<SettingsItem> = vec![
            SettingsItem::PageTitle { text: tr("tab_general") },
        ];

        match self.active_sub_page {
            0 => {
                items.push(SettingsItem::SectionHeader { label: tr("section_appearance") });
                items.push(SettingsItem::GroupStart);
                items.push(SettingsItem::RowStepper { label: tr("global_scale"), value: format!("{:.2}", self.config.global_scale), enabled: true });
                items.push(SettingsItem::RowStepper { label: tr("base_width"), value: self.config.base_width.to_string(), enabled: true });
                items.push(SettingsItem::RowStepper { label: tr("base_height"), value: self.config.base_height.to_string(), enabled: true });
                items.push(SettingsItem::RowStepper { label: tr("expanded_width"), value: self.config.expanded_width.to_string(), enabled: true });
                items.push(SettingsItem::RowStepper { label: tr("expanded_height"), value: self.config.expanded_height.to_string(), enabled: true });
                items.push(SettingsItem::RowStepper { label: tr("position_x_offset"), value: self.config.position_x_offset.to_string(), enabled: true });
                items.push(SettingsItem::RowStepper { label: tr("position_y_offset"), value: self.config.position_y_offset.to_string(), enabled: true });
                items.push(SettingsItem::RowStepper { label: tr("font_size"), value: format!("{:.0}", self.config.font_size), enabled: true });
                {
                    let monitors = self.get_monitor_list();
                    let selected_idx = (self.config.monitor_index as usize).min(monitors.len().saturating_sub(1));
                    let options: Vec<(String, bool)> = monitors.iter().enumerate()
                        .map(|(i, name)| (name.clone(), i == selected_idx))
                        .collect();
                    items.push(SettingsItem::RowSourceSelect {
                        label: tr("monitor"),
                        options,
                        enabled: true,
                    });
                }
                items.push(SettingsItem::GroupEnd);
            }
            1 => {
                items.push(SettingsItem::SectionHeader { label: tr("section_effects") });
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
                        (tr("shape_square"), self.config.expanded_cover_shape == "square"),
                        (tr("shape_circle"), self.config.expanded_cover_shape == "circle"),
                    ],
                    enabled: true,
                });
                items.push(SettingsItem::RowSwitch { label: tr("adaptive_border"), on: self.config.adaptive_border, enabled: true });
                items.push(SettingsItem::RowSwitch { label: tr("motion_blur"), on: self.config.motion_blur, enabled: true });
                items.push(SettingsItem::RowSwitch { label: tr("cover_rotate"), on: self.config.cover_rotate, enabled: true });
                items.push(SettingsItem::RowSwitch { label: tr("audio_gate"), on: self.config.audio_gate, enabled: true });
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
                    reset_label: if self.config.custom_font_path.is_some() { Some(tr("font_reset")) } else { None },
                });
                items.push(SettingsItem::FontPreview {
                    has_custom_font: self.config.custom_font_path.is_some(),
                });
                items.push(SettingsItem::GroupEnd);
            }
            2 => {
                items.push(SettingsItem::SectionHeader { label: tr("section_behavior") });
                items.push(SettingsItem::GroupStart);
                items.push(SettingsItem::RowSwitch { label: tr("start_boot"), on: self.config.auto_start, enabled: true });
                items.push(SettingsItem::RowSwitch { label: tr("auto_hide"), on: self.config.auto_hide, enabled: true });
                if self.config.auto_hide {
                    items.push(SettingsItem::RowStepper { label: tr("hide_delay"), value: format!("{:.0}", self.config.auto_hide_delay), enabled: true });
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

                items.push(SettingsItem::SectionHeader { label: tr("section_updates") });
                items.push(SettingsItem::GroupStart);
                items.push(SettingsItem::RowSwitch { label: tr("check_updates"), on: self.config.check_for_updates, enabled: true });
                if self.config.check_for_updates {
                    items.push(SettingsItem::RowStepper { label: tr("update_interval"), value: format!("{:.0}", self.config.update_check_interval), enabled: true });
                }
                items.push(SettingsItem::GroupEnd);

                items.push(SettingsItem::Spacer { height: 10.0 });
                items.push(SettingsItem::CenterLink { label: tr("reset_defaults"), color: COLOR_DANGER });
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
            SettingsItem::PageTitle { text: tr("tab_music") },
            SettingsItem::SectionHeader { label: tr("section_playback") },
            SettingsItem::GroupStart,
            SettingsItem::RowSwitch { label: tr("smtc_control"), on: self.config.smtc_enabled, enabled: true },
            SettingsItem::GroupEnd,
            SettingsItem::SectionHeader { label: tr("section_lyrics") },
            SettingsItem::GroupStart,
            SettingsItem::RowSwitch { label: tr("show_lyrics"), on: self.config.show_lyrics, enabled: true },
            SettingsItem::RowSourceSelect {
                label: tr("lyrics_source"),
                options: vec![
                    ("163".to_string(), source == "163"),
                    ("LRCLIB".to_string(), source == "lrclib"),
                ],
                enabled: show_lyrics,
            },
            SettingsItem::RowSwitch { label: tr("lyrics_fallback"), on: if show_lyrics { self.config.lyrics_fallback } else { false }, enabled: show_lyrics },
            SettingsItem::RowStepper { label: tr("lyrics_delay"), value: format!("{:.1}", self.config.lyrics_delay), enabled: show_lyrics },
            SettingsItem::RowSwitch { label: tr("lyrics_scroll"), on: if show_lyrics { self.config.lyrics_scroll } else { false }, enabled: show_lyrics },
            SettingsItem::RowStepper { label: tr("lyrics_scroll_max_width"), value: format!("{}", self.config.lyrics_scroll_max_width as i32), enabled: show_lyrics && self.config.lyrics_scroll },
            SettingsItem::GroupEnd,
            SettingsItem::SectionHeader { label: tr("media_apps") },
            SettingsItem::GroupStart,
        ];

        if self.detected_apps.is_empty() {
            items.push(SettingsItem::RowLabel { label: tr("no_sessions") });
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
        vec![
            SettingsItem::PageTitle { text: tr("tab_about") },
            SettingsItem::Spacer { height: 20.0 },
            SettingsItem::CenterText { text: "WinIsland".to_string(), size: 28.0, color: COLOR_TEXT_PRI },
            SettingsItem::CenterText { text: format!("Version {}", APP_VERSION), size: 14.0, color: COLOR_TEXT_SEC },
            SettingsItem::CenterText { text: format!("{} {}", tr("created_by"), APP_AUTHOR), size: 14.0, color: COLOR_TEXT_SEC },
            SettingsItem::Spacer { height: 10.0 },
            SettingsItem::CenterLink { label: tr("visit_homepage"), color: COLOR_ACCENT },
        ]
    }

    fn build_current_items(&mut self) -> Vec<SettingsItem> {
        if self.items_dirty || self.cached_items.is_none() {
            let items = match self.active_page {
                0 => self.build_general_items(),
                1 => self.build_music_items(),
                2 => self.build_about_items(),
                _ => vec![],
            };
            self.cached_items = Some(items);
            self.items_dirty = false;
        }
        self.cached_items.as_ref().unwrap().clone()
    }
    
    fn mark_items_dirty(&mut self) {
        self.items_dirty = true;
    }

    fn get_monitor_list(&self) -> Vec<String> {
        use windows::Win32::Graphics::Gdi::*;
        let mut monitors: Vec<String> = Vec::new();
        unsafe {
            let mut idx = 0u32;
            loop {
                let mut dd: DISPLAY_DEVICEW = std::mem::zeroed();
                dd.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
                if EnumDisplayDevicesW(None, idx, &mut dd, 0).as_bool() {
                    if (dd.StateFlags & DISPLAY_DEVICE_ACTIVE) != 0 {
                        let name = String::from_utf16_lossy(&dd.DeviceName).trim_end_matches('\0').to_string();
                        let mut dm: DISPLAY_DEVICEW = std::mem::zeroed();
                        dm.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
                        let label = if EnumDisplayDevicesW(
                            windows::core::PCWSTR(dd.DeviceName.as_ptr()),
                            0, &mut dm, 0
                        ).as_bool() {
                            let friendly = String::from_utf16_lossy(&dm.DeviceString).trim_end_matches('\0').to_string();
                            if friendly.is_empty() { name.clone() } else { friendly }
                        } else {
                            name.clone()
                        };
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
        self.switch_anim.set_target(4, self.config.auto_start);
        self.switch_anim.set_target(5, self.config.auto_hide);
        self.switch_anim.set_target(6, self.config.check_for_updates);
        self.switch_anim.set_target(7, self.config.smtc_enabled);
        self.switch_anim.set_target(8, self.config.show_lyrics);
        let fb_on = if self.config.show_lyrics { self.config.lyrics_fallback } else { false };
        self.switch_anim.set_target(9, fb_on);
        let fw_on = if self.config.show_lyrics { self.config.lyrics_scroll } else { false };
        self.switch_anim.set_target(10, fw_on);
    }

    fn update_detected_apps(&mut self) {
        use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager;
        if let Ok(manager_async) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
            if let Ok(manager) = manager_async.get() {
                if let Ok(sessions) = manager.GetSessions() {
                    if let Ok(size) = sessions.Size() {
                        for i in 0..size {
                            if let Ok(session) = sessions.GetAt(i) {
                                if let Ok(id) = session.SourceAppUserModelId() {
                                    let name = id.to_string();
                                    if !self.detected_apps.contains(&name) {
                                        self.detected_apps.push(name);
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
            }
        }
    }

    fn draw(&mut self) {
        let win = self.window.as_ref().unwrap();
        let size = win.inner_size();
        let p_w = size.width as i32;
        let p_h = size.height as i32;
        if p_w <= 0 || p_h <= 0 { return; }

        let theme = self.theme();
        let scale = win.scale_factor() as f32;
        let win_w = self.win_w / scale;
        let win_h = self.win_h / scale;

        let mut sk_surface = if let Some(ref s) = self.sk_surface {
            if s.width() == p_w && s.height() == p_h {
                s.clone()
            } else {
                let new_s = surfaces::raster_n32_premul(skia_safe::ISize::new(p_w, p_h)).unwrap();
                self.sk_surface = Some(new_s.clone());
                new_s
            }
        } else {
            let new_s = surfaces::raster_n32_premul(skia_safe::ISize::new(p_w, p_h)).unwrap();
            self.sk_surface = Some(new_s.clone());
            new_s
        };

        let canvas = sk_surface.canvas();
        canvas.reset_matrix();
        canvas.clear(theme.win_bg);
        canvas.scale((scale, scale));

        self.draw_sidebar(canvas, &theme);

        let content_w = win_w - SIDEBAR_W;
        self.draw_sub_tabs(canvas, &theme, content_w);
        
        let items = self.build_current_items();
        let anim = self.get_page_anim();

        let content_start_y = if self.active_page == 0 {
            SUB_TAB_START_Y + SUB_TAB_H + CONTENT_START_Y
        } else {
            CONTENT_START_Y
        };

        let total_h = content_height(&items, content_start_y);
        self.max_scroll = (total_h - win_h + 20.0).max(0.0);
        self.target_scroll_y = self.target_scroll_y.clamp(0.0, self.max_scroll);
        self.scroll_y += (self.target_scroll_y - self.scroll_y) * 0.2;
        if (self.scroll_y - self.target_scroll_y).abs() < 0.5 {
            self.scroll_y = self.target_scroll_y;
        }

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
        draw_items(canvas, &items, content_start_y, content_w, &anim, &self.anim, &theme);
        canvas.restore();

        self.draw_popup(canvas, &theme);

        if let Some(surface) = self.surface.as_mut() {
            let mut buffer = surface.buffer_mut().unwrap();
            let info = skia_safe::ImageInfo::new(
                skia_safe::ISize::new(p_w, p_h),
                skia_safe::ColorType::BGRA8888,
                skia_safe::AlphaType::Premul,
                None,
            );
            let dst_row_bytes = (p_w * 4) as usize;
            let u8_buffer: &mut [u8] = bytemuck::cast_slice_mut(&mut *buffer);
            let _ = sk_surface.read_pixels(&info, u8_buffer, dst_row_bytes, (0, 0));
            buffer.present().unwrap();
        }
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
                    SIDEBAR_SEL_RADIUS, SIDEBAR_SEL_RADIUS, &paint,
                );
                paint.set_color(theme.text_pri);
            } else {
                let hover_val = self.anim.get(&format!("sidebar_{}", i));
                if hover_val > 0.005 {
                    let base = theme.sidebar_hover;
                    let alpha = (base.a() as f32 * hover_val) as u8;
                    paint.set_color(Color::from_argb(alpha, base.r(), base.g(), base.b()));
                    canvas.draw_round_rect(
                        Rect::from_xywh(row_x, row_y, row_w, SIDEBAR_ROW_H),
                        SIDEBAR_SEL_RADIUS, SIDEBAR_SEL_RADIUS, &paint,
                    );
                }
                paint.set_color(theme.text_sec);
            }

            fm.draw_text(canvas, label, (row_x + 12.0, row_y + 21.0), 13.0, false, &paint);
        }
    }

    fn draw_sub_tabs(&self, canvas: &skia_safe::Canvas, theme: &SettingsTheme, content_w: f32) {
        if self.active_page != 0 { return; }
        
        let fm = FontManager::global();
        let tabs = [tr("section_appearance"), tr("section_effects"), tr("section_behavior")];
        let tab_w = content_w / 3.0;
        let start_x = SIDEBAR_W;
        
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        
        let mut sep = Paint::default();
        sep.set_anti_alias(true);
        sep.set_color(theme.separator);
        sep.set_stroke_width(0.5);
        sep.set_style(skia_safe::paint::Style::Stroke);
        canvas.draw_line((SIDEBAR_W, SUB_TAB_START_Y + SUB_TAB_H), (SIDEBAR_W + content_w, SUB_TAB_START_Y + SUB_TAB_H), &sep);
        
        for (i, label) in tabs.iter().enumerate() {
            let tab_x = start_x + i as f32 * tab_w;
            let is_active = self.active_sub_page == i;
            let is_hover = self.sub_tab_hover == i as i32;
            
            if is_active {
                paint.set_color(theme.text_pri);
                let underline_x = tab_x + tab_w / 2.0 - 30.0;
                let underline_y = SUB_TAB_START_Y + SUB_TAB_H - 2.0;
                paint.set_style(skia_safe::paint::Style::Fill);
                canvas.draw_rect(
                    Rect::from_xywh(underline_x, underline_y, 60.0, 2.0),
                    &paint,
                );
            } else if is_hover {
                paint.set_color(theme.text_pri);
            } else {
                paint.set_color(theme.text_sec);
            }
            
            let text_x = tab_x + tab_w / 2.0;
            let text_y = SUB_TAB_START_Y + SUB_TAB_H / 2.0 + 5.0;
            fm.draw_text(canvas, label, (text_x - 30.0, text_y), 13.0, false, &paint);
        }
    }

    fn draw_popup(&self, canvas: &skia_safe::Canvas, theme: &SettingsTheme) {
        let popup = match &self.popup {
            Some(p) => p,
            None => return,
        };
        let opacity = self.anim.get("popup_opacity");
        if opacity < 0.005 { return; }
        let fm = FontManager::global();
        let menu = popup.menu_rect();

        let mut shadow = Paint::default();
        shadow.set_anti_alias(true);
        shadow.set_color(Color::from_argb((60.0 * opacity) as u8, 0, 0, 0));
        canvas.draw_round_rect(
            Rect::from_xywh(menu.left - 1.0, menu.top + 2.0, menu.width() + 2.0, menu.height() + 2.0),
            POPUP_MENU_R, POPUP_MENU_R, &shadow,
        );

        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_color(Color::from_argb((255.0 * opacity) as u8, theme.popup_bg.r(), theme.popup_bg.g(), theme.popup_bg.b()));
        canvas.draw_round_rect(menu, POPUP_MENU_R, POPUP_MENU_R, &paint);

        let mut border = Paint::default();
        border.set_anti_alias(true);
        border.set_color(Color::from_argb((40.0 * opacity) as u8, theme.popup_border.r(), theme.popup_border.g(), theme.popup_border.b()));
        border.set_style(skia_safe::paint::Style::Stroke);
        border.set_stroke_width(0.5);
        canvas.draw_round_rect(menu, POPUP_MENU_R, POPUP_MENU_R, &border);

        let text_alpha = (255.0 * opacity) as u8;
        for (i, opt_label) in popup.options.iter().enumerate() {
            let item_rect = popup.item_rect(i);

            if popup.hover_idx == Some(i) {
                let a = theme.accent.a() as f32 * opacity;
                paint.set_color(Color::from_argb(a as u8, theme.accent.r(), theme.accent.g(), theme.accent.b()));
                paint.set_style(skia_safe::paint::Style::Fill);
                canvas.draw_round_rect(item_rect, 4.0, 4.0, &paint);
            }

            paint.set_color(Color::from_argb(text_alpha, theme.text_pri.r(), theme.text_pri.g(), theme.text_pri.b()));
            paint.set_style(skia_safe::paint::Style::Fill);
            fm.draw_text(canvas, opt_label, (item_rect.left + 8.0, item_rect.top + 19.0), 12.0, false, &paint);

            if i == popup.selected_idx {
                let check_base = if popup.hover_idx == Some(i) { theme.text_pri } else { theme.accent };
                paint.set_color(Color::from_argb(text_alpha, check_base.r(), check_base.g(), check_base.b()));
                paint.set_style(skia_safe::paint::Style::Stroke);
                paint.set_stroke_width(2.0);
                let cx = item_rect.right - 14.0;
                let cy = item_rect.top + POPUP_ITEM_H / 2.0;
                let svg = format!(
                    "M {} {} L {} {} L {} {}",
                    cx - 4.0, cy, cx - 1.0, cy + 3.0, cx + 4.0, cy - 3.0,
                );
                if let Some(path) = skia_safe::Path::from_svg(&svg) {
                    canvas.draw_path(&path, &paint);
                }
                paint.set_style(skia_safe::paint::Style::Fill);
            }

            if i < popup.options.len() - 1 {
                let mut sep = Paint::default();
                sep.set_anti_alias(true);
                sep.set_color(Color::from_argb((30.0 * opacity) as u8, theme.separator.r(), theme.separator.g(), theme.separator.b()));
                sep.set_stroke_width(0.5);
                sep.set_style(skia_safe::paint::Style::Stroke);
                canvas.draw_line((item_rect.left, item_rect.bottom), (item_rect.right, item_rect.bottom), &sep);
            }
        }
    }

    fn get_page_anim(&self) -> SwitchAnimator {
        match self.active_page {
            0 => {
                match self.active_sub_page {
                    0 => SwitchAnimator::new(&[]),
                    1 => SwitchAnimator::new_with_anims(&self.switch_anim, &[0, 1, 2]),
                    2 => SwitchAnimator::new_with_anims(&self.switch_anim, &[4, 5, 6]),
                    _ => SwitchAnimator::new(&[]),
                }
            }
            1 => {
                SwitchAnimator::new_with_anims(&self.switch_anim, &[7, 8, 9, 10])
            }
            _ => SwitchAnimator::new(&[]),
        }
    }

    fn handle_click(&mut self) {
        let (mx, my) = self.logical_mouse_pos;

        if let Some(popup) = &self.popup {
            let menu = popup.menu_rect();
            if mx >= menu.left && mx <= menu.right && my >= menu.top && my <= menu.bottom {
                for i in 0..popup.options.len() {
                    let ir = popup.item_rect(i);
                    if my >= ir.top && my <= ir.bottom {
                        let value = popup.values[i].clone();
                        match popup.kind {
                            PopupKind::LyricsSource => {
                                self.config.lyrics_source = value;
                            }
                            PopupKind::Language => {
                                self.config.language = value.clone();
                                set_lang(&value);
                            }
                            PopupKind::Monitor => {
                                self.config.monitor_index = value.parse::<i32>().unwrap_or(0);
                            }
                            PopupKind::IslandStyle => {
                                self.config.island_style = value;
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
                        break;
                    }
                }
            }
            self.popup = None;
            self.anim.set_with_speed("popup_opacity", 0.0, 0.3);
            if let Some(win) = &self.window { win.request_redraw(); }
            return;
        }

        if mx < SIDEBAR_W {
            let pages = 3;
            let start_y = 20.0;
            for i in 0..pages {
                let row_y = start_y + i as f32 * (SIDEBAR_ROW_H + 2.0);
                if my >= row_y && my <= row_y + SIDEBAR_ROW_H && mx >= SIDEBAR_PAD && mx <= SIDEBAR_W - SIDEBAR_PAD {
                    if self.active_page != i as usize {
                        self.active_page = i as usize;
                        self.scroll_y = 0.0;
                        self.target_scroll_y = 0.0;
                        self.mark_items_dirty();
                        if let Some(win) = &self.window { win.request_redraw(); }
                    }
                    return;
                }
            }
            return;
        }

        let scale = self.window.as_ref().unwrap().scale_factor() as f32;
        let content_w = self.win_w / scale - SIDEBAR_W;

        if self.active_page == 0 && my >= SUB_TAB_START_Y && my <= SUB_TAB_START_Y + SUB_TAB_H {
            let tab_w = content_w / 3.0;
            let rel_x = mx - SIDEBAR_W;
            let tab_idx = (rel_x / tab_w) as usize;
            if tab_idx < 3 {
                if self.active_sub_page != tab_idx {
                    self.active_sub_page = tab_idx;
                    self.scroll_y = 0.0;
                    self.target_scroll_y = 0.0;
                    self.mark_items_dirty();
                    if let Some(win) = &self.window { win.request_redraw(); }
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
            0 => self.handle_general_click(&items, content_x, content_y, content_w, content_start_y),
            1 => self.handle_music_click(&items, content_x, content_y, content_w, content_start_y),
            2 => self.handle_about_click(&items, content_x, content_y, content_w, content_start_y),
            _ => {}
        }
    }

    fn handle_general_click(&mut self, items: &[SettingsItem], mx: f32, my: f32, width: f32, start_y: f32) {
        let result = hit_test(items, mx, my, start_y, width);
        let mut changed = false;

        match result {
            ClickResult::StepperDec(idx) | ClickResult::StepperInc(idx) => {
                let is_dec = matches!(result, ClickResult::StepperDec(_));
                if let Some(item) = get_row_item(items, idx) {
                    if let SettingsItem::RowStepper { label, .. } = item {
                        let l = label.clone();
                        if l == tr("global_scale") {
                            if is_dec { self.config.global_scale = ((self.config.global_scale - 0.05) * 100.0).round() / 100.0; self.config.global_scale = self.config.global_scale.max(0.5); }
                            else { self.config.global_scale = ((self.config.global_scale + 0.05) * 100.0).round() / 100.0; self.config.global_scale = self.config.global_scale.min(5.0); }
                            changed = true;
                        } else if l == tr("base_width") {
                            if is_dec { self.config.base_width -= 5.0; } else { self.config.base_width += 5.0; }
                            changed = true;
                        } else if l == tr("base_height") {
                            if is_dec { self.config.base_height -= 2.0; } else { self.config.base_height += 2.0; }
                            changed = true;
                        } else if l == tr("expanded_width") {
                            if is_dec { self.config.expanded_width -= 10.0; } else { self.config.expanded_width += 10.0; }
                            changed = true;
                        } else if l == tr("expanded_height") {
                            if is_dec { self.config.expanded_height -= 10.0; } else { self.config.expanded_height += 10.0; }
                            changed = true;
                        } else if l == tr("position_x_offset") {
                            if is_dec { self.config.position_x_offset -= 5; } else { self.config.position_x_offset += 5; }
                            changed = true;
                        } else if l == tr("position_y_offset") {
                            if is_dec { self.config.position_y_offset -= 5; } else { self.config.position_y_offset += 5; }
                            changed = true;
                        } else if l == tr("font_size") {
                            if is_dec { self.config.font_size = (self.config.font_size - 1.0).max(0.0); }
                            else { self.config.font_size = (self.config.font_size + 1.0).min(30.0); }
                            changed = true;
                        } else if l == tr("hide_delay") {
                            if is_dec { self.config.auto_hide_delay = (self.config.auto_hide_delay - 1.0).max(1.0); }
                            else { self.config.auto_hide_delay = (self.config.auto_hide_delay + 1.0).min(60.0); }
                            changed = true;
                        } else if l == tr("update_interval") {
                            if is_dec { self.config.update_check_interval = (self.config.update_check_interval - 1.0).max(1.0); }
                            else { self.config.update_check_interval = (self.config.update_check_interval + 1.0).min(24.0); }
                            changed = true;
                        }
                    }
                }
            }
            ClickResult::Switch(idx) => {
                let actual_idx = match self.active_sub_page {
                    1 => idx,
                    2 => idx + 4,
                    _ => idx,
                };
                match actual_idx {
                    0 => self.config.adaptive_border = !self.config.adaptive_border,
                    1 => self.config.motion_blur = !self.config.motion_blur,
                    2 => self.config.cover_rotate = !self.config.cover_rotate,
                    3 => self.config.audio_gate = !self.config.audio_gate,
                    4 => { self.config.auto_start = !self.config.auto_start; let _ = set_autostart(self.config.auto_start); }
                    5 => self.config.auto_hide = !self.config.auto_hide,
                    6 => self.config.check_for_updates = !self.config.check_for_updates,
                    _ => {}
                }
                self.sync_switch_targets();
                changed = true;
            }
            ClickResult::FontSelect(_) => {
                if let Some(path) = rfd::FileDialog::new().add_filter("Fonts", &["ttf", "otf"]).pick_file() {
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
                let mut row_count = 0;
                for item in items {
                    if row_count >= idx { break; }
                    btn_content_y += item.height();
                    if item.is_row() { row_count += 1; }
                }
                let cy = btn_content_y + ROW_HEIGHT / 2.0;
                let btn_x = SIDEBAR_W + CONTENT_PADDING + content_w - GROUP_INNER_PAD - POPUP_BTN_W;
                let btn_y = cy - POPUP_BTN_H / 2.0 - self.scroll_y;

                if let Some(SettingsItem::RowSourceSelect { label, .. }) = get_row_item(items, idx) {
                    if label == &tr("monitor") {
                        let monitors = self.get_monitor_list();
                        let selected_idx = (self.config.monitor_index as usize).min(monitors.len().saturating_sub(1));
                        let values: Vec<String> = (0..monitors.len()).map(|i| i.to_string()).collect();
                        self.popup = Some(PopupState {
                            kind: PopupKind::Monitor,
                            button_rect: Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            options: monitors,
                            values,
                            selected_idx,
                            hover_idx: None,
                        });
                    } else if label == &tr("island_style") {
                        let selected_idx = match self.config.island_style.as_str() {
                            "glass" => 1,
                            "mica" => 2,
                            "dynamic" => 3,
                            _ => 0,
                        };
                        self.popup = Some(PopupState {
                            kind: PopupKind::IslandStyle,
                            button_rect: Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            options: vec![tr("style_default"), tr("style_glass"), tr("style_mica"), tr("style_dynamic")],
                            values: vec!["default".to_string(), "glass".to_string(), "mica".to_string(), "dynamic".to_string()],
                            selected_idx,
                            hover_idx: None,
                        });
                    } else if label == &tr("settings_theme") {
                        let selected_idx = match self.config.settings_theme.as_str() {
                            "light" => 1,
                            "dark" => 2,
                            _ => 0,
                        };
                        self.popup = Some(PopupState {
                            kind: PopupKind::SettingsTheme,
                            button_rect: Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            options: vec![tr("theme_system"), tr("theme_light"), tr("theme_dark")],
                            values: vec!["system".to_string(), "light".to_string(), "dark".to_string()],
                            selected_idx,
                            hover_idx: None,
                        });
                    } else if label == &tr("mini_cover_shape") {
                        let selected_idx = if self.config.mini_cover_shape == "circle" { 1 } else { 0 };
                        self.popup = Some(PopupState {
                            kind: PopupKind::MiniCoverShape,
                            button_rect: Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            options: vec![tr("shape_square"), tr("shape_circle")],
                            values: vec!["square".to_string(), "circle".to_string()],
                            selected_idx,
                            hover_idx: None,
                        });
                    } else if label == &tr("expanded_cover_shape") {
                        let selected_idx = if self.config.expanded_cover_shape == "circle" { 1 } else { 0 };
                        self.popup = Some(PopupState {
                            kind: PopupKind::ExpandedCoverShape,
                            button_rect: Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            options: vec![tr("shape_square"), tr("shape_circle")],
                            values: vec!["square".to_string(), "circle".to_string()],
                            selected_idx,
                            hover_idx: None,
                        });
                    } else {
                        let lang = current_lang();
                        self.popup = Some(PopupState {
                            kind: PopupKind::Language,
                            button_rect: Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            options: vec!["English".to_string(), "中文".to_string()],
                            values: vec!["en".to_string(), "zh".to_string()],
                            selected_idx: if lang == "zh" { 1 } else { 0 },
                            hover_idx: None,
                        });
                    }
                    self.anim.set_with_speed("popup_opacity", 1.0, 0.25);
                    if let Some(win) = &self.window { win.request_redraw(); }
                }
            }
            ClickResult::CenterLink(_) => {
                self.config = AppConfig::default();
                set_lang(if self.config.language == "auto" { "en" } else { &self.config.language });
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
            if let Some(win) = &self.window { win.request_redraw(); }
        }
    }

    fn handle_music_click(&mut self, items: &[SettingsItem], mx: f32, my: f32, width: f32, start_y: f32) {
        let result = hit_test(items, mx, my, start_y, width);
        let mut changed = false;

        match result {
            ClickResult::Switch(idx) => {
                let actual_idx = idx + 7;
                match actual_idx {
                    7 => self.config.smtc_enabled = !self.config.smtc_enabled,
                    8 => self.config.show_lyrics = !self.config.show_lyrics,
                    9 => if self.config.show_lyrics { self.config.lyrics_fallback = !self.config.lyrics_fallback },
                    10 => if self.config.show_lyrics { self.config.lyrics_scroll = !self.config.lyrics_scroll },
                    _ => {}
                }
                self.sync_switch_targets();
                changed = true;
            }
            ClickResult::SourceButton(idx) => {
                let content_w = width;
                let mut btn_content_y = start_y;
                let mut row_count = 0;
                for item in items {
                    if row_count >= idx { break; }
                    btn_content_y += item.height();
                    if item.is_row() { row_count += 1; }
                }
                let cy = btn_content_y + ROW_HEIGHT / 2.0;
                let btn_x = SIDEBAR_W + CONTENT_PADDING + content_w - GROUP_INNER_PAD - POPUP_BTN_W;
                let btn_y = cy - POPUP_BTN_H / 2.0 - self.scroll_y;

                let source = &self.config.lyrics_source;
                self.popup = Some(PopupState {
                    kind: PopupKind::LyricsSource,
                    button_rect: Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                    options: vec!["163".to_string(), "LRCLIB".to_string()],
                    values: vec!["163".to_string(), "lrclib".to_string()],
                    selected_idx: if source == "163" { 0 } else { 1 },
                    hover_idx: None,
                });
                self.anim.set_with_speed("popup_opacity", 1.0, 0.25);
                if let Some(win) = &self.window { win.request_redraw(); }
            }
            ClickResult::StepperDec(idx) | ClickResult::StepperInc(idx) => {
                let is_dec = matches!(result, ClickResult::StepperDec(_));
                if let Some(item) = get_row_item(items, idx) {
                    if let SettingsItem::RowStepper { label, .. } = item {
                        if label == &tr("lyrics_delay") && self.config.show_lyrics {
                            if is_dec { self.config.lyrics_delay = ((self.config.lyrics_delay * 10.0 - 1.0).round() / 10.0).max(-10.0); }
                            else { self.config.lyrics_delay = ((self.config.lyrics_delay * 10.0 + 1.0).round() / 10.0).min(10.0); }
                            changed = true;
                        } else if label == &tr("lyrics_scroll_max_width") && self.config.show_lyrics && self.config.lyrics_scroll {
                            if is_dec { self.config.lyrics_scroll_max_width = (self.config.lyrics_scroll_max_width - 10.0).max(100.0); }
                            else { self.config.lyrics_scroll_max_width = (self.config.lyrics_scroll_max_width + 10.0).min(500.0); }
                            changed = true;
                        }
                    }
                }
            }
            ClickResult::AppItem(idx) => {
                if self.config.smtc_enabled && !self.detected_apps.is_empty() {
                    let mut row_count = 0;
                    let mut app_row_start = None;
                    for item in items {
                        if matches!(item, SettingsItem::RowAppItem { .. }) {
                            if app_row_start.is_none() {
                                app_row_start = Some(row_count);
                            }
                        }
                        if item.is_row() {
                            row_count += 1;
                        }
                    }
                    if let Some(start) = app_row_start {
                        let app_idx = idx - start;
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
                }
            }
            _ => {}
        }

        if changed {
            self.mark_items_dirty();
            save_config(&self.config);
            if let Some(win) = &self.window { win.request_redraw(); }
        }
    }

    fn handle_about_click(&mut self, items: &[SettingsItem], mx: f32, my: f32, width: f32, start_y: f32) {
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
                if my >= row_y && my <= row_y + SIDEBAR_ROW_H && mx >= SIDEBAR_PAD && mx <= SIDEBAR_W - SIDEBAR_PAD {
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
        let items = self.build_current_items();
        hover_test(&items, content_x, content_y, content_start_y, content_w)
    }
}

impl ApplicationHandler for SettingsApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let monitor = event_loop.primary_monitor().or_else(|| event_loop.available_monitors().next());
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
        surface.resize(
            std::num::NonZeroU32::new(size.width).unwrap(),
            std::num::NonZeroU32::new(size.height).unwrap(),
        ).unwrap();
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
                    if let Some(win) = &self.window { win.request_redraw(); }
                }
            }
            WindowEvent::Resized(new_size) => {
                self.win_w = new_size.width as f32;
                self.win_h = new_size.height as f32;
                if let Some(surface) = &mut self.surface {
                    surface.resize(
                        std::num::NonZeroU32::new(new_size.width).unwrap(),
                        std::num::NonZeroU32::new(new_size.height).unwrap(),
                    ).unwrap();
                    if let Some(win) = &self.window { win.request_redraw(); }
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let (Some(win), Some(surface)) = (&self.window, &mut self.surface) {
                    let size = win.inner_size();
                    surface.resize(
                        std::num::NonZeroU32::new(size.width).unwrap(),
                        std::num::NonZeroU32::new(size.height).unwrap(),
                    ).unwrap();
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
                                    self.mark_items_dirty();
                                    if let Some(win) = &self.window { win.request_redraw(); }
                                }
                            } else if self.active_page > 0 {
                                self.active_page -= 1;
                                self.scroll_y = 0.0;
                                self.target_scroll_y = 0.0;
                                self.mark_items_dirty();
                                if let Some(win) = &self.window { win.request_redraw(); }
                            }
                        }
                        Key::Named(NamedKey::ArrowRight) => {
                            if self.active_page == 0 {
                                if self.active_sub_page < 2 {
                                    self.active_sub_page += 1;
                                    self.scroll_y = 0.0;
                                    self.target_scroll_y = 0.0;
                                    self.mark_items_dirty();
                                    if let Some(win) = &self.window { win.request_redraw(); }
                                }
                            } else if self.active_page < 2 {
                                self.active_page += 1;
                                self.scroll_y = 0.0;
                                self.target_scroll_y = 0.0;
                                self.mark_items_dirty();
                                if let Some(win) = &self.window { win.request_redraw(); }
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
                    let menu = popup.menu_rect();
                    let mut new_hover = None;
                    if pmx >= menu.left && pmx <= menu.right && pmy >= menu.top && pmy <= menu.bottom {
                        for i in 0..popup.options.len() {
                            let ir = popup.item_rect(i);
                            if pmy >= ir.top && pmy <= ir.bottom {
                                new_hover = Some(i);
                                break;
                            }
                        }
                    }
                    if new_hover != popup.hover_idx {
                        popup.hover_idx = new_hover;
                        if let Some(win) = &self.window { win.request_redraw(); }
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
                            if my >= row_y && my <= row_y + SIDEBAR_ROW_H && mx >= SIDEBAR_PAD && mx <= SIDEBAR_W - SIDEBAR_PAD {
                                new_hover = i;
                            }
                        }
                    }
                    if new_hover != self.sidebar_hover {
                        self.sidebar_hover = new_hover;
                        for idx in 0..3 {
                            let key = format!("sidebar_{}", idx);
                            if idx == new_hover as usize {
                                self.anim.set(&key, 1.0);
                            } else {
                                self.anim.set(&key, 0.0);
                            }
                        }
                        if let Some(win) = &self.window { win.request_redraw(); }
                    }

                    let scale = self.window.as_ref().unwrap().scale_factor() as f32;
                    let content_w = self.win_w / scale - SIDEBAR_W;

                    if self.active_page == 0 && mx >= SIDEBAR_W && my >= SUB_TAB_START_Y && my <= SUB_TAB_START_Y + SUB_TAB_H {
                        let tab_w = content_w / 3.0;
                        let rel_x = mx - SIDEBAR_W;
                        let tab_idx = (rel_x / tab_w) as i32;
                        let new_sub_hover = if tab_idx >= 0 && tab_idx < 3 { tab_idx } else { -1 };
                        if new_sub_hover != self.sub_tab_hover {
                            self.sub_tab_hover = new_sub_hover;
                            if let Some(win) = &self.window { win.request_redraw(); }
                        }
                    } else if self.sub_tab_hover != -1 {
                        self.sub_tab_hover = -1;
                        if let Some(win) = &self.window { win.request_redraw(); }
                    }

                    if mx >= SIDEBAR_W {
                        let content_x = mx - SIDEBAR_W;
                        let content_start_y = if self.active_page == 0 {
                            SUB_TAB_START_Y + SUB_TAB_H + CONTENT_START_Y
                        } else {
                            CONTENT_START_Y
                        };
                        let content_y = my + self.scroll_y;
                        let items = self.build_current_items();
                        let mut item_y = content_start_y;
                        let mut new_row: Option<usize> = None;
                        let mut ri: usize = 0;
                        for item in &items {
                            if item.is_row() {
                                if content_y >= item_y && content_y <= item_y + ROW_HEIGHT
                                    && content_x >= CONTENT_PADDING && content_x <= content_w - CONTENT_PADDING {
                                    new_row = Some(ri);
                                }
                                ri += 1;
                            }
                            item_y += item.height();
                        }
                        self.total_rows = ri;
                        if new_row != self.hover_row {
                            if let Some(old) = self.hover_row {
                                self.anim.set(&format!("hover_row_{}", old), 0.0);
                            }
                            if let Some(new) = new_row {
                                self.anim.set(&format!("hover_row_{}", new), 1.0);
                            }
                            self.hover_row = new_row;
                        }
                    } else {
                        if self.hover_row.is_some() {
                            if let Some(old) = self.hover_row {
                                self.anim.set(&format!("hover_row_{}", old), 0.0);
                            }
                            self.hover_row = None;
                        }
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
            self.anim.set_with_speed("popup_opacity", 0.0, 0.3);
                    if let Some(win) = &self.window { win.request_redraw(); }
                    return;
                }
                let (mx, _) = self.logical_mouse_pos;
                if mx >= SIDEBAR_W {
                    let diff = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => y * 40.0,
                        winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                    };
                    self.target_scroll_y = (self.target_scroll_y - diff).clamp(0.0, self.max_scroll);
                    if let Some(win) = &self.window { win.request_redraw(); }
                }
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                self.handle_click();
            }
            WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Left, .. } => {}
            WindowEvent::RedrawRequested => self.draw(),
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        if self.window.is_none() { return; }
        
        self.frame_count += 1;
        if self.frame_count % 30 == 0 {
            unsafe {
                let h = OpenMutexW(MUTEX_ALL_ACCESS, false, w!("Local\\WinIsland_SingleInstance_Mutex"));
                if h.is_err() { _el.exit(); return; }
                let _ = windows::Win32::Foundation::CloseHandle(h.unwrap());
            }
        }

        let has_anim = self.switch_anim.is_animating() || self.anim.is_animating();
        let has_popup = self.popup.is_some();
        let is_scrolling = (self.target_scroll_y - self.scroll_y).abs() > 0.1;

        if !has_anim && !has_popup && !is_scrolling {
            return;
        }

        let mut redraw = self.switch_anim.tick();
        if self.anim.tick() { redraw = true; }

        let items = self.build_current_items();
        let ch = content_height(&items, CONTENT_START_Y);
        let view_h = self.win_h;
        let max_scroll = (ch - view_h).max(0.0);
        self.target_scroll_y = self.target_scroll_y.clamp(0.0, max_scroll);
        if is_scrolling {
            self.scroll_y += (self.target_scroll_y - self.scroll_y) * 0.35;
            redraw = true;
        } else if (self.scroll_y - self.target_scroll_y).abs() > f32::EPSILON {
            self.scroll_y = self.target_scroll_y;
        }

        if redraw {
            if let Some(win) = &self.window {
                win.request_redraw();
            }
        }
        std::thread::sleep(Duration::from_millis(16));
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
