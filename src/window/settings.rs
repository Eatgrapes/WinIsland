use crate::core::config::{AppConfig, APP_AUTHOR, APP_HOMEPAGE, APP_VERSION};
use crate::core::persistence::save_config;
use crate::core::i18n::{tr, set_lang, current_lang};
use crate::utils::color::*;
use crate::utils::font::FontManager;
use crate::utils::settings_ui::*;
use skia_safe::{surfaces, Color, Paint, Rect};
use softbuffer::{Context, Surface};
use std::sync::Arc;
use windows::core::w;
use windows::Win32::System::Threading::{OpenMutexW, MUTEX_ALL_ACCESS};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId, WindowButtons};
use winit::keyboard::{Key, NamedKey};
use crate::utils::icon::get_app_icon;
use crate::utils::autostart::set_autostart;

const SETTINGS_W: f32 = 400.0;
const SETTINGS_H: f32 = 550.0;
const START_Y: f32 = 90.0;

pub struct SettingsApp {
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    sk_surface: Option<skia_safe::Surface>,
    config: AppConfig,
    active_tab: usize,
    switch_anim: SwitchAnimator,
    logical_mouse_pos: (f32, f32),
    frame_count: u64,
    scroll_y: f32,
    target_scroll_y: f32,
}

impl SettingsApp {
    pub fn new(config: AppConfig) -> Self {
        let switch_anim = SwitchAnimator::new(&[
            config.adaptive_border,
            config.motion_blur,
            config.auto_start,
            config.auto_hide,
            config.check_for_updates,
        ]);
        Self {
            window: None,
            surface: None,
            sk_surface: None,
            config,
            active_tab: 0,
            switch_anim,
            logical_mouse_pos: (0.0, 0.0),
            frame_count: 0,
            scroll_y: 0.0,
            target_scroll_y: 0.0,
        }
    }

    fn build_general_items(&self) -> Vec<SettingsItem> {
        let mut items: Vec<SettingsItem> = vec![
            SettingsItem::Stepper { label: tr("global_scale"), value: format!("{:.2}", self.config.global_scale), enabled: true },
            SettingsItem::Stepper { label: tr("base_width"), value: self.config.base_width.to_string(), enabled: true },
            SettingsItem::Stepper { label: tr("base_height"), value: self.config.base_height.to_string(), enabled: true },
            SettingsItem::Stepper { label: tr("expanded_width"), value: self.config.expanded_width.to_string(), enabled: true },
            SettingsItem::Stepper { label: tr("expanded_height"), value: self.config.expanded_height.to_string(), enabled: true },
            SettingsItem::Switch { label: tr("adaptive_border"), on: self.config.adaptive_border },
            SettingsItem::Switch { label: tr("motion_blur"), on: self.config.motion_blur },
            SettingsItem::FontPicker {
                label: tr("custom_font"),
                btn_label: tr("font_select"),
                reset_label: if self.config.custom_font_path.is_some() { Some(tr("font_reset")) } else { None },
            },
            SettingsItem::Switch { label: tr("start_boot"), on: self.config.auto_start },
            SettingsItem::Switch { label: tr("auto_hide"), on: self.config.auto_hide },
            SettingsItem::Switch { label: tr("check_updates"), on: self.config.check_for_updates },
            SettingsItem::Stepper { label: tr("update_interval"), value: format!("{:.0}", self.config.update_check_interval), enabled: self.config.check_for_updates },
            SettingsItem::TextButton { label: tr("language"), btn_label: tr("lang_name"), btn_x: 300.0, btn_w: 75.0 },
        ];
        if self.config.auto_hide {
            items.push(SettingsItem::Stepper { label: tr("hide_delay"), value: format!("{:.0}", self.config.auto_hide_delay), enabled: true });
        }
        items.push(SettingsItem::CenterLink { label: tr("reset_defaults"), color: COLOR_DANGER });
        items
    }

    fn build_about_items(&self) -> Vec<SettingsItem> {
        vec![
            SettingsItem::CenterText { text: "WinIsland".to_string(), size: 28.0, color: COLOR_TEXT_PRI },
            SettingsItem::CenterText { text: format!("Version {}", APP_VERSION), size: 14.0, color: COLOR_TEXT_SEC },
            SettingsItem::CenterText { text: format!("{} {}", tr("created_by"), APP_AUTHOR), size: 14.0, color: COLOR_TEXT_SEC },
            SettingsItem::CenterText { text: String::new(), size: 14.0, color: COLOR_TEXT_SEC },
            SettingsItem::CenterLink { label: tr("visit_homepage"), color: COLOR_ACCENT },
        ]
    }

    fn sync_switch_targets(&mut self) {
        self.switch_anim.set_target(0, self.config.adaptive_border);
        self.switch_anim.set_target(1, self.config.motion_blur);
        self.switch_anim.set_target(2, self.config.auto_start);
        self.switch_anim.set_target(3, self.config.auto_hide);
        self.switch_anim.set_target(4, self.config.check_for_updates);
    }

    fn draw(&mut self) {
        let win = self.window.as_ref().unwrap();
        let size = win.inner_size();
        let p_w = size.width as i32;
        let p_h = size.height as i32;
        if p_w <= 0 || p_h <= 0 { return; }

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
        canvas.clear(COLOR_BG);
        let scale = win.scale_factor() as f32;
        canvas.scale((scale, scale));

        let logical_w = p_w as f32 / scale;
        let logical_h = p_h as f32 / scale;
        let dx = (logical_w - SETTINGS_W) / 2.0;
        let dy = (logical_h - SETTINGS_H) / 2.0;
        canvas.translate((dx, dy));

        self.draw_tabs(canvas);

        if self.active_tab == 0 {
            let items = self.build_general_items();
            canvas.save();
            canvas.clip_rect(Rect::from_xywh(0.0, 70.0, SETTINGS_W, SETTINGS_H - 70.0), skia_safe::ClipOp::Intersect, true);
            canvas.translate((0.0, -self.scroll_y));
            draw_items(canvas, &items, START_Y, SETTINGS_W, &self.switch_anim);
            canvas.restore();

            let ch = content_height(&items, START_Y);
            let view_h = SETTINGS_H - 70.0;
            if ch > view_h {
                let bar_h = (view_h / ch) * view_h;
                let bar_y = 70.0 + (self.scroll_y / (ch - view_h)) * (view_h - bar_h);
                let mut p = Paint::default();
                p.set_anti_alias(true);
                p.set_color(Color::from_argb(80, 255, 255, 255));
                canvas.draw_round_rect(Rect::from_xywh(SETTINGS_W - 6.0, bar_y, 4.0, bar_h), 2.0, 2.0, &p);
            }
        } else {
            let items = self.build_about_items();
            draw_items(canvas, &items, 120.0, SETTINGS_W, &self.switch_anim);
        }

        if let Some(surface) = self.surface.as_mut() {
            let mut buffer = surface.buffer_mut().unwrap();
            let info = skia_safe::ImageInfo::new(skia_safe::ISize::new(p_w, p_h), skia_safe::ColorType::BGRA8888, skia_safe::AlphaType::Premul, None);
            let dst_row_bytes = (p_w * 4) as usize;
            let u8_buffer: &mut [u8] = bytemuck::cast_slice_mut(&mut *buffer);
            let _ = sk_surface.read_pixels(&info, u8_buffer, dst_row_bytes, (0, 0));
            buffer.present().unwrap();
        }
    }

    fn draw_tabs(&self, canvas: &skia_safe::Canvas) {
        let fm = FontManager::global();
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        let center_x = SETTINGS_W / 2.0;
        let tabs = [tr("tab_general"), tr("tab_about")];
        paint.set_color(COLOR_CARD);
        canvas.draw_round_rect(Rect::from_xywh(center_x - 85.0, 20.0, 170.0, 36.0), 10.0, 10.0, &paint);
        for (i, label) in tabs.iter().enumerate() {
            let bx = center_x - 82.0 + (i as f32 * 82.0);
            if self.active_tab == i {
                paint.set_color(COLOR_CARD_HIGHLIGHT);
                canvas.draw_round_rect(Rect::from_xywh(bx, 23.0, 80.0, 30.0), 8.0, 8.0, &paint);
                paint.set_color(COLOR_TEXT_PRI);
            } else {
                paint.set_color(COLOR_TEXT_SEC);
            }
            fm.draw_text_in_rect(canvas, label, bx, 43.0, 80.0, 14.0, true, &paint);
        }
    }

    fn handle_click(&mut self) {
        let (mx, my) = self.logical_mouse_pos;
        let cx = SETTINGS_W / 2.0;

        let win = self.window.as_ref().unwrap();
        let scale = win.scale_factor() as f32;
        let size = win.inner_size();
        let dx = ((size.width as f32 / scale) - SETTINGS_W) / 2.0;
        let dy = ((size.height as f32 / scale) - SETTINGS_H) / 2.0;
        let lmx = mx - dx;
        let lmy = my - dy;

        if lmy >= 20.0 && lmy <= 56.0 {
            if lmx >= cx - 85.0 && lmx <= cx { self.active_tab = 0; }
            else if lmx >= cx && lmx <= cx + 85.0 { self.active_tab = 1; }
            if let Some(win) = &self.window { win.request_redraw(); }
            return;
        }

        if self.active_tab == 0 {
            let content_my = if lmy >= 70.0 { lmy + self.scroll_y } else { lmy };
            let items = self.build_general_items();
            let result = hit_test(&items, lmx, content_my, START_Y, SETTINGS_W);
            let mut changed = false;

            match result {
                ClickResult::StepperDec(0) => { self.config.global_scale = ((self.config.global_scale - 0.05) * 100.0).round() / 100.0; self.config.global_scale = self.config.global_scale.max(0.5); changed = true; }
                ClickResult::StepperInc(0) => { self.config.global_scale = ((self.config.global_scale + 0.05) * 100.0).round() / 100.0; self.config.global_scale = self.config.global_scale.min(5.0); changed = true; }
                ClickResult::StepperDec(1) => { self.config.base_width -= 5.0; changed = true; }
                ClickResult::StepperInc(1) => { self.config.base_width += 5.0; changed = true; }
                ClickResult::StepperDec(2) => { self.config.base_height -= 2.0; changed = true; }
                ClickResult::StepperInc(2) => { self.config.base_height += 2.0; changed = true; }
                ClickResult::StepperDec(3) => { self.config.expanded_width -= 10.0; changed = true; }
                ClickResult::StepperInc(3) => { self.config.expanded_width += 10.0; changed = true; }
                ClickResult::StepperDec(4) => { self.config.expanded_height -= 10.0; changed = true; }
                ClickResult::StepperInc(4) => { self.config.expanded_height += 10.0; changed = true; }
                ClickResult::Switch(idx) => {
                    match idx {
                        0 => self.config.adaptive_border = !self.config.adaptive_border,
                        1 => self.config.motion_blur = !self.config.motion_blur,
                        2 => { self.config.auto_start = !self.config.auto_start; let _ = set_autostart(self.config.auto_start); }
                        3 => self.config.auto_hide = !self.config.auto_hide,
                        4 => self.config.check_for_updates = !self.config.check_for_updates,
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
                ClickResult::StepperDec(idx) if idx == items.iter().position(|i| matches!(i, SettingsItem::Stepper { label, .. } if label == &tr("update_interval"))).unwrap_or(usize::MAX) => {
                    self.config.update_check_interval = (self.config.update_check_interval - 1.0).max(1.0);
                    changed = true;
                }
                ClickResult::StepperInc(idx) if idx == items.iter().position(|i| matches!(i, SettingsItem::Stepper { label, .. } if label == &tr("update_interval"))).unwrap_or(usize::MAX) => {
                    self.config.update_check_interval = (self.config.update_check_interval + 1.0).min(24.0);
                    changed = true;
                }
                ClickResult::TextButton(_) => {
                    self.config.language = if current_lang() == "zh" { "en".to_string() } else { "zh".to_string() };
                    set_lang(&self.config.language);
                    changed = true;
                }
                ClickResult::StepperDec(idx) if self.config.auto_hide && idx == items.len() - 2 => {
                    self.config.auto_hide_delay = (self.config.auto_hide_delay - 1.0).max(1.0);
                    changed = true;
                }
                ClickResult::StepperInc(idx) if self.config.auto_hide && idx == items.len() - 2 => {
                    self.config.auto_hide_delay = (self.config.auto_hide_delay + 1.0).min(60.0);
                    changed = true;
                }
                ClickResult::CenterLink(_) => {
                    self.config = AppConfig::default();
                    set_lang(if self.config.language == "auto" { "en" } else { &self.config.language });
                    FontManager::global().refresh_custom_font();
                    self.switch_anim = SwitchAnimator::new(&[
                        self.config.adaptive_border,
                        self.config.motion_blur,
                        self.config.auto_start,
                        self.config.auto_hide,
                        self.config.check_for_updates,
                    ]);
                    changed = true;
                }
                _ => {}
            }

            if changed {
                let items = self.build_general_items();
                let ch = content_height(&items, START_Y);
                let view_h = SETTINGS_H - 70.0;
                let max_scroll = (ch - view_h).max(0.0);
                self.target_scroll_y = self.target_scroll_y.clamp(0.0, max_scroll);
                self.scroll_y = self.scroll_y.clamp(0.0, max_scroll);
                save_config(&self.config);
                if let Some(win) = &self.window { win.request_redraw(); }
            }
        } else {
            let items = self.build_about_items();
            let result = hit_test(&items, lmx, lmy, 120.0, SETTINGS_W);
            if let ClickResult::CenterLink(_) = result {
                let _ = open::that(APP_HOMEPAGE);
            }
        }
    }

    fn get_hover_state(&self) -> bool {
        let (mx, my) = self.logical_mouse_pos;
        let cx = SETTINGS_W / 2.0;

        let win = self.window.as_ref().unwrap();
        let scale = win.scale_factor() as f32;
        let size = win.inner_size();
        let dx = ((size.width as f32 / scale) - SETTINGS_W) / 2.0;
        let dy = ((size.height as f32 / scale) - SETTINGS_H) / 2.0;
        let lmx = mx - dx;
        let lmy = my - dy;

        if lmy >= 20.0 && lmy <= 56.0 && lmx >= cx - 85.0 && lmx <= cx + 85.0 {
            return true;
        }

        if self.active_tab == 0 {
            let content_my = if lmy >= 70.0 { lmy + self.scroll_y } else { lmy };
            let items = self.build_general_items();
            hover_test(&items, lmx, content_my, START_Y, SETTINGS_W)
        } else {
            let items = self.build_about_items();
            hover_test(&items, lmx, lmy, 120.0, SETTINGS_W)
        }
    }
}

impl ApplicationHandler for SettingsApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title("Settings")
            .with_inner_size(LogicalSize::new(SETTINGS_W as f64, SETTINGS_H as f64))
            .with_resizable(false)
            .with_enabled_buttons(WindowButtons::CLOSE | WindowButtons::MINIMIZE)
            .with_window_icon(get_app_icon());
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        self.window = Some(window.clone());
        let context = Context::new(window.clone()).unwrap();
        let mut surface = Surface::new(&context, window.clone()).unwrap();
        let size = window.inner_size();
        surface.resize(std::num::NonZeroU32::new(size.width).unwrap(), std::num::NonZeroU32::new(size.height).unwrap()).unwrap();
        self.surface = Some(surface);
    }
    fn window_event(&mut self, _el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => _el.exit(),
            WindowEvent::Resized(new_size) => {
                if let Some(surface) = &mut self.surface {
                    surface.resize(std::num::NonZeroU32::new(new_size.width).unwrap(), std::num::NonZeroU32::new(new_size.height).unwrap()).unwrap();
                    if let Some(win) = &self.window { win.request_redraw(); }
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let (Some(win), Some(surface)) = (&self.window, &mut self.surface) {
                    let size = win.inner_size();
                    surface.resize(std::num::NonZeroU32::new(size.width).unwrap(), std::num::NonZeroU32::new(size.height).unwrap()).unwrap();
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
                if let Some(win) = &self.window {
                    let cursor = if self.get_hover_state() {
                        winit::window::CursorIcon::Pointer
                    } else {
                        winit::window::CursorIcon::Default
                    };
                    win.set_cursor(cursor);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if self.active_tab == 0 {
                    let diff = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => y * 25.0,
                        winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                    };
                    self.target_scroll_y -= diff;
                    let items = self.build_general_items();
                    let ch = content_height(&items, START_Y);
                    let max_scroll = (ch - (SETTINGS_H - 70.0)).max(0.0);
                    self.target_scroll_y = self.target_scroll_y.clamp(0.0, max_scroll);
                    if let Some(win) = &self.window { win.request_redraw(); }
                }
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => self.handle_click(),
            WindowEvent::RedrawRequested => self.draw(),
            _ => (),
        }
    }
    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        if let Some(win) = &self.window {
            self.frame_count += 1;
            if self.frame_count % 60 == 0 {
                unsafe {
                    let h = OpenMutexW(MUTEX_ALL_ACCESS, false, w!("Local\\WinIsland_SingleInstance_Mutex"));
                    if h.is_err() { _el.exit(); return; }
                    let _ = windows::Win32::Foundation::CloseHandle(h.unwrap());
                }
            }
            let mut redraw = self.switch_anim.tick();
            let items = self.build_general_items();
            let ch = content_height(&items, START_Y);
            let view_h = SETTINGS_H - 70.0;
            let max_scroll = (ch - view_h).max(0.0);
            self.target_scroll_y = self.target_scroll_y.clamp(0.0, max_scroll);
            if (self.target_scroll_y - self.scroll_y).abs() > 0.1 {
                self.scroll_y += (self.target_scroll_y - self.scroll_y) * 0.28;
                redraw = true;
            } else if (self.scroll_y - self.target_scroll_y).abs() > f32::EPSILON {
                self.scroll_y = self.target_scroll_y;
            }
            if redraw { win.request_redraw(); }
        }
    }
}

pub fn run_settings(config: AppConfig) {
    let el = EventLoop::new().unwrap();
    let mut app = SettingsApp::new(config);
    el.run_app(&mut app).unwrap();
}
