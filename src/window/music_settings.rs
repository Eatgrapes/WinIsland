use crate::core::config::AppConfig;
use crate::core::persistence::save_config;
use crate::core::i18n::tr;
use crate::utils::color::*;
use crate::utils::settings_ui::{items::LABEL_Y_OFFSET, *};
use skia_safe::{surfaces, Paint};
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

const MUSIC_W: f32 = 400.0;
const MUSIC_H: f32 = 550.0;
const START_Y: f32 = 10.0;

pub struct MusicApp {
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    sk_surface: Option<skia_safe::Surface>,
    config: AppConfig,
    logical_mouse_pos: (f32, f32),
    frame_count: u64,
    switch_anim: SwitchAnimator,
    detected_apps: Vec<String>,
}

impl MusicApp {
    pub fn new(config: AppConfig) -> Self {
        let switch_anim = SwitchAnimator::new(&[
            config.smtc_enabled,
            config.show_lyrics,
            config.lyrics_fallback,
        ]);
        Self {
            window: None,
            surface: None,
            sk_surface: None,
            config,
            logical_mouse_pos: (0.0, 0.0),
            frame_count: 0,
            switch_anim,
            detected_apps: Vec::new(),
        }
    }

    fn build_items(&self) -> Vec<SettingsItem> {
        let show_lyrics = self.config.show_lyrics;
        let enabled = self.config.smtc_enabled;
        let source = &self.config.lyrics_source;

        let mut items = vec![
            SettingsItem::Title { text: tr("music_settings_title"), size: 22.0 },
            SettingsItem::Switch { label: tr("smtc_control"), on: self.config.smtc_enabled },
            SettingsItem::Switch { label: tr("show_lyrics"), on: self.config.show_lyrics },
            SettingsItem::SourceSelect {
                label: tr("lyrics_source"),
                options: vec![
                    ("163".to_string(), 235.0, 50.0, source == "163"),
                    ("LRCLIB".to_string(), 292.0, 68.0, source == "lrclib"),
                ],
                enabled: show_lyrics,
            },
            SettingsItem::Switch { label: tr("lyrics_fallback"), on: if show_lyrics { self.config.lyrics_fallback } else { false } },
            SettingsItem::SectionHeader {
                label: tr("media_apps"),
                btn: Some((tr("scan_apps"), MUSIC_W - 130.0, 110.0, enabled)),
            },
        ];

        if self.detected_apps.is_empty() {
            items.push(SettingsItem::Label { label: tr("no_sessions"), enabled });
        } else {
            for app in &self.detected_apps {
                let display_name = app.split('!').next().unwrap_or(app);
                items.push(SettingsItem::Label { label: display_name.to_string(), enabled });
            }
        }

        items
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
    }

    fn sync_switch_targets(&mut self) {
        self.switch_anim.set_target(0, self.config.smtc_enabled);
        self.switch_anim.set_target(1, self.config.show_lyrics);
        self.switch_anim.set_target(2, if self.config.show_lyrics { self.config.lyrics_fallback } else { false });
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
        let dx = (logical_w - MUSIC_W) / 2.0;
        let dy = (logical_h - MUSIC_H) / 2.0;
        canvas.translate((dx, dy));

        let items = self.build_items();
        draw_items(canvas, &items, START_Y, MUSIC_W, &self.switch_anim);

        if !self.detected_apps.is_empty() && self.config.smtc_enabled {
            let fm = crate::utils::font::FontManager::global();
            let mut paint = Paint::default();
            paint.set_anti_alias(true);
            let app_start_idx = if self.detected_apps.is_empty() { 0 } else {
                items.len() - self.detected_apps.len()
            };
            let mut app_y = START_Y;
            for (i, item) in items.iter().enumerate() {
                if i >= app_start_idx && i < app_start_idx + self.detected_apps.len() {
                    let app_idx = i - app_start_idx;
                    let is_active = self.config.smtc_apps.contains(&self.detected_apps[app_idx]);
                    paint.set_color(if is_active { COLOR_ACCENT } else { COLOR_TEXT_SEC });
                    canvas.draw_circle((45.0, app_y + 17.5), 8.0, &paint);

                    let del_str = tr("delete");
                    let (_, rect) = fm.measure(&del_str, 12.0, false);
                    paint.set_color(COLOR_DANGER);
                    fm.draw_text(canvas, &del_str, (MUSIC_W - 35.0 - rect.width(), app_y + LABEL_Y_OFFSET), 12.0, false, &paint);
                }
                app_y += item.height();
            }
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
        let dx = ((size.width as f32 / scale) - MUSIC_W) / 2.0;
        let dy = ((size.height as f32 / scale) - MUSIC_H) / 2.0;
        let lmx = mx - dx;
        let lmy = my - dy;

        let items = self.build_items();
        let result = hit_test(&items, lmx, lmy, START_Y, MUSIC_W);
        let mut changed = false;

        match result {
            ClickResult::Switch(idx) => {
                match idx {
                    0 => self.config.smtc_enabled = !self.config.smtc_enabled,
                    1 => self.config.show_lyrics = !self.config.show_lyrics,
                    2 => if self.config.show_lyrics { self.config.lyrics_fallback = !self.config.lyrics_fallback },
                    _ => {}
                }
                self.sync_switch_targets();
                changed = true;
            }
            ClickResult::SourceOption(_, opt_idx) => {
                self.config.lyrics_source = if opt_idx == 0 { "163".to_string() } else { "lrclib".to_string() };
                changed = true;
            }
            ClickResult::SectionButton(_) => {
                if self.config.smtc_enabled {
                    self.update_detected_apps();
                    if let Some(win) = &self.window { win.request_redraw(); }
                }
            }
            ClickResult::Label(idx) => {
                if self.config.smtc_enabled && !self.detected_apps.is_empty() {
                    let app_start = items.len() - self.detected_apps.len();
                    if idx >= app_start {
                        let app_idx = idx - app_start;
                        if app_idx < self.detected_apps.len() {
                            let app = &self.detected_apps[app_idx];
                            if self.config.smtc_apps.contains(app) {
                                self.config.smtc_apps.retain(|a| a != app);
                            } else {
                                self.config.smtc_apps.push(app.clone());
                            }
                            changed = true;
                        }
                    }
                }
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
        let dx = ((size.width as f32 / scale) - MUSIC_W) / 2.0;
        let dy = ((size.height as f32 / scale) - MUSIC_H) / 2.0;
        let lmx = mx - dx;
        let lmy = my - dy;

        let items = self.build_items();
        hover_test(&items, lmx, lmy, START_Y, MUSIC_W)
    }
}

impl ApplicationHandler for MusicApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title(tr("music_settings_title"))
            .with_inner_size(LogicalSize::new(MUSIC_W as f64, MUSIC_H as f64))
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
        self.update_detected_apps();
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
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => self.handle_click(),
            WindowEvent::RedrawRequested => self.draw(),
            _ => (),
        }
    }
    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        if let Some(win) = &self.window {
            let win_clone = win.clone();
            self.frame_count += 1;
            if self.frame_count % 60 == 0 {
                unsafe {
                    let h = OpenMutexW(MUTEX_ALL_ACCESS, false, w!("Local\\WinIsland_SingleInstance_Mutex"));
                    if h.is_err() { _el.exit(); return; }
                    let _ = windows::Win32::Foundation::CloseHandle(h.unwrap());
                }
                win_clone.request_redraw();
            }
            let redraw = self.switch_anim.tick();
            if redraw { win_clone.request_redraw(); }
            std::thread::sleep(Duration::from_millis(16));
        }
    }
}

pub fn run_music_settings(config: AppConfig) {
    let el = EventLoop::new().unwrap();
    let mut app = MusicApp::new(config);
    el.run_app(&mut app).unwrap();
}
