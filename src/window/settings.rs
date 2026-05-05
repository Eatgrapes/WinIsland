use crate::core::config::{APP_AUTHOR, APP_HOMEPAGE, APP_VERSION, AppConfig};
use crate::core::i18n::{current_lang, init_i18n, set_lang, tr};
use crate::core::persistence::save_config;
use crate::utils::anim::AnimPool;
use crate::utils::autostart::set_autostart;
use crate::utils::color::*;
use crate::utils::font::FontManager;
use crate::utils::icon::get_app_icon;
use crate::utils::settings_ui::items::*;
use crate::utils::settings_ui::*;
use skia_safe::{Color, Paint, Rect, surfaces};
use softbuffer::{Context, Surface};
use std::sync::Arc;
use std::time::{Duration, Instant};
use windows::Win32::System::Threading::{MUTEX_ALL_ACCESS, OpenMutexW};
use windows::core::w;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowButtons, WindowId};

const WIN_W: f32 = 680.0;
const WIN_H: f32 = 480.0;
const SIDEBAR_W: f32 = 180.0;
const SIDEBAR_ROW_H: f32 = 32.0;
const CONTENT_START_Y: f32 = 10.0;

const POPUP_OPACITY_KEY: u64 = 1;
const SIDEBAR_KEY_BASE: u64 = 1_000;
const HOVER_ROW_KEY_BASE: u64 = 10_000;

#[derive(Clone, PartialEq)]
enum PopupKind {
    LyricsSource,
    Language,
    Monitor,
    IslandStyle,
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
            if needed > max_text_w {
                max_text_w = needed;
            }
        }
        let menu_w = max_text_w;
        let right_edge = self.button_rect.right;
        let menu_x = right_edge - menu_w;
        Rect::from_xywh(menu_x, self.button_rect.bottom + 2.0, menu_w, menu_h)
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
    config: AppConfig,
    active_page: usize,
    switch_anim: SwitchAnimator,
    anim: AnimPool,
    logical_mouse_pos: (f32, f32),
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
    items_dirty: bool,
    cached_items: Vec<SettingsItem>,
    cached_content_height: f32,
    cached_max_scroll: f32,
    cached_row_tops: Vec<f32>,
}

impl SettingsApp {
    pub fn new(config: AppConfig) -> Self {
        let switch_anim = SwitchAnimator::new(&[
            config.adaptive_border,
            config.motion_blur,
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
            switch_anim,
            anim: AnimPool::new(),
            logical_mouse_pos: (0.0, 0.0),
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
            items_dirty: true,
            cached_items: Vec::new(),
            cached_content_height: 0.0,
            cached_max_scroll: 0.0,
            cached_row_tops: Vec::new(),
        }
    }

    fn build_general_items(&self) -> Vec<SettingsItem> {
        let mut items: Vec<SettingsItem> = vec![
            SettingsItem::PageTitle {
                text: tr("tab_general"),
            },
            SettingsItem::SectionHeader {
                label: tr("section_appearance"),
            },
            SettingsItem::GroupStart,
            SettingsItem::RowStepper {
                label: tr("global_scale"),
                value: format!("{:.2}", self.config.global_scale),
                enabled: true,
            },
            SettingsItem::RowStepper {
                label: tr("base_width"),
                value: self.config.base_width.to_string(),
                enabled: true,
            },
            SettingsItem::RowStepper {
                label: tr("base_height"),
                value: self.config.base_height.to_string(),
                enabled: true,
            },
            SettingsItem::RowStepper {
                label: tr("expanded_width"),
                value: self.config.expanded_width.to_string(),
                enabled: true,
            },
            SettingsItem::RowStepper {
                label: tr("expanded_height"),
                value: self.config.expanded_height.to_string(),
                enabled: true,
            },
            SettingsItem::RowStepper {
                label: tr("position_x_offset"),
                value: self.config.position_x_offset.to_string(),
                enabled: true,
            },
            SettingsItem::RowStepper {
                label: tr("position_y_offset"),
                value: self.config.position_y_offset.to_string(),
                enabled: true,
            },
            SettingsItem::RowStepper {
                label: tr("font_size"),
                value: format!("{:.0}", self.config.font_size),
                enabled: true,
            },
        ];
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
        items.push(SettingsItem::GroupEnd);
        items.push(SettingsItem::SectionHeader {
            label: tr("section_effects"),
        });
        items.push(SettingsItem::GroupStart);
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
        items.push(SettingsItem::RowSourceSelect {
            label: tr("island_style"),
            options: vec![
                (tr("style_default"), self.config.island_style == "default"),
                (tr("style_glass"), self.config.island_style == "glass"),
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
        items.push(SettingsItem::GroupEnd);
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
                ("涓枃".to_string(), current_lang() == "zh"),
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
            color: COLOR_DANGER,
        });
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
                let display_name = app.split('!').next().unwrap_or(app);
                let active = self.config.smtc_apps.contains(app);
                items.push(SettingsItem::RowAppItem {
                    label: display_name.to_string(),
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
            SettingsItem::PageTitle {
                text: tr("tab_about"),
            },
            SettingsItem::Spacer { height: 20.0 },
            SettingsItem::CenterText {
                text: "WinIsland".to_string(),
                size: 28.0,
                color: COLOR_TEXT_PRI,
            },
            SettingsItem::CenterText {
                text: format!("Version {}", APP_VERSION),
                size: 14.0,
                color: COLOR_TEXT_SEC,
            },
            SettingsItem::CenterText {
                text: format!("{} {}", tr("created_by"), APP_AUTHOR),
                size: 14.0,
                color: COLOR_TEXT_SEC,
            },
            SettingsItem::Spacer { height: 10.0 },
            SettingsItem::CenterLink {
                label: tr("visit_homepage"),
                color: COLOR_ACCENT,
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
        self.cached_content_height = content_height(&self.cached_items, CONTENT_START_Y);
        self.cached_max_scroll = (self.cached_content_height - WIN_H).max(0.0);
        self.cached_row_tops.clear();
        let mut y = CONTENT_START_Y;
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

    fn get_monitor_list(&self) -> Vec<String> {
        use windows::Win32::Graphics::Gdi::*;
        let mut monitors: Vec<String> = Vec::new();
        unsafe {
            let mut idx = 0u32;
            let mut active_count = 0;
            loop {
                let mut dd: DISPLAY_DEVICEW = std::mem::zeroed();
                dd.cb = size_of::<DISPLAY_DEVICEW>() as u32;
                if EnumDisplayDevicesW(None, idx, &mut dd, 0).as_bool() {
                    if (dd.StateFlags & DISPLAY_DEVICE_ACTIVE) != 0 {
                        active_count += 1;
                        let name = String::from_utf16_lossy(&dd.DeviceName)
                            .trim_end_matches('\0')
                            .to_string();
                        let mut dm: DISPLAY_DEVICEW = std::mem::zeroed();
                        dm.cb = size_of::<DISPLAY_DEVICEW>() as u32;
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
        self.switch_anim.set_target(2, self.config.auto_start);
        self.switch_anim.set_target(3, self.config.auto_hide);
        self.switch_anim
            .set_target(4, self.config.check_for_updates);
        self.switch_anim.set_target(5, self.config.smtc_enabled);
        self.switch_anim.set_target(6, self.config.show_lyrics);
        let fb_on = if self.config.show_lyrics {
            self.config.lyrics_fallback
        } else {
            false
        };
        self.switch_anim.set_target(7, fb_on);
        let fw_on = if self.config.show_lyrics {
            self.config.lyrics_scroll
        } else {
            false
        };
        self.switch_anim.set_target(8, fw_on);
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
            (size.width as i32, size.height as i32, win.scale_factor() as f32)
        };
        if p_w <= 0 || p_h <= 0 {
            return;
        }
        self.ensure_items_cache();
        let anim = self.get_page_anim();
        let mut surface = match self.surface.take() {
            Some(s) => s,
            None => return,
        };

        {
            let mut buffer = surface.buffer_mut().unwrap();
            let info = skia_safe::ImageInfo::new(
                skia_safe::ISize::new(p_w, p_h),
                skia_safe::ColorType::BGRA8888,
                skia_safe::AlphaType::Premul,
                None,
            );
            let dst_row_bytes = (p_w * 4) as usize;
            let u8_buffer: &mut [u8] = bytemuck::cast_slice_mut(&mut *buffer);
            let mut sk_surface =
                surfaces::wrap_pixels(&info, u8_buffer, dst_row_bytes, None).unwrap();

            let canvas = sk_surface.canvas();
            canvas.reset_matrix();
            canvas.clear(COLOR_WIN_BG);
            canvas.scale((scale, scale));

            self.draw_sidebar(canvas);

            let content_w = WIN_W - SIDEBAR_W;
            canvas.save();
            canvas.clip_rect(
                Rect::from_xywh(SIDEBAR_W, 0.0, content_w, WIN_H),
                skia_safe::ClipOp::Intersect,
                true,
            );
            canvas.translate((SIDEBAR_W, -self.scroll_y));

            draw_items(
                canvas,
                &self.cached_items,
                CONTENT_START_Y,
                content_w,
                &anim,
                &self.anim,
                self.scroll_y,
                self.scroll_y + WIN_H,
            );
            canvas.restore();

            let ch = self.cached_content_height;
            let view_h = WIN_H;
            if ch > view_h {
                let bar_h = (view_h / ch) * view_h;
                let bar_y = (self.scroll_y / (ch - view_h)) * (view_h - bar_h);
                let mut p = Paint::default();
                p.set_anti_alias(true);
                p.set_color(Color::from_argb(60, 255, 255, 255));
                canvas.draw_round_rect(
                    Rect::from_xywh(WIN_W - 6.0, bar_y, 4.0, bar_h),
                    2.0,
                    2.0,
                    &p,
                );
            }

            self.draw_popup(canvas);
            buffer.present().unwrap();
        }

        self.surface = Some(surface);
    }

    fn draw_sidebar(&self, canvas: &skia_safe::Canvas) {
        let fm = FontManager::global();
        let mut paint = Paint::default();
        paint.set_anti_alias(true);

        paint.set_color(COLOR_SIDEBAR_BG);
        canvas.draw_rect(Rect::from_xywh(0.0, 0.0, SIDEBAR_W, WIN_H), &paint);

        let mut sep = Paint::default();
        sep.set_anti_alias(true);
        sep.set_color(color_separator());
        sep.set_stroke_width(0.5);
        sep.set_style(skia_safe::paint::Style::Stroke);
        canvas.draw_line((SIDEBAR_W, 0.0), (SIDEBAR_W, WIN_H), &sep);

        let pages = [tr("tab_general"), tr("tab_music"), tr("tab_about")];
        let start_y = 20.0;

        for (i, label) in pages.iter().enumerate() {
            let row_y = start_y + i as f32 * (SIDEBAR_ROW_H + 2.0);
            let row_x = SIDEBAR_PAD;
            let row_w = SIDEBAR_W - SIDEBAR_PAD * 2.0;

            if self.active_page == i {
                paint.set_color(color_sidebar_sel());
                canvas.draw_round_rect(
                    Rect::from_xywh(row_x, row_y, row_w, SIDEBAR_ROW_H),
                    SIDEBAR_SEL_RADIUS,
                    SIDEBAR_SEL_RADIUS,
                    &paint,
                );
                paint.set_color(COLOR_TEXT_PRI);
            } else {
                let hover_val = self.anim.get(SIDEBAR_KEY_BASE + i as u64);
                if hover_val > 0.005 {
                    let base = color_sidebar_hover();
                    let alpha = (base.a() as f32 * hover_val) as u8;
                    paint.set_color(Color::from_argb(alpha, base.r(), base.g(), base.b()));
                    canvas.draw_round_rect(
                        Rect::from_xywh(row_x, row_y, row_w, SIDEBAR_ROW_H),
                        SIDEBAR_SEL_RADIUS,
                        SIDEBAR_SEL_RADIUS,
                        &paint,
                    );
                }
                paint.set_color(COLOR_TEXT_SEC);
            }

            fm.draw_text(
                canvas,
                label,
                (row_x + 12.0, row_y + 21.0),
                13.0,
                false,
                &paint,
            );
        }
    }

    fn draw_popup(&self, canvas: &skia_safe::Canvas) {
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
        paint.set_color(Color::from_argb((255.0 * opacity) as u8, 50, 50, 52));
        canvas.draw_round_rect(menu, POPUP_MENU_R, POPUP_MENU_R, &paint);

        let mut border = Paint::default();
        border.set_anti_alias(true);
        border.set_color(Color::from_argb((40.0 * opacity) as u8, 255, 255, 255));
        border.set_style(skia_safe::paint::Style::Stroke);
        border.set_stroke_width(0.5);
        canvas.draw_round_rect(menu, POPUP_MENU_R, POPUP_MENU_R, &border);

        let text_alpha = (255.0 * opacity) as u8;
        for (i, opt_label) in popup.options.iter().enumerate() {
            let item_rect = popup.item_rect(i);

            if popup.hover_idx == Some(i) {
                let a = COLOR_ACCENT.a() as f32 * opacity;
                paint.set_color(Color::from_argb(
                    a as u8,
                    COLOR_ACCENT.r(),
                    COLOR_ACCENT.g(),
                    COLOR_ACCENT.b(),
                ));
                paint.set_style(skia_safe::paint::Style::Fill);
                canvas.draw_round_rect(item_rect, 4.0, 4.0, &paint);
            }

            paint.set_color(Color::from_argb(
                text_alpha,
                COLOR_TEXT_PRI.r(),
                COLOR_TEXT_PRI.g(),
                COLOR_TEXT_PRI.b(),
            ));
            paint.set_style(skia_safe::paint::Style::Fill);
            fm.draw_text(
                canvas,
                opt_label,
                (item_rect.left + 8.0, item_rect.top + 19.0),
                12.0,
                false,
                &paint,
            );

            if i == popup.selected_idx {
                let check_base = if popup.hover_idx == Some(i) {
                    COLOR_TEXT_PRI
                } else {
                    COLOR_ACCENT
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
                sep.set_color(Color::from_argb((30.0 * opacity) as u8, 255, 255, 255));
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
            0 => SwitchAnimator::new_with_anims(&self.switch_anim, &[0, 1, 2, 3, 4]),
            1 => SwitchAnimator::new_with_anims(&self.switch_anim, &[5, 6, 7, 8]),
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
                        }
                        save_config(&self.config);
                        self.items_dirty = true;
                        break;
                    }
                }
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
                        self.items_dirty = true;
                        if let Some(win) = &self.window {
                            win.request_redraw();
                        }
                    }
                    return;
                }
            }
            return;
        }

        let content_x = mx - SIDEBAR_W;
        let content_y = my + self.scroll_y;
        let content_w = WIN_W - SIDEBAR_W;
        let items = self.build_current_items();

        match self.active_page {
            0 => self.handle_general_click(&items, content_x, content_y, content_w),
            1 => self.handle_music_click(&items, content_x, content_y, content_w),
            2 => self.handle_about_click(&items, content_x, content_y, content_w),
            _ => {}
        }
    }

    fn handle_general_click(&mut self, items: &[SettingsItem], mx: f32, my: f32, width: f32) {
        let result = hit_test(items, mx, my, CONTENT_START_Y, width);
        let mut changed = false;

        match result {
            ClickResult::StepperDec(idx) | ClickResult::StepperInc(idx) => {
                let is_dec = matches!(result, ClickResult::StepperDec(_));
                if let Some(item) = items.get(idx) {
                    if let SettingsItem::RowStepper { label, .. } = item {
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
            }
            ClickResult::Switch(idx) => {
                match idx {
                    0 => self.config.adaptive_border = !self.config.adaptive_border,
                    1 => self.config.motion_blur = !self.config.motion_blur,
                    2 => {
                        self.config.auto_start = !self.config.auto_start;
                        let _ = set_autostart(self.config.auto_start);
                    }
                    3 => self.config.auto_hide = !self.config.auto_hide,
                    4 => self.config.check_for_updates = !self.config.check_for_updates,
                    _ => {}
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
                let mut btn_content_y = CONTENT_START_Y;
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
                        self.popup = Some(PopupState {
                            kind: PopupKind::Monitor,
                            button_rect: Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            options: monitors,
                            values,
                            selected_idx,
                            hover_idx: None,
                        });
                    } else if label == &tr("island_style") {
                        let selected_idx = if self.config.island_style == "glass" {
                            1
                        } else {
                            0
                        };
                        self.popup = Some(PopupState {
                            kind: PopupKind::IslandStyle,
                            button_rect: Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            options: vec![tr("style_default"), tr("style_glass")],
                            values: vec!["default".to_string(), "glass".to_string()],
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
            save_config(&self.config);
            self.items_dirty = true;
            if let Some(win) = &self.window {
                win.request_redraw();
            }
        }
    }

    fn handle_music_click(&mut self, items: &[SettingsItem], mx: f32, my: f32, width: f32) {
        let result = hit_test(items, mx, my, CONTENT_START_Y, width);
        let mut changed = false;

        match result {
            ClickResult::Switch(idx) => {
                match idx {
                    0 => self.config.smtc_enabled = !self.config.smtc_enabled,
                    1 => self.config.show_lyrics = !self.config.show_lyrics,
                    2 => {
                        if self.config.show_lyrics {
                            self.config.lyrics_fallback = !self.config.lyrics_fallback
                        }
                    }
                    3 => {
                        if self.config.show_lyrics {
                            self.config.lyrics_scroll = !self.config.lyrics_scroll
                        }
                    }
                    _ => {}
                }
                self.sync_switch_targets();
                changed = true;
            }
            ClickResult::SourceButton(idx) => {
                let content_w = width;
                let mut btn_content_y = CONTENT_START_Y;
                for item in items.iter().take(idx) {
                    btn_content_y += item.height();
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
                self.anim.set_with_speed(POPUP_OPACITY_KEY, 1.0, 0.25);
                if let Some(win) = &self.window {
                    win.request_redraw();
                }
            }
            ClickResult::StepperDec(idx) | ClickResult::StepperInc(idx) => {
                let is_dec = matches!(result, ClickResult::StepperDec(_));
                if let Some(item) = items.get(idx) {
                    if let SettingsItem::RowStepper { label, .. } = item {
                        if label == &tr("lyrics_delay") && self.config.show_lyrics {
                            if is_dec {
                                self.config.lyrics_delay =
                                    ((self.config.lyrics_delay * 10.0 - 1.0).round() / 10.0)
                                        .max(-10.0);
                            } else {
                                self.config.lyrics_delay =
                                    ((self.config.lyrics_delay * 10.0 + 1.0).round() / 10.0)
                                        .min(10.0);
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
            }
            ClickResult::AppItem(idx) => {
                if self.config.smtc_enabled && !self.detected_apps.is_empty() {
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
            }
            _ => {}
        }

        if changed {
            save_config(&self.config);
            self.items_dirty = true;
            if let Some(win) = &self.window {
                win.request_redraw();
            }
        }
    }

    fn handle_about_click(&mut self, items: &[SettingsItem], mx: f32, my: f32, width: f32) {
        let result = hit_test(items, mx, my, CONTENT_START_Y, width);
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

        let content_x = mx - SIDEBAR_W;
        let content_y = my + self.scroll_y;
        let content_w = WIN_W - SIDEBAR_W;
        self.ensure_items_cache();
        hover_test(
            &self.cached_items,
            content_x,
            content_y,
            CONTENT_START_Y,
            content_w,
        )
    }
}

impl ApplicationHandler for SettingsApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title("Settings")
            .with_inner_size(LogicalSize::new(WIN_W as f64, WIN_H as f64))
            .with_resizable(false)
            .with_enabled_buttons(WindowButtons::CLOSE | WindowButtons::MINIMIZE)
            .with_window_icon(get_app_icon());
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        self.window = Some(window.clone());
        let context = Context::new(window.clone()).unwrap();
        let mut surface = Surface::new(&context, window.clone()).unwrap();
        let size = window.inner_size();
        resize_surface(&mut surface, size.width, size.height);
        self.surface = Some(surface);
        self.update_detected_apps();
    }

    fn window_event(&mut self, _el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => _el.exit(),
            WindowEvent::Resized(new_size) => {
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
                    if let Key::Named(NamedKey::F11) = event.logical_key {}
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.window.as_ref().unwrap().scale_factor() as f32;
                self.logical_mouse_pos = (position.x as f32 / scale, position.y as f32 / scale);

                if let Some(popup) = &mut self.popup {
                    let (pmx, pmy) = self.logical_mouse_pos;
                    let menu = popup.menu_rect();
                    let mut new_hover = None;
                    if pmx >= menu.left
                        && pmx <= menu.right
                        && pmy >= menu.top
                        && pmy <= menu.bottom
                    {
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
                        if let Some(win) = &self.window {
                            win.request_redraw();
                        }
                    }
                }

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

                if mx >= SIDEBAR_W {
                    let content_x = mx - SIDEBAR_W;
                    let content_y = my + self.scroll_y;
                    let content_w = WIN_W - SIDEBAR_W;
                    let mut new_row: Option<usize> = None;
                    self.ensure_items_cache();
                    if content_x >= CONTENT_PADDING && content_x <= content_w - CONTENT_PADDING {
                        let idx = match self
                            .cached_row_tops
                            .binary_search_by(|y| y.partial_cmp(&content_y).unwrap())
                        {
                            Ok(i) => Some(i),
                            Err(0) => None,
                            Err(i) => Some(i - 1),
                        };
                        if let Some(i) = idx {
                            if content_y <= self.cached_row_tops[i] + ROW_HEIGHT {
                                new_row = Some(i);
                            }
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
                } else {
                    if self.hover_row.is_some() {
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
                        winit::event::MouseScrollDelta::LineDelta(_, y) => y * 25.0,
                        winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                    };
                    self.target_scroll_y -= diff;
                    self.ensure_items_cache();
                    let max_scroll = self.cached_max_scroll;
                    self.target_scroll_y = self.target_scroll_y.clamp(0.0, max_scroll);
                    if let Some(win) = &self.window {
                        win.request_redraw();
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => self.handle_click(),
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
        if self.frame_count % 60 == 0 {
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
        let mut redraw = self.switch_anim.tick();
        if self.anim.tick() {
            redraw = true;
        }

        self.ensure_items_cache();
        let max_scroll = self.cached_max_scroll;
        self.target_scroll_y = self.target_scroll_y.clamp(0.0, max_scroll);
        let dt = self.last_frame_time.elapsed().as_secs_f32().clamp(0.001, 0.05);
        self.last_frame_time = Instant::now();

        let stiffness = 55.0;
        let damping = 16.0;
        let diff = self.target_scroll_y - self.scroll_y;
        let accel = diff * stiffness - self.scroll_vel_y * damping;
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

fn resize_surface(surface: &mut Surface<Arc<Window>, Arc<Window>>, width: u32, height: u32) {
    if let (Some(width), Some(height)) = (
        std::num::NonZeroU32::new(width),
        std::num::NonZeroU32::new(height),
    ) {
        let _ = surface.resize(width, height);
    }
}
