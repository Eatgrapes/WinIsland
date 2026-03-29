use crate::core::config::{AppConfig, APP_AUTHOR, APP_HOMEPAGE, APP_VERSION};
use crate::core::persistence::save_config;
use crate::core::i18n::{tr, set_lang, current_lang};
use crate::utils::color::*;
use crate::utils::font::FontManager;
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
const SETTINGS_W: f32 = 400.0;
const SETTINGS_H: f32 = 550.0;
use crate::utils::icon::get_app_icon;
use crate::utils::autostart::set_autostart;

pub struct SettingsApp {
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    sk_surface: Option<skia_safe::Surface>,
    config: AppConfig,
    active_tab: usize,
    border_switch_pos: f32,
    blur_switch_pos: f32,
    autostart_switch_pos: f32,
    update_switch_pos: f32,
    logical_mouse_pos: (f32, f32),
    frame_count: u64,
    scroll_y: f32,
    target_scroll_y: f32,
}
impl SettingsApp {
    pub fn new(config: AppConfig) -> Self {
        let initial_border = if config.adaptive_border { 1.0 } else { 0.0 };
        let initial_blur = if config.motion_blur { 1.0 } else { 0.0 };
        let initial_autostart = if config.auto_start { 1.0 } else { 0.0 };
        let initial_update = if config.check_for_updates { 1.0 } else { 0.0 };
        Self {
            window: None,
            surface: None,
            sk_surface: None,
            config,
            active_tab: 0,
            border_switch_pos: initial_border,
            blur_switch_pos: initial_blur,
            autostart_switch_pos: initial_autostart,
            update_switch_pos: initial_update,
            logical_mouse_pos: (0.0, 0.0),
            frame_count: 0,
            scroll_y: 0.0,
            target_scroll_y: 0.0,
        }
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
            canvas.save();
            canvas.clip_rect(Rect::from_xywh(0.0, 70.0, SETTINGS_W, SETTINGS_H - 70.0), skia_safe::ClipOp::Intersect, true);
            canvas.translate((0.0, -self.scroll_y));
            self.draw_general(canvas);
            canvas.restore();

            let content_h = if self.config.auto_hide { 900.0 } else { 850.0 };
            let view_h = SETTINGS_H - 70.0;
            if content_h > view_h {
                let bar_h = (view_h / content_h) * view_h;
                let bar_y = 70.0 + (self.scroll_y / (content_h - view_h)) * (view_h - bar_h);
                let mut p = Paint::default();
                p.set_anti_alias(true);
                p.set_color(Color::from_argb(80, 255, 255, 255));
                canvas.draw_round_rect(Rect::from_xywh(SETTINGS_W - 6.0, bar_y, 4.0, bar_h), 2.0, 2.0, &p);
            }
        } else {
            self.draw_about(canvas);
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
    fn draw_general(&self, canvas: &skia_safe::Canvas) {
        let fm = FontManager::global();
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        let items = [
            (tr("global_scale"), format!("{:.2}", self.config.global_scale)),
            (tr("base_width"), self.config.base_width.to_string()),
            (tr("base_height"), self.config.base_height.to_string()),
            (tr("expanded_width"), self.config.expanded_width.to_string()),
            (tr("expanded_height"), self.config.expanded_height.to_string()),
        ];
        let start_y = 90.0;
        for (i, (label, val)) in items.iter().enumerate() {
            let y = start_y + (i as f32 * 50.0);
            paint.set_color(COLOR_CARD);
            canvas.draw_round_rect(Rect::from_xywh(20.0, y - 5.0, SETTINGS_W - 40.0, 42.0), 10.0, 10.0, &paint);
            paint.set_color(COLOR_TEXT_PRI);
            fm.draw_text(canvas, label, (35.0, y + 21.0), 14.0, false, &paint);
            self.draw_button(canvas, 270.0, y + 2.0, "-");
            paint.set_color(COLOR_TEXT_PRI);
            fm.draw_text_centered(canvas, val, 325.0, y + 21.0, 14.0, false, &paint);
            self.draw_button(canvas, 345.0, y + 2.0, "+");
        }
        let sw_border_y = start_y + (items.len() as f32 * 50.0) + 10.0;
        paint.set_color(COLOR_CARD);
        canvas.draw_round_rect(Rect::from_xywh(20.0, sw_border_y - 5.0, SETTINGS_W - 40.0, 42.0), 10.0, 10.0, &paint);
        paint.set_color(COLOR_TEXT_PRI);
        fm.draw_text(canvas, &tr("adaptive_border"), (35.0, sw_border_y + 21.0), 14.0, false, &paint);
        self.draw_switch(canvas, 326.0, sw_border_y + 3.0, self.border_switch_pos);

        let sw_blur_y = sw_border_y + 50.0;
        paint.set_color(COLOR_CARD);
        canvas.draw_round_rect(Rect::from_xywh(20.0, sw_blur_y - 5.0, SETTINGS_W - 40.0, 42.0), 10.0, 10.0, &paint);
        paint.set_color(COLOR_TEXT_PRI);
        fm.draw_text(canvas, &tr("motion_blur"), (35.0, sw_blur_y + 21.0), 14.0, false, &paint);
        self.draw_switch(canvas, 326.0, sw_blur_y + 3.0, self.blur_switch_pos);

        let font_y = sw_blur_y + 50.0;
        paint.set_color(COLOR_CARD);
        canvas.draw_round_rect(Rect::from_xywh(20.0, font_y - 5.0, SETTINGS_W - 40.0, 42.0), 10.0, 10.0, &paint);
        paint.set_color(COLOR_TEXT_PRI);
        fm.draw_text(canvas, &tr("custom_font"), (35.0, font_y + 21.0), 14.0, false, &paint);
        self.draw_text_button(canvas, 310.0, font_y + 3.0, 65.0, 26.0, &tr("font_select"));
        if self.config.custom_font_path.is_some() {
            self.draw_text_button_danger(canvas, 235.0, font_y + 3.0, 65.0, 26.0, &tr("font_reset"));
        }

        let autostart_y = font_y + 50.0;
        paint.set_color(COLOR_CARD);
        canvas.draw_round_rect(Rect::from_xywh(20.0, autostart_y - 5.0, SETTINGS_W - 40.0, 42.0), 10.0, 10.0, &paint);
        paint.set_color(COLOR_TEXT_PRI);
        fm.draw_text(canvas, &tr("start_boot"), (35.0, autostart_y + 21.0), 14.0, false, &paint);
        self.draw_switch(canvas, 326.0, autostart_y + 3.0, self.autostart_switch_pos);

        let autohide_y = autostart_y + 50.0;
        paint.set_color(COLOR_CARD);
        canvas.draw_round_rect(Rect::from_xywh(20.0, autohide_y - 5.0, SETTINGS_W - 40.0, 42.0), 10.0, 10.0, &paint);
        paint.set_color(COLOR_TEXT_PRI);
        fm.draw_text(canvas, &tr("auto_hide"), (35.0, autohide_y + 21.0), 14.0, false, &paint);
        self.draw_switch(canvas, 326.0, autohide_y + 3.0, if self.config.auto_hide { 1.0 } else { 0.0 });

        let update_y = autohide_y + 50.0;
        paint.set_color(COLOR_CARD);
        canvas.draw_round_rect(Rect::from_xywh(20.0, update_y - 5.0, SETTINGS_W - 40.0, 42.0), 10.0, 10.0, &paint);
        paint.set_color(COLOR_TEXT_PRI);
        fm.draw_text(canvas, &tr("check_updates"), (35.0, update_y + 21.0), 14.0, false, &paint);
        self.draw_switch(canvas, 326.0, update_y + 3.0, self.update_switch_pos);

        let interval_y = update_y + 50.0;
        paint.set_color(COLOR_CARD);
        canvas.draw_round_rect(Rect::from_xywh(20.0, interval_y - 5.0, SETTINGS_W - 40.0, 42.0), 10.0, 10.0, &paint);
        paint.set_color(if self.config.check_for_updates { COLOR_TEXT_PRI } else { COLOR_TEXT_SEC });
        fm.draw_text(canvas, &tr("update_interval"), (35.0, interval_y + 21.0), 14.0, false, &paint);
        let interval_str = format!("{:.0}", self.config.update_check_interval);
        self.draw_button(canvas, 270.0, interval_y + 2.0, "-");
        fm.draw_text_centered(canvas, &interval_str, 325.0, interval_y + 21.0, 14.0, false, &paint);
        self.draw_button(canvas, 345.0, interval_y + 2.0, "+");

        let lang_y = interval_y + 50.0;
        paint.set_color(COLOR_CARD);
        canvas.draw_round_rect(Rect::from_xywh(20.0, lang_y - 5.0, SETTINGS_W - 40.0, 42.0), 10.0, 10.0, &paint);
        paint.set_color(COLOR_TEXT_PRI);
        fm.draw_text(canvas, &tr("language"), (35.0, lang_y + 21.0), 14.0, false, &paint);
        self.draw_text_button(canvas, 300.0, lang_y + 3.0, 75.0, 26.0, &tr("lang_name"));

        let delay_y = lang_y + 50.0;
        paint.set_color(COLOR_CARD);
        canvas.draw_round_rect(Rect::from_xywh(20.0, delay_y - 5.0, SETTINGS_W - 40.0, 42.0), 10.0, 10.0, &paint);
        paint.set_color(if self.config.auto_hide { COLOR_TEXT_PRI } else { COLOR_TEXT_SEC });
        fm.draw_text(canvas, &tr("hide_delay"), (35.0, delay_y + 21.0), 14.0, false, &paint);
        let delay_str = format!("{:.0}", self.config.auto_hide_delay);
        self.draw_button(canvas, 270.0, delay_y + 2.0, "-");
        fm.draw_text_centered(canvas, &delay_str, 325.0, delay_y + 21.0, 14.0, false, &paint);
        self.draw_button(canvas, 345.0, delay_y + 2.0, "+");

        paint.set_color(COLOR_DANGER);
        let reset_str = tr("reset_defaults");
        let reset_y = delay_y + 60.0;
        fm.draw_text_centered(canvas, &reset_str, SETTINGS_W / 2.0, reset_y, 14.0, false, &paint);
    }
    fn draw_text_button_danger(&self, canvas: &skia_safe::Canvas, x: f32, y: f32, w: f32, h: f32, label: &str) {
        let fm = FontManager::global();
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_color(COLOR_CARD_HIGHLIGHT);
        canvas.draw_round_rect(Rect::from_xywh(x, y, w, h), h/2.0, h/2.0, &paint);
        paint.set_color(COLOR_DANGER);
        fm.draw_text_in_rect(canvas, label, x, y + 17.0, w, 12.0, true, &paint);
    }
    fn draw_text_button(&self, canvas: &skia_safe::Canvas, x: f32, y: f32, w: f32, h: f32, label: &str) {
        let fm = FontManager::global();
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_color(COLOR_CARD_HIGHLIGHT);
        canvas.draw_round_rect(Rect::from_xywh(x, y, w, h), h/2.0, h/2.0, &paint);
        paint.set_color(COLOR_TEXT_PRI);
        fm.draw_text_in_rect(canvas, label, x, y + 17.0, w, 12.0, true, &paint);
    }
    fn draw_button(&self, canvas: &skia_safe::Canvas, x: f32, y: f32, label: &str) {
        let fm = FontManager::global();
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_color(COLOR_CARD_HIGHLIGHT);
        canvas.draw_round_rect(Rect::from_xywh(x, y, 28.0, 28.0), 14.0, 14.0, &paint);
        paint.set_color(COLOR_TEXT_PRI);
        fm.draw_text_in_rect(canvas, label, x, y + 20.0, 28.0, 20.0, false, &paint);
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
    fn draw_about(&self, canvas: &skia_safe::Canvas) {
        let fm = FontManager::global();
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_color(COLOR_TEXT_PRI);
        fm.draw_text_centered(canvas, "WinIsland", SETTINGS_W / 2.0, 160.0, 28.0, true, &paint);
        paint.set_color(COLOR_TEXT_SEC);
        let v_str = format!("Version {}", APP_VERSION);
        fm.draw_text_centered(canvas, &v_str, SETTINGS_W / 2.0, 195.0, 14.0, false, &paint);
        let a_str = format!("{} {}", tr("created_by"), APP_AUTHOR);
        fm.draw_text_centered(canvas, &a_str, SETTINGS_W / 2.0, 220.0, 14.0, false, &paint);
        paint.set_color(COLOR_ACCENT);
        let link_str = tr("visit_homepage");
        fm.draw_text_centered(canvas, &link_str, SETTINGS_W / 2.0, 280.0, 14.0, false, &paint);
    }
    fn handle_click(&mut self) {
        let (mx, my) = self.logical_mouse_pos;
        let mut changed = false;
        let cx = SETTINGS_W / 2.0;
        
        let win = self.window.as_ref().unwrap();
        let scale = win.scale_factor() as f32;
        let size = win.inner_size();
        let dx = ((size.width as f32 / scale) - SETTINGS_W) / 2.0;
        let dy = ((size.height as f32 / scale) - SETTINGS_H) / 2.0;
        let lmx = mx - dx;
        let lmy = my - dy;

        let content_my = if self.active_tab == 0 && lmy >= 70.0 {
            lmy + self.scroll_y
        } else {
            lmy
        };

        if lmy >= 20.0 && lmy <= 56.0 {
            if lmx >= cx - 85.0 && lmx <= cx { self.active_tab = 0; changed = true; }
            else if lmx >= cx && lmx <= cx + 85.0 { self.active_tab = 1; changed = true; }
        }
        if self.active_tab == 0 {
            let sy = 90.0;
            self.check_btn(lmx, content_my, 270.0, sy + 2.0, |c| {
                c.global_scale = (c.global_scale - 0.05).max(0.5);
                c.global_scale = (c.global_scale * 100.0).round() / 100.0;
            }, &mut changed);
            self.check_btn(lmx, content_my, 345.0, sy + 2.0, |c| {
                c.global_scale = (c.global_scale + 0.05).min(5.0);
                c.global_scale = (c.global_scale * 100.0).round() / 100.0;
            }, &mut changed);
            self.check_btn(lmx, content_my, 270.0, sy + 52.0, |c| c.base_width -= 5.0, &mut changed);
            self.check_btn(lmx, content_my, 345.0, sy + 52.0, |c| c.base_width += 5.0, &mut changed);
            self.check_btn(lmx, content_my, 270.0, sy + 102.0, |c| c.base_height -= 2.0, &mut changed);
            self.check_btn(lmx, content_my, 345.0, sy + 102.0, |c| c.base_height += 2.0, &mut changed);
            self.check_btn(lmx, content_my, 270.0, sy + 152.0, |c| c.expanded_width -= 10.0, &mut changed);
            self.check_btn(lmx, content_my, 345.0, sy + 152.0, |c| c.expanded_width += 10.0, &mut changed);
            self.check_btn(lmx, content_my, 270.0, sy + 202.0, |c| c.expanded_height -= 10.0, &mut changed);
            self.check_btn(lmx, content_my, 345.0, sy + 202.0, |c| c.expanded_height += 10.0, &mut changed);

            let sw_border_y = sy + 260.0;
            if Self::in_rect(lmx, content_my, 326.0, sw_border_y + 3.0, 48.0, 26.0) {
                self.config.adaptive_border = !self.config.adaptive_border;
                changed = true;
            }
            if Self::in_rect(lmx, content_my, 326.0, sw_border_y + 53.0, 48.0, 26.0) {
                self.config.motion_blur = !self.config.motion_blur;
                changed = true;
            }
            let font_y = sw_border_y + 100.0;
            if Self::in_rect(lmx, content_my, 310.0, font_y + 3.0, 65.0, 26.0) {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Fonts", &["ttf", "otf"])
                    .pick_file() {
                    self.config.custom_font_path = Some(path.to_string_lossy().into_owned());
                    FontManager::global().refresh_custom_font();
                    changed = true;
                }
            }
            if self.config.custom_font_path.is_some() && Self::in_rect(lmx, content_my, 235.0, font_y + 3.0, 65.0, 26.0) {
                self.config.custom_font_path = None;
                FontManager::global().refresh_custom_font();
                changed = true;
            }

            let autostart_y = font_y + 50.0;
            if Self::in_rect(lmx, content_my, 326.0, autostart_y + 3.0, 48.0, 26.0) {
                self.config.auto_start = !self.config.auto_start;
                let _ = set_autostart(self.config.auto_start);
                changed = true;
            }
            let autohide_y = autostart_y + 50.0;
            if Self::in_rect(lmx, content_my, 326.0, autohide_y + 3.0, 48.0, 26.0) {
                self.config.auto_hide = !self.config.auto_hide;
                changed = true;
            }
            let update_y = autohide_y + 50.0;
            if Self::in_rect(lmx, content_my, 326.0, update_y + 3.0, 48.0, 26.0) {
                self.config.check_for_updates = !self.config.check_for_updates;
                changed = true;
            }
            let interval_y = update_y + 50.0;
            if self.config.check_for_updates {
                self.check_btn(lmx, content_my, 270.0, interval_y + 2.0, |c| c.update_check_interval = (c.update_check_interval - 1.0).max(1.0), &mut changed);
                self.check_btn(lmx, content_my, 345.0, interval_y + 2.0, |c| c.update_check_interval = (c.update_check_interval + 1.0).min(24.0), &mut changed);
            }
            let lang_y = interval_y + 50.0;
            if Self::in_rect(lmx, content_my, 300.0, lang_y + 3.0, 75.0, 26.0) {
                self.config.language = if current_lang() == "zh" { "en".to_string() } else { "zh".to_string() };
                set_lang(&self.config.language);
                changed = true;
            }
            let delay_y = lang_y + 50.0;
            if self.config.auto_hide {
                self.check_btn(lmx, content_my, 270.0, delay_y + 2.0, |c| c.auto_hide_delay = (c.auto_hide_delay - 1.0).max(1.0), &mut changed);
                self.check_btn(lmx, content_my, 345.0, delay_y + 2.0, |c| c.auto_hide_delay = (c.auto_hide_delay + 1.0).min(60.0), &mut changed);
            }
            let reset_y = if self.config.auto_hide { 860.0 } else { 810.0 };
            if lmx >= cx - 100.0 && lmx <= cx + 100.0 && content_my >= reset_y - 24.0 && content_my <= reset_y + 12.0 {
                self.config = AppConfig::default();
                set_lang(if self.config.language == "auto" { "en" } else { &self.config.language });
                FontManager::global().refresh_custom_font();
                changed = true;
            }
        } else if lmy >= 260.0 && lmy <= 300.0 && lmx >= cx - 100.0 && lmx <= cx + 100.0 {
            let _ = open::that(APP_HOMEPAGE);
        }
        if changed {
            let content_h = if self.config.auto_hide { 900.0 } else { 850.0 };
            let max_scroll = (content_h - (SETTINGS_H - 70.0)).max(0.0);
            self.target_scroll_y = self.target_scroll_y.clamp(0.0, max_scroll);
            self.scroll_y = self.scroll_y.clamp(0.0, max_scroll);
            save_config(&self.config);
            if let Some(win) = &self.window { win.request_redraw(); }
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

        let content_my = if self.active_tab == 0 && lmy >= 70.0 {
            lmy + self.scroll_y
        } else {
            lmy
        };

        if lmy >= 20.0 && lmy <= 56.0 {
            if lmx >= cx - 85.0 && lmx <= cx + 85.0 { return true; }
        }
        if self.active_tab == 0 {
            let sy = 90.0;
            if Self::in_rect(lmx, content_my, 270.0, sy + 2.0, 28.0, 28.0) { return true; }
            if Self::in_rect(lmx, content_my, 345.0, sy + 2.0, 28.0, 28.0) { return true; }
            if Self::in_rect(lmx, content_my, 270.0, sy + 52.0, 28.0, 28.0) { return true; }
            if Self::in_rect(lmx, content_my, 345.0, sy + 52.0, 28.0, 28.0) { return true; }
            if Self::in_rect(lmx, content_my, 270.0, sy + 102.0, 28.0, 28.0) { return true; }
            if Self::in_rect(lmx, content_my, 345.0, sy + 102.0, 28.0, 28.0) { return true; }
            if Self::in_rect(lmx, content_my, 270.0, sy + 152.0, 28.0, 28.0) { return true; }
            if Self::in_rect(lmx, content_my, 345.0, sy + 152.0, 28.0, 28.0) { return true; }
            if Self::in_rect(lmx, content_my, 270.0, sy + 202.0, 28.0, 28.0) { return true; }
            if Self::in_rect(lmx, content_my, 345.0, sy + 202.0, 28.0, 28.0) { return true; }

            let sw_border_y = sy + 260.0;
            if Self::in_rect(lmx, content_my, 326.0, sw_border_y + 3.0, 48.0, 26.0) { return true; }
            if Self::in_rect(lmx, content_my, 326.0, sw_border_y + 53.0, 48.0, 26.0) { return true; }
            let font_y = sw_border_y + 100.0;
            if Self::in_rect(lmx, content_my, 310.0, font_y + 3.0, 65.0, 26.0) { return true; }
            if self.config.custom_font_path.is_some() && Self::in_rect(lmx, content_my, 235.0, font_y + 3.0, 65.0, 26.0) { return true; }

            let autostart_y = font_y + 50.0;
            if Self::in_rect(lmx, content_my, 326.0, autostart_y + 3.0, 48.0, 26.0) { return true; }
            let autohide_y = autostart_y + 50.0;
            if Self::in_rect(lmx, content_my, 326.0, autohide_y + 3.0, 48.0, 26.0) { return true; }
            let update_y = autohide_y + 50.0;
            if Self::in_rect(lmx, content_my, 326.0, update_y + 3.0, 48.0, 26.0) { return true; }
            let interval_y = update_y + 50.0;
            if self.config.check_for_updates {
                if Self::in_rect(lmx, content_my, 270.0, interval_y + 2.0, 28.0, 28.0) { return true; }
                if Self::in_rect(lmx, content_my, 345.0, interval_y + 2.0, 28.0, 28.0) { return true; }
            }
            let lang_y = interval_y + 50.0;
            if Self::in_rect(lmx, content_my, 300.0, lang_y + 3.0, 75.0, 26.0) { return true; }
            let delay_y = lang_y + 50.0;
            if self.config.auto_hide {
                if Self::in_rect(lmx, content_my, 270.0, delay_y + 2.0, 28.0, 28.0) { return true; }
                if Self::in_rect(lmx, content_my, 345.0, delay_y + 2.0, 28.0, 28.0) { return true; }
            }
            let reset_y = delay_y + 60.0;
            if lmx >= cx - 100.0 && lmx <= cx + 100.0 && content_my >= reset_y - 24.0 && content_my <= reset_y + 12.0 { return true; }
        } else if lmy >= 260.0 && lmy <= 300.0 && lmx >= cx - 100.0 && lmx <= cx + 100.0 {
            return true;
        }
        false
    }
    fn in_rect(mx: f32, my: f32, x: f32, y: f32, w: f32, h: f32) -> bool {
        mx >= x && mx <= x + w && my >= y && my <= y + h
    }
    fn check_btn<F>(&mut self, mx: f32, my: f32, bx: f32, by: f32, mut f: F, changed: &mut bool) 
    where F: FnMut(&mut AppConfig) {
        if mx >= bx && mx <= bx + 28.0 && my >= by && my <= by + 28.0 {
            f(&mut self.config);
            *changed = true;
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
                    if let Key::Named(NamedKey::F11) = event.logical_key {
                        // Ignore F11
                    }
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
                    let content_h = if self.config.auto_hide { 900.0 } else { 850.0 };
                    let max_scroll = (content_h - (SETTINGS_H - 70.0)).max(0.0);
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
            let mut redraw = false;
            let tb = if self.config.adaptive_border { 1.0 } else { 0.0 };
            if (tb - self.border_switch_pos).abs() > 0.01 { self.border_switch_pos += (tb - self.border_switch_pos) * 0.2; redraw = true; }
            let tbu = if self.config.motion_blur { 1.0 } else { 0.0 };
            if (tbu - self.blur_switch_pos).abs() > 0.01 { self.blur_switch_pos += (tbu - self.blur_switch_pos) * 0.2; redraw = true; }
            let tas = if self.config.auto_start { 1.0 } else { 0.0 };
            if (tas - self.autostart_switch_pos).abs() > 0.01 { self.autostart_switch_pos += (tas - self.autostart_switch_pos) * 0.2; redraw = true; }
            let tcu = if self.config.check_for_updates { 1.0 } else { 0.0 };
            if (tcu - self.update_switch_pos).abs() > 0.01 { self.update_switch_pos += (tcu - self.update_switch_pos) * 0.2; redraw = true; }
            let content_h = if self.config.auto_hide { 900.0 } else { 850.0 };
            let max_scroll = (content_h - (SETTINGS_H - 70.0)).max(0.0);
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
