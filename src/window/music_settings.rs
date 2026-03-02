use crate::core::config::AppConfig;
use crate::core::persistence::save_config;
use crate::core::i18n::tr;
use crate::utils::color::*;
use skia_safe::{surfaces, Color, Font, FontMgr, FontStyle, Paint, Rect};
use softbuffer::{Context, Surface};
use std::sync::Arc;
use std::time::Duration;
use windows::core::w;
use windows::Win32::System::Threading::{OpenMutexW, MUTEX_ALL_ACCESS};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};
const MUSIC_W: f32 = 400.0;
const MUSIC_H: f32 = 550.0;
use crate::utils::icon::get_app_icon;
pub struct MusicApp {
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    config: AppConfig,
    logical_mouse_pos: (f32, f32),
    font_mgr: FontMgr,
    frame_count: u64,
    switch_pos: f32,
    lyrics_switch_pos: f32,
    detected_apps: Vec<String>,
}
impl MusicApp {
    pub fn new(config: AppConfig) -> Self {
        let sp = if config.smtc_enabled { 1.0 } else { 0.0 };
        let lsp = if config.show_lyrics { 1.0 } else { 0.0 };
        Self {
            window: None,
            surface: None,
            config,
            logical_mouse_pos: (0.0, 0.0),
            font_mgr: FontMgr::new(),
            frame_count: 0,
            switch_pos: sp,
            lyrics_switch_pos: lsp,
            detected_apps: Vec::new(),
        }
    }
    fn get_font(&self, size: f32, bold: bool) -> Font {
        let style = if bold { FontStyle::bold() } else { FontStyle::normal() };
        if let Some(path) = &self.config.custom_font_path {
            if let Ok(data) = std::fs::read(path) {
                if let Some(tf) = self.font_mgr.new_from_data(&data, None) {
                    return Font::from_typeface(tf, size);
                }
            }
        }
        let typeface = self.font_mgr.match_family_style("Segoe UI", style)
            .unwrap_or_else(|| self.font_mgr.legacy_make_typeface(None, style).unwrap());
        Font::from_typeface(typeface, size)
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
    fn draw(&mut self) {
        let win = self.window.as_ref().unwrap();
        let scale = win.scale_factor() as f32;
        let p_w = (MUSIC_W * scale) as i32;
        let p_h = (MUSIC_H * scale) as i32;
        if p_w <= 0 || p_h <= 0 { return; }
        let mut sk_surface = surfaces::raster_n32_premul(skia_safe::ISize::new(p_w, p_h)).unwrap();
        let canvas = sk_surface.canvas();
        canvas.clear(COLOR_BG);
        canvas.scale((scale, scale));
        let font_title = self.get_font(22.0, true);
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_color(COLOR_TEXT_PRI);
        canvas.draw_str(&tr("music_settings_title"), (25.0, 45.0), &font_title, &paint);
        
        paint.set_color(COLOR_CARD);
        canvas.draw_round_rect(Rect::from_xywh(20.0, 70.0, MUSIC_W - 40.0, 100.0), 12.0, 12.0, &paint);
        
        let font_item = self.get_font(15.0, false);
        paint.set_color(COLOR_TEXT_PRI);
        canvas.draw_str(&tr("smtc_control"), (40.0, 102.0), &font_item, &paint);
        self.draw_switch(canvas, 325.0, 82.0, self.switch_pos);
        
        canvas.draw_str(&tr("show_lyrics"), (40.0, 152.0), &font_item, &paint);
        self.draw_switch(canvas, 325.0, 132.0, self.lyrics_switch_pos);

        let enabled = self.config.smtc_enabled;
        let text_color = if enabled { COLOR_TEXT_PRI } else { COLOR_TEXT_SEC };
        let sec_color = if enabled { COLOR_TEXT_SEC } else { COLOR_DISABLED };
        let media_apps_y = 195.0;
        paint.set_color(sec_color);
        let font_sec = self.get_font(12.0, true);
        canvas.draw_str(&tr("media_apps"), (30.0, media_apps_y + 15.0), &font_sec, &paint);
        self.draw_text_button(canvas, MUSIC_W - 130.0, media_apps_y, 110.0, 24.0, &tr("scan_apps"), enabled);
        
        let mut current_y = media_apps_y + 30.0;
        if self.detected_apps.is_empty() {
            paint.set_color(sec_color);
            canvas.draw_str(&tr("no_sessions"), (40.0, current_y + 25.0), &font_item, &paint);
        } else {
            for app in &self.detected_apps {
                paint.set_color(COLOR_CARD);
                canvas.draw_round_rect(Rect::from_xywh(20.0, current_y, MUSIC_W - 40.0, 45.0), 10.0, 10.0, &paint);
                let is_active = self.config.smtc_apps.contains(app);
                paint.set_color(if is_active && enabled { COLOR_ACCENT } else { if enabled { COLOR_TEXT_SEC } else { COLOR_DISABLED } });
                canvas.draw_circle((45.0, current_y + 22.5), 8.0, &paint);
                paint.set_color(text_color);
                let display_name = app.split('!').next().unwrap_or(app);
                canvas.draw_str(display_name, (65.0, current_y + 27.0), &font_item, &paint);
                if enabled {
                    let del_font = self.get_font(12.0, false);
                    let del_str = tr("delete");
                    let (_, rect) = del_font.measure_str(&del_str, None);
                    paint.set_color(COLOR_DANGER);
                    canvas.draw_str(&del_str, (MUSIC_W - 35.0 - rect.width(), current_y + 27.0), &del_font, &paint);
                }
                current_y += 50.0;
                if current_y > MUSIC_H - 50.0 { break; }
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
    fn draw_switch(&self, canvas: &skia_safe::Canvas, x: f32, y: f32, pos: f32) {
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        let color_off = COLOR_CARD_HIGHLIGHT;
        let color_on = COLOR_ACCENT;
        let r = color_off.r() as f32 + (color_on.r() as f32 - color_off.r() as f32) * pos;
        let g = color_off.g() as f32 + (color_on.g() as f32 - color_off.g() as f32) * pos;
        let b = color_off.b() as f32 + (color_on.b() as f32 - color_off.b() as f32) * pos;
        paint.set_color(Color::from_rgb(r as u8, g as u8, b as u8));
        canvas.draw_round_rect(Rect::from_xywh(x, y, 48.0, 26.0), 13.0, 13.0, &paint);
        paint.set_color(Color::WHITE);
        canvas.draw_round_rect(Rect::from_xywh(x + 2.0 + (pos * 22.0), y + 2.0, 22.0, 22.0), 11.0, 11.0, &paint);
    }
    fn draw_text_button(&self, canvas: &skia_safe::Canvas, x: f32, y: f32, w: f32, h: f32, label: &str, enabled: bool) {
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_color(if enabled { COLOR_CARD_HIGHLIGHT } else { COLOR_DISABLED });
        canvas.draw_round_rect(Rect::from_xywh(x, y, w, h), h/2.0, h/2.0, &paint);
        let font = self.get_font(12.0, true);
        paint.set_color(if enabled { COLOR_TEXT_PRI } else { COLOR_TEXT_SEC });
        let (_, rect) = font.measure_str(label, None);
        canvas.draw_str(label, (x + (w - rect.width()) / 2.0, y + 16.0), &font, &paint);
    }
    fn handle_click(&mut self) {
        let (mx, my) = self.logical_mouse_pos;
        let mut changed = false;
        if mx >= 320.0 && mx <= 380.0 && my >= 80.0 && my <= 110.0 {
            self.config.smtc_enabled = !self.config.smtc_enabled;
            changed = true;
        }
        if mx >= 320.0 && mx <= 380.0 && my >= 130.0 && my <= 160.0 {
            self.config.show_lyrics = !self.config.show_lyrics;
            changed = true;
        }
        
        let media_apps_y = 195.0;

        if self.config.smtc_enabled {
            if mx >= MUSIC_W - 130.0 && mx <= MUSIC_W - 20.0 && my >= media_apps_y && my <= media_apps_y + 24.0 {
                self.update_detected_apps();
                if let Some(win) = &self.window { win.request_redraw(); }
            }
            let mut current_y = media_apps_y + 30.0;
            let mut to_remove = None;
            for (i, app) in self.detected_apps.iter().enumerate() {
                if mx >= 320.0 && mx <= 380.0 && my >= current_y && my <= current_y + 45.0 {
                    to_remove = Some(i);
                    changed = true;
                    break;
                } else if mx >= 20.0 && mx <= 320.0 && my >= current_y && my <= current_y + 45.0 {
                    if self.config.smtc_apps.contains(app) {
                        self.config.smtc_apps.retain(|a| a != app);
                    } else {
                        self.config.smtc_apps.push(app.clone());
                    }
                    changed = true;
                    break;
                }
                current_y += 50.0;
                if current_y > MUSIC_H - 50.0 { break; }
            }
            if let Some(i) = to_remove {
                let app = self.detected_apps.remove(i);
                self.config.smtc_apps.retain(|a| a != &app);
            }
        }
        if changed {
            save_config(&self.config);
            if let Some(win) = &self.window { win.request_redraw(); }
        }
    }
}
impl ApplicationHandler for MusicApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title(tr("music_settings_title"))
            .with_inner_size(LogicalSize::new(MUSIC_W as f64, MUSIC_H as f64))
            .with_resizable(false)
            .with_window_icon(get_app_icon());
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        self.window = Some(window.clone());
        let context = Context::new(window.clone()).unwrap();
        let mut surface = Surface::new(&context, window.clone()).unwrap();
        let scale = window.scale_factor();
        surface.resize(std::num::NonZeroU32::new((MUSIC_W as f64 * scale) as u32).unwrap(), std::num::NonZeroU32::new((MUSIC_H as f64 * scale) as u32).unwrap()).unwrap();
        self.surface = Some(surface);
        self.update_detected_apps();
    }
    fn window_event(&mut self, _el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => _el.exit(),
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.window.as_ref().unwrap().scale_factor() as f32;
                self.logical_mouse_pos = (position.x as f32 / scale, position.y as f32 / scale);
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
                    let h = OpenMutexW(MUTEX_ALL_ACCESS, false, w!("Global\\WinIsland_SingleInstance_Mutex"));
                    if h.is_err() { _el.exit(); return; }
                    let _ = windows::Win32::Foundation::CloseHandle(h.unwrap());
                }
                win_clone.request_redraw();
            }
            let mut redraw = false;
            let target = if self.config.smtc_enabled { 1.0 } else { 0.0 };
            if (target - self.switch_pos).abs() > 0.01 {
                self.switch_pos += (target - self.switch_pos) * 0.2;
                redraw = true;
            }
            let l_target = if self.config.show_lyrics { 1.0 } else { 0.0 };
            if (l_target - self.lyrics_switch_pos).abs() > 0.01 {
                self.lyrics_switch_pos += (l_target - self.lyrics_switch_pos) * 0.2;
                redraw = true;
            }
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
