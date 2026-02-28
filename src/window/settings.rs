use std::sync::Arc;
use std::time::Duration;
use winit::application::ApplicationHandler;
use winit::event::{WindowEvent, MouseButton, ElementState};
use winit::event_loop::{ActiveEventLoop, EventLoop, ControlFlow};
use winit::window::{Window, WindowId};
use winit::dpi::LogicalSize;
use skia_safe::{Color, Paint, Rect, surfaces, Font, FontStyle, FontMgr};
use softbuffer::{Context, Surface};
use crate::core::config::{AppConfig, APP_VERSION, APP_AUTHOR, APP_HOMEPAGE};
use crate::core::persistence::save_config;

const SETTINGS_W: f32 = 400.0;
const SETTINGS_H: f32 = 500.0;

pub struct SettingsApp {
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    config: AppConfig,
    active_tab: usize,
    switch_pos: f32,
    logical_mouse_pos: (f32, f32),
    font_mgr: FontMgr,
}

impl SettingsApp {
    pub fn new(config: AppConfig) -> Self {
        let initial_switch = if config.adaptive_border { 1.0 } else { 0.0 };
        Self {
            window: None,
            surface: None,
            config,
            active_tab: 0,
            switch_pos: initial_switch,
            logical_mouse_pos: (0.0, 0.0),
            font_mgr: FontMgr::new(),
        }
    }

    fn get_font(&self, size: f32, bold: bool) -> Font {
        let style = if bold { FontStyle::bold() } else { FontStyle::normal() };
        let typeface = self.font_mgr.match_family_style("Segoe UI", style)
            .unwrap_or_else(|| self.font_mgr.legacy_make_typeface(None, style).unwrap());
        Font::from_typeface(typeface, size)
    }

    fn draw(&mut self) {
        let win = self.window.as_ref().unwrap();
        let scale = win.scale_factor() as f32;
        let p_w = (SETTINGS_W * scale) as i32;
        let p_h = (SETTINGS_H * scale) as i32;

        if p_w <= 0 || p_h <= 0 { return; }

        let mut sk_surface = surfaces::raster_n32_premul(skia_safe::ISize::new(p_w, p_h)).unwrap();
        let canvas = sk_surface.canvas();
        canvas.clear(Color::from_rgb(245, 245, 247));
        
        canvas.scale((scale, scale));

        let (config, active_tab, switch_pos) = (self.config.clone(), self.active_tab, self.switch_pos);
        self.draw_tabs(canvas, active_tab);

        if active_tab == 0 {
            self.draw_general(canvas, &config, switch_pos);
        } else {
            self.draw_about(canvas);
        }

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

    fn draw_tabs(&self, canvas: &skia_safe::Canvas, active_tab: usize) {
        let font = self.get_font(14.0, true);
        let mut paint = Paint::default();
        paint.set_anti_alias(true);

        let center_x = SETTINGS_W / 2.0;
        let tabs = ["General", "About"];
        
        for (i, label) in tabs.iter().enumerate() {
            let bx = center_x - 80.0 + (i as f32 * 80.0);
            let rect = Rect::from_xywh(bx, 20.0, 75.0, 32.0);
            
            if active_tab == i {
                paint.set_color(Color::from_rgb(0, 122, 255));
                canvas.draw_round_rect(rect, 8.0, 8.0, &paint);
                paint.set_color(Color::WHITE);
            } else {
                paint.set_color(Color::from_rgb(180, 180, 185));
            }
            
            canvas.draw_str(label, (bx + 12.0, 41.0), &font, &paint);
        }
    }

    fn draw_general(&self, canvas: &skia_safe::Canvas, config: &AppConfig, switch_pos: f32) {
        let font = self.get_font(14.0, false);
        let mut paint = Paint::default();
        paint.set_anti_alias(true);

        let items = [
            ("Global Scale", format!("{:.2}", config.global_scale)),
            ("Base Width", config.base_width.to_string()),
            ("Base Height", config.base_height.to_string()),
            ("Expanded Width", config.expanded_width.to_string()),
            ("Expanded Height", config.expanded_height.to_string()),
        ];

        let start_y = 90.0;
        for (i, (label, val)) in items.iter().enumerate() {
            let y = start_y + (i as f32 * 50.0);
            paint.set_color(Color::from_rgb(50, 50, 50));
            canvas.draw_str(label, (30.0, y + 18.0), &font, &paint);
            
            self.draw_button(canvas, 270.0, y, "-");
            paint.set_color(Color::BLACK);
            let text_x = 315.0 - (val.len() as f32 * 4.0);
            canvas.draw_str(val, (text_x, y + 18.0), &font, &paint);
            self.draw_button(canvas, 345.0, y, "+");
        }

        let sw_y = start_y + (items.len() as f32 * 50.0) + 10.0;
        paint.set_color(Color::from_rgb(50, 50, 50));
        canvas.draw_str("Adaptive Border", (30.0, sw_y + 18.0), &font, &paint);
        self.draw_switch(canvas, 326.0, sw_y, switch_pos);

        paint.set_color(Color::from_rgb(255, 59, 48));
        canvas.draw_str("Reset to Defaults", (center_text_x(SETTINGS_W, "Reset to Defaults", &font), SETTINGS_H - 60.0), &font, &paint);
    }

    fn draw_button(&self, canvas: &skia_safe::Canvas, x: f32, y: f32, label: &str) {
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_color(Color::WHITE);
        let rect = Rect::from_xywh(x, y, 28.0, 28.0);
        canvas.draw_round_rect(rect, 14.0, 14.0, &paint);
        
        paint.set_color(Color::from_rgb(0, 122, 255));
        let font = self.get_font(20.0, true);
        canvas.draw_str(label, (x + 8.0, y + 21.0), &font, &paint);
    }

    fn draw_switch(&self, canvas: &skia_safe::Canvas, x: f32, y: f32, switch_pos: f32) {
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        
        let color_off = Color::from_rgb(200, 200, 205);
        let color_on = Color::from_rgb(52, 199, 89);
        
        let r = color_off.r() as f32 + (color_on.r() as f32 - color_off.r() as f32) * switch_pos;
        let g = color_off.g() as f32 + (color_on.g() as f32 - color_off.g() as f32) * switch_pos;
        let b = color_off.b() as f32 + (color_on.b() as f32 - color_off.b() as f32) * switch_pos;
        
        paint.set_color(Color::from_rgb(r as u8, g as u8, b as u8));
        let rect = Rect::from_xywh(x, y, 48.0, 26.0);
        canvas.draw_round_rect(rect, 13.0, 13.0, &paint);
        
        paint.set_color(Color::WHITE);
        let knob_x = x + 2.0 + (switch_pos * 22.0);
        let knob_rect = Rect::from_xywh(knob_x, y + 2.0, 22.0, 22.0);
        canvas.draw_round_rect(knob_rect, 11.0, 11.0, &paint);
    }

    fn draw_about(&self, canvas: &skia_safe::Canvas) {
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        
        paint.set_color(Color::BLACK);
        let font_title = self.get_font(24.0, true);
        canvas.draw_str("WinIsland", (center_text_x(SETTINGS_W, "WinIsland", &font_title), 150.0), &font_title, &paint);
        
        paint.set_color(Color::from_rgb(100, 100, 100));
        let font_norm = self.get_font(14.0, false);
        let v_str = format!("Version {}", APP_VERSION);
        let a_str = format!("Created by {}", APP_AUTHOR);
        canvas.draw_str(&v_str, (center_text_x(SETTINGS_W, &v_str, &font_norm), 185.0), &font_norm, &paint);
        canvas.draw_str(&a_str, (center_text_x(SETTINGS_W, &a_str, &font_norm), 210.0), &font_norm, &paint);
        
        paint.set_color(Color::from_rgb(0, 122, 255));
        let font_link = self.get_font(12.0, false);
        canvas.draw_str("Visit Project Homepage", (center_text_x(SETTINGS_W, "Visit Project Homepage", &font_link), 260.0), &font_link, &paint);
    }

    fn handle_click(&mut self) {
        let (mx, my) = self.logical_mouse_pos;
        let mut state_changed = false;
        let center_x = SETTINGS_W / 2.0;

        if my >= 10.0 && my <= 60.0 {
            if mx >= center_x - 85.0 && mx <= center_x {
                self.active_tab = 0;
                state_changed = true;
            } else if mx >= center_x && mx <= center_x + 85.0 {
                self.active_tab = 1;
                state_changed = true;
            }
        }

        if self.active_tab == 0 {
            let start_y = 90.0;
            self.check_btn(mx, my, 270.0, start_y, |c| c.global_scale = (c.global_scale - 0.05).max(0.5), &mut state_changed);
            self.check_btn(mx, my, 345.0, start_y, |c| c.global_scale = (c.global_scale + 0.05).min(2.0), &mut state_changed);
            
            self.check_btn(mx, my, 270.0, start_y + 50.0, |c| c.base_width -= 5.0, &mut state_changed);
            self.check_btn(mx, my, 345.0, start_y + 50.0, |c| c.base_width += 5.0, &mut state_changed);
            
            self.check_btn(mx, my, 270.0, start_y + 100.0, |c| c.base_height -= 2.0, &mut state_changed);
            self.check_btn(mx, my, 345.0, start_y + 100.0, |c| c.base_height += 2.0, &mut state_changed);
            
            self.check_btn(mx, my, 270.0, start_y + 150.0, |c| c.expanded_width -= 10.0, &mut state_changed);
            self.check_btn(mx, my, 345.0, start_y + 150.0, |c| c.expanded_width += 10.0, &mut state_changed);
            
            self.check_btn(mx, my, 270.0, start_y + 200.0, |c| c.expanded_height -= 10.0, &mut state_changed);
            self.check_btn(mx, my, 345.0, start_y + 200.0, |c| c.expanded_height += 10.0, &mut state_changed);
            
            let sw_y = start_y + (5.0 * 50.0) + 10.0;
            if mx >= 320.0 && mx <= 380.0 && my >= sw_y && my <= sw_y + 35.0 {
                self.config.adaptive_border = !self.config.adaptive_border;
                state_changed = true;
            }

            if my >= SETTINGS_H - 80.0 && my <= SETTINGS_H - 20.0 {
                if mx >= center_x - 100.0 && mx <= center_x + 100.0 {
                    self.config = AppConfig::default();
                    state_changed = true;
                }
            }
        } else {
            if my >= 240.0 && my <= 280.0 && mx >= center_x - 100.0 && mx <= center_x + 100.0 {
                let _ = open::that(APP_HOMEPAGE);
            }
        }

        if state_changed {
            save_config(&self.config);
            if let Some(win) = &self.window {
                win.request_redraw();
            }
        }
    }

    fn check_btn<F>(&mut self, mx: f32, my: f32, bx: f32, by: f32, mut f: F, state_changed: &mut bool) 
    where F: FnMut(&mut AppConfig) {
        if mx >= bx - 10.0 && mx <= bx + 38.0 && my >= by - 10.0 && my <= by + 38.0 {
            f(&mut self.config);
            self.config.global_scale = (self.config.global_scale * 100.0).round() / 100.0;
            *state_changed = true;
        }
    }
}

fn center_text_x(width: f32, text: &str, font: &Font) -> f32 {
    let (_, rect) = font.measure_str(text, None);
    (width - rect.width()) / 2.0
}

impl ApplicationHandler for SettingsApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);
        let attrs = Window::default_attributes()
            .with_title("WinIsland Settings")
            .with_inner_size(LogicalSize::new(SETTINGS_W as f64, SETTINGS_H as f64))
            .with_resizable(false);
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        self.window = Some(window.clone());
        
        let scale = window.scale_factor();
        let context = Context::new(window.clone()).unwrap();
        let mut surface = Surface::new(&context, window.clone()).unwrap();
        surface.resize(
            std::num::NonZeroU32::new((SETTINGS_W as f64 * scale) as u32).unwrap(),
            std::num::NonZeroU32::new((SETTINGS_H as f64 * scale) as u32).unwrap()
        ).unwrap();
        self.surface = Some(surface);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(win) = &self.window {
                    let scale = win.scale_factor() as f32;
                    self.logical_mouse_pos = (position.x as f32 / scale, position.y as f32 / scale);
                }
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                self.handle_click();
            }
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(surface) = self.surface.as_mut() {
                    surface.resize(
                        std::num::NonZeroU32::new((SETTINGS_W as f64 * scale_factor) as u32).unwrap(),
                        std::num::NonZeroU32::new((SETTINGS_H as f64 * scale_factor) as u32).unwrap()
                    ).unwrap();
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(win) = &self.window {
            let target = if self.config.adaptive_border { 1.0 } else { 0.0 };
            let diff = target - self.switch_pos;
            if diff.abs() > 0.01 {
                self.switch_pos += diff * 0.2;
                win.request_redraw();
            } else if self.switch_pos != target {
                self.switch_pos = target;
                win.request_redraw();
            }
            std::thread::sleep(Duration::from_millis(16));
        }
    }
}

pub fn run_settings(config: AppConfig) {
    let event_loop = EventLoop::new().unwrap();
    let mut app = SettingsApp::new(config);
    event_loop.run_app(&mut app).unwrap();
}
