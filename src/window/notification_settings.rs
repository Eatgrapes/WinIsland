use crate::core::config::AppConfig;
use crate::core::persistence::save_config;
use crate::core::i18n::tr;
use crate::utils::color::*;
use crate::utils::settings_ui::*;
use softbuffer::{Context, Surface};
use std::sync::Arc;
use std::time::Duration;
use windows::core::w;
use windows::Win32::System::Threading::{OpenMutexW, MUTEX_ALL_ACCESS};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId, WindowButtons};
use winit::keyboard::{Key, NamedKey};
use crate::utils::icon::get_app_icon;
use skia_safe::surfaces;

const NOTIF_W: f32 = 400.0;
const NOTIF_H: f32 = 550.0;
const START_Y: f32 = 10.0;

pub struct NotificationApp {
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    sk_surface: Option<skia_safe::Surface>,
    config: AppConfig,
    logical_mouse_pos: (f32, f32),
    frame_count: u64,
    switch_anim: SwitchAnimator,
    scroll_y: f32,
    target_scroll_y: f32,
}

impl NotificationApp {
    pub fn new(config: AppConfig) -> Self {
        let switch_anim = SwitchAnimator::new(&[
            config.notification_enabled,
            config.notification_show_app_name,
        ]);
        Self {
            window: None,
            surface: None,
            sk_surface: None,
            config,
            logical_mouse_pos: (0.0, 0.0),
            frame_count: 0,
            switch_anim,
            scroll_y: 0.0,
            target_scroll_y: 0.0,
        }
    }

    fn build_items(&self) -> Vec<SettingsItem> {
        let enabled = self.config.notification_enabled;
        let mut items = vec![
            SettingsItem::Title { text: tr("notification_settings_title"), size: 22.0 },
            SettingsItem::Switch { label: tr("notification_enabled"), on: self.config.notification_enabled },
            SettingsItem::Switch { label: tr("notification_show_app"), on: self.config.notification_show_app_name },
            SettingsItem::Stepper {
                label: tr("notification_duration"),
                value: format!("{:.1}", self.config.notification_duration),
                enabled,
            },
            SettingsItem::SectionHeader {
                label: tr("notification_guide_title"),
                btn: None,
            },
        ];

        items.push(SettingsItem::Label {
            label: tr("notification_guide_step1"),
            enabled: true,
        });
        items.push(SettingsItem::Label {
            label: tr("notification_guide_step2"),
            enabled: true,
        });
        items.push(SettingsItem::Label {
            label: tr("notification_guide_step3"),
            enabled: true,
        });
        items.push(SettingsItem::Label {
            label: tr("notification_guide_step4"),
            enabled: true,
        });

        items.push(SettingsItem::SectionHeader {
            label: tr("notification_excluded_apps"),
            btn: None,
        });

        if self.config.notification_excluded_apps.is_empty() {
            items.push(SettingsItem::Label {
                label: tr("notification_no_excluded"),
                enabled,
            });
        } else {
            for app in &self.config.notification_excluded_apps {
                items.push(SettingsItem::AppItem {
                    label: app.clone(),
                    active: true,
                    enabled,
                });
            }
        }

        items.push(SettingsItem::CenterLink {
            label: tr("open_notification_settings"),
            color: COLOR_ACCENT,
        });

        items
    }

    fn sync_switch_targets(&mut self) {
        self.switch_anim.set_target(0, self.config.notification_enabled);
        self.switch_anim.set_target(1, self.config.notification_show_app_name);
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
        let dx = (logical_w - NOTIF_W) / 2.0;
        let dy = (logical_h - NOTIF_H) / 2.0;
        canvas.translate((dx, dy));

        let items = self.build_items();
        canvas.save();
        canvas.clip_rect(skia_safe::Rect::from_xywh(0.0, 0.0, NOTIF_W, NOTIF_H), skia_safe::ClipOp::Intersect, true);
        canvas.translate((0.0, -self.scroll_y));
        draw_items(canvas, &items, START_Y, NOTIF_W, &self.switch_anim);
        canvas.restore();

        let ch = content_height(&items, START_Y);
        let view_h = NOTIF_H;
        if ch > view_h {
            let bar_h = (view_h / ch) * view_h;
            let bar_y = (self.scroll_y / (ch - view_h)) * (view_h - bar_h);
            let mut p = skia_safe::Paint::default();
            p.set_anti_alias(true);
            p.set_color(skia_safe::Color::from_argb(80, 255, 255, 255));
            canvas.draw_round_rect(skia_safe::Rect::from_xywh(NOTIF_W - 6.0, bar_y, 4.0, bar_h), 2.0, 2.0, &p);
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

    fn handle_click(&mut self) {
        let (mx, my) = self.logical_mouse_pos;
        let win = self.window.as_ref().unwrap();
        let scale = win.scale_factor() as f32;
        let size = win.inner_size();
        let dx = ((size.width as f32 / scale) - NOTIF_W) / 2.0;
        let dy = ((size.height as f32 / scale) - NOTIF_H) / 2.0;
        let lmx = mx - dx;
        let lmy = my - dy;

        let content_my = lmy + self.scroll_y;
        let items = self.build_items();
        let result = hit_test(&items, lmx, content_my, START_Y, NOTIF_W);
        let mut changed = false;

        match result {
            ClickResult::Switch(idx) => {
                match idx {
                    0 => self.config.notification_enabled = !self.config.notification_enabled,
                    1 => self.config.notification_show_app_name = !self.config.notification_show_app_name,
                    _ => {}
                }
                self.sync_switch_targets();
                changed = true;
            }
            ClickResult::StepperDec(idx) => {
                let stepper_items: Vec<usize> = items.iter().enumerate()
                    .filter_map(|(i, item)| matches!(item, SettingsItem::Stepper { .. }).then_some(i))
                    .collect();
                if let Some(&stepper_idx) = stepper_items.iter().find(|&&i| i == idx) {
                    if stepper_idx == idx && idx == items.iter().position(|i| matches!(i, SettingsItem::Stepper { label, .. } if label == &tr("notification_duration"))).unwrap_or(usize::MAX) {
                        self.config.notification_duration = ((self.config.notification_duration * 10.0 - 5.0).round() / 10.0).max(1.0);
                        changed = true;
                    }
                }
            }
            ClickResult::StepperInc(idx) => {
                if idx == items.iter().position(|i| matches!(i, SettingsItem::Stepper { label, .. } if label == &tr("notification_duration"))).unwrap_or(usize::MAX) {
                    self.config.notification_duration = ((self.config.notification_duration * 10.0 + 5.0).round() / 10.0).min(30.0);
                    changed = true;
                }
            }
            ClickResult::AppItem(idx) => {
                let app_start = items.iter().position(|i| matches!(i, SettingsItem::AppItem { .. })).unwrap_or(items.len());
                let app_idx = idx - app_start;
                if app_idx < self.config.notification_excluded_apps.len() {
                    self.config.notification_excluded_apps.remove(app_idx);
                    changed = true;
                }
            }
            ClickResult::CenterLink(_) => {
                let _ = open::that("ms-settings:notifications");
            }
            _ => {}
        }

        if changed {
            save_config(&self.config);
            if let Some(win) = &self.window { win.request_redraw(); }
        }
    }

    fn get_hover_state(&self) -> bool {
        let (mx, my) = self.logical_mouse_pos;
        let win = self.window.as_ref().unwrap();
        let scale = win.scale_factor() as f32;
        let size = win.inner_size();
        let dx = ((size.width as f32 / scale) - NOTIF_W) / 2.0;
        let dy = ((size.height as f32 / scale) - NOTIF_H) / 2.0;
        let lmx = mx - dx;
        let lmy = my - dy;

        let content_my = lmy + self.scroll_y;
        let items = self.build_items();
        hover_test(&items, lmx, content_my, START_Y, NOTIF_W)
    }
}

impl ApplicationHandler for NotificationApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title(tr("notification_settings_title"))
            .with_inner_size(LogicalSize::new(NOTIF_W as f64, NOTIF_H as f64))
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
                let diff = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y * 25.0,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                };
                self.target_scroll_y -= diff;
                let items = self.build_items();
                let ch = content_height(&items, START_Y);
                let max_scroll = (ch - NOTIF_H).max(0.0);
                self.target_scroll_y = self.target_scroll_y.clamp(0.0, max_scroll);
                if let Some(win) = &self.window { win.request_redraw(); }
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
            let items = self.build_items();
            let ch = content_height(&items, START_Y);
            let view_h = NOTIF_H;
            let max_scroll = (ch - view_h).max(0.0);
            self.target_scroll_y = self.target_scroll_y.clamp(0.0, max_scroll);
            if (self.target_scroll_y - self.scroll_y).abs() > 0.1 {
                self.scroll_y += (self.target_scroll_y - self.scroll_y) * 0.28;
                redraw = true;
            } else if (self.scroll_y - self.target_scroll_y).abs() > f32::EPSILON {
                self.scroll_y = self.target_scroll_y;
            }
            if redraw { win.request_redraw(); }
            std::thread::sleep(Duration::from_millis(16));
        }
    }
}

pub fn run_notification_settings(config: AppConfig) {
    let el = EventLoop::new().unwrap();
    let mut app = NotificationApp::new(config);
    el.run_app(&mut app).unwrap();
}
