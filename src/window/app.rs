use std::sync::Arc;
use std::time::{Duration, Instant};
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::platform::windows::WindowAttributesExtWindows;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{Window, WindowId, WindowLevel, WindowButtons};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, GWL_STYLE, WS_EX_TOOLWINDOW, WS_EX_NOACTIVATE, WS_MAXIMIZEBOX, WS_THICKFRAME};
use crate::core::config::{AppConfig, PADDING, TOP_OFFSET, WINDOW_TITLE};
use crate::core::persistence::load_config;
use crate::core::render::draw_island;
use crate::utils::blur::calculate_blur_sigmas;
use crate::utils::color::get_island_border_weights;
use crate::utils::mouse::{get_global_cursor_pos, is_point_in_rect, is_left_button_pressed};
use crate::utils::physics::Spring;
use crate::core::smtc::SmtcListener;
use crate::core::audio::AudioProcessor;
use crate::window::tray::{TrayAction, TrayManager};
use crate::utils::icon::get_app_icon;
use crate::ui::expanded::main_view::{get_progress_bar_rect, get_pause_btn_rect};

pub struct App {
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    tray: Option<TrayManager>,
    smtc: SmtcListener,
    audio: AudioProcessor,
    config: AppConfig,
    expanded: bool,
    widget_view: bool,
    visible: bool,
    border_weights: [f32; 4],
    target_border_weights: [f32; 4],
    spring_w: Spring,
    spring_h: Spring,
    spring_r: Spring,
    spring_view: Spring,
    os_w: u32,
    os_h: u32,
    win_x: i32,
    win_y: i32,
    frame_count: u64,
    last_media_title: String,
    last_media_playing: bool,
    last_playing_time: Instant,
    current_lyric_text: String,
    old_lyric_text: String,
    lyric_transition: f32,
    idle_timer: Instant,
    spring_hide: Spring,
    auto_hidden: bool,
    is_dragging: bool,
    drag_start_py: i32,
    drag_start_hide_val: f32,
    manually_hidden: bool,
    drag_has_moved: bool,
    last_frame_time: Instant,
    last_mon_size: (u32, u32),
    last_mon_pos: (i32, i32),
    lyric_scroll_offset: f32,
    lyric_scroll_pause: f32,
    seeking_progress: bool,
    seeking_bar_left: f32,
    seeking_bar_right: f32,
    seeking_duration_ms: u64,
}

impl Default for App {
    fn default() -> Self {
        let config = load_config();
        Self {
            window: None,
            surface: None,
            tray: None,
            config: config.clone(),
            expanded: false,
            widget_view: false,
            visible: true,
            border_weights: [0.0; 4],
            target_border_weights: [0.0; 4],
            spring_w: Spring::new(config.base_width * config.global_scale),
            spring_h: Spring::new(config.base_height * config.global_scale),
            spring_r: Spring::new((config.base_height * config.global_scale) / 2.0),
            spring_view: Spring::new(0.0),
            smtc: SmtcListener::new(config.lyrics_source.clone(), config.lyrics_fallback, config.smtc_apps.clone()),
            audio: AudioProcessor::new(),
            os_w: 0,
            os_h: 0,
            win_x: 0,
            win_y: 0,
            frame_count: 0,
            last_media_title: String::new(),
            last_media_playing: false,
            last_playing_time: Instant::now(),
            current_lyric_text: String::new(),
            old_lyric_text: String::new(),
            lyric_transition: 1.0,
            idle_timer: Instant::now(),
            spring_hide: Spring::new(0.0),
            auto_hidden: false,
            is_dragging: false,
            drag_start_py: 0,
            drag_start_hide_val: 0.0,
            manually_hidden: false,
            drag_has_moved: false,
            last_frame_time: Instant::now(),
            last_mon_size: (0, 0),
            last_mon_pos: (0, 0),
            lyric_scroll_offset: 0.0,
            lyric_scroll_pause: 0.0,
            seeking_progress: false,
            seeking_bar_left: 0.0,
            seeking_bar_right: 0.0,
            seeking_duration_ms: 0,
        }
    }
}

impl App {
    fn get_target_monitor(window: &Window, monitor_index: i32) -> Option<winit::monitor::MonitorHandle> {
        if monitor_index <= 0 {
            return window.primary_monitor().or_else(|| window.current_monitor());
        }
        let monitors: Vec<_> = window.available_monitors().collect();
        let idx = monitor_index as usize;
        if idx < monitors.len() {
            Some(monitors[idx].clone())
        } else {
            window.primary_monitor().or_else(|| window.current_monitor())
        }
    }

    fn enforce_topmost(window: &Window, win_x: i32, win_y: i32, os_w: u32, os_h: u32) {
        if let Ok(handle) = window.window_handle() {
            if let RawWindowHandle::Win32(raw) = handle.as_raw() {
                let hwnd = HWND(raw.hwnd.get() as *mut core::ffi::c_void);
                unsafe {
                    let _ = SetWindowPos(
                        hwnd,
                        HWND_TOPMOST,
                        win_x,
                        win_y,
                        os_w as i32,
                        os_h as i32,
                        SWP_NOACTIVATE,
                    );
                }
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);
        if self.window.is_none() {
            let max_w = self.config.expanded_width.max(450.0);
            self.os_w = (max_w * self.config.global_scale + PADDING) as u32;
            self.os_h = (self.config.expanded_height * self.config.global_scale + PADDING) as u32;
            let attrs = Window::default_attributes()
                .with_title(WINDOW_TITLE)
                .with_inner_size(PhysicalSize::new(self.os_w, self.os_h))
                .with_transparent(true)
                .with_decorations(false)
                .with_resizable(false)
                .with_enabled_buttons(WindowButtons::empty())
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_skip_taskbar(true)
                .with_window_icon(get_app_icon());
            let window = Arc::new(event_loop.create_window(attrs).unwrap());

            if let Ok(handle) = window.window_handle() {
                if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
                    let hwnd = HWND(win32_handle.hwnd.get() as _);
                    unsafe {
                        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_TOOLWINDOW.0 as isize | WS_EX_NOACTIVATE.0 as isize);
                        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                        SetWindowLongPtrW(hwnd, GWL_STYLE, style & !(WS_MAXIMIZEBOX.0 as isize | WS_THICKFRAME.0 as isize));
                    }
                }
            }

            self.window = Some(window.clone());
            if let Some(monitor) = Self::get_target_monitor(&window, self.config.monitor_index) {
                let mon_size = monitor.size();
                let mon_pos = monitor.position();
                self.last_mon_size = (mon_size.width, mon_size.height);
                self.last_mon_pos = (mon_pos.x, mon_pos.y);
                let center_x = mon_pos.x + (mon_size.width as i32) / 2;
                let top_y = mon_pos.y + TOP_OFFSET;
                self.win_x = center_x - (self.os_w as i32) / 2 + self.config.position_x_offset;
                self.win_y = top_y - (PADDING / 2.0) as i32 + self.config.position_y_offset;
                window.set_outer_position(PhysicalPosition::new(self.win_x, self.win_y));
            }
            let context = Context::new(window.clone()).unwrap();
            let mut surface = Surface::new(&context, window.clone()).unwrap();
            surface
                .resize(
                    std::num::NonZeroU32::new(self.os_w).unwrap(),
                    std::num::NonZeroU32::new(self.os_h).unwrap(),
                )
                .unwrap();
            self.surface = Some(surface);
            let is_light = window.theme() == Some(winit::window::Theme::Light);
            self.tray = Some(TrayManager::new(is_light));
            Self::enforce_topmost(&window, self.win_x, self.win_y, self.os_w, self.os_h);
            window.request_redraw();
        }
    }
    fn window_event(&mut self, _event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if let Some(win) = &self.window {
            if win.id() == id {
                match event {
                    WindowEvent::ThemeChanged(theme) => {
                        let is_light = theme == winit::window::Theme::Light;
                        if let Some(tray) = self.tray.as_mut() {
                            tray.update_theme(is_light);
                        }
                    }
                    WindowEvent::Resized(_) => {
                        if win.is_maximized() {
                            win.set_maximized(false);
                        }
                    }
                    WindowEvent::CloseRequested => (),
                    WindowEvent::MouseInput {
                        state,
                        button: MouseButton::Left,
                        ..
                    } => {
                        let (px, py) = get_global_cursor_pos();
                        let rel_x = px - self.win_x;
                        let rel_y = py - self.win_y;
                        let island_y = PADDING as f64 / 2.0;
                        let offset_x = (self.os_w as f64 - self.spring_w.value as f64) / 2.0;
                        let scale = self.config.global_scale as f64;
                        
                        let hidden_peek_h = (5.0 * scale).max(3.0);
                        let hide_distance = (self.spring_h.value as f64 - hidden_peek_h + TOP_OFFSET as f64).max(0.0);
                        let hide_y_offset = self.spring_hide.value as f64 * hide_distance;
                        let current_island_y = island_y - hide_y_offset;
                        
                        let is_hovering_visible = is_point_in_rect(
                            rel_x as f64,
                            rel_y as f64,
                            offset_x,
                            current_island_y,
                            self.spring_w.value as f64,
                            self.spring_h.value as f64,
                        );

                        let hidden_handle_h = (24.0 * scale).max(14.0);
                        let hidden_handle_y = (current_island_y + self.spring_h.value as f64 - hidden_peek_h - hidden_handle_h * 0.35).max(0.0);
                        let is_on_hidden_handle = (self.auto_hidden || self.manually_hidden) && is_point_in_rect(
                            rel_x as f64,
                            rel_y as f64,
                            offset_x,
                            hidden_handle_y,
                            self.spring_w.value as f64,
                            hidden_handle_h,
                        );

                        if state == ElementState::Pressed {
                            if self.expanded {
                                let view_val = self.spring_view.value as f64;
                                let w = self.spring_w.value as f64;
                                let h = self.spring_h.value as f64;
                                let page_shift = view_val * w;

                                if view_val < 0.5 {
                                    let media = self.smtc.get_info();
                                    let music_on = self.config.smtc_enabled && !media.title.is_empty()
                                        && (media.is_playing || self.last_playing_time.elapsed() < Duration::from_secs(5));

                                    let (bx, by, bw, bh) = get_pause_btn_rect(
                                        offset_x as f32, island_y as f32, w as f32, h as f32,
                                        self.config.global_scale
                                    );
                                    let cx = rel_x as f32 - (page_shift as f32);
                                    let cy = rel_y as f32;
                                    if music_on && cx >= bx && cx <= bx + bw && cy >= by && cy <= by + bh {
                                        self.smtc.request_toggle_play();
                                        return;
                                    }

                                    if let Some((bar_left, bar_right, bar_top, bar_hit_h)) = get_progress_bar_rect(
                                        offset_x as f32, island_y as f32, w as f32, &media,
                                        music_on,
                                        self.config.global_scale
                                    ) {
                                        if cx >= bar_left && cx <= bar_right && cy >= bar_top && cy <= bar_top + bar_hit_h {
                                            let ratio = ((cx - bar_left) / (bar_right - bar_left)).clamp(0.0, 1.0);
                                            let seek_ms = (ratio as f64 * media.duration_secs as f64 * 1000.0) as u64;
                                            self.smtc.request_seek(seek_ms);
                                            self.seeking_progress = true;
                                            self.seeking_bar_left = bar_left;
                                            self.seeking_bar_right = bar_right;
                                            self.seeking_duration_ms = media.duration_secs * 1000;
                                            return;
                                        }
                                    }
                                }

                                if view_val > 0.5 {
                                    let gear_x = offset_x + w - 28.0 * scale + w - page_shift;
                                    let gear_y = island_y + h - 28.0 * scale;
                                    let dist_sq = (rel_x as f64 - gear_x).powi(2) + (rel_y as f64 - gear_y).powi(2);
                                    if dist_sq <= (20.0 * scale).powi(2) {
                                        let _ = std::process::Command::new(std::env::current_exe().unwrap())
                                            .arg("--settings")
                                            .spawn();
                                        return;
                                    }

                                    let arrow_x = offset_x + 12.0 * scale + w - page_shift;
                                    let arrow_y = island_y + h / 2.0;
                                    let adx = rel_x as f64 - arrow_x;
                                    let ady = rel_y as f64 - arrow_y;
                                    if adx * adx + ady * ady <= (20.0 * scale).powi(2) {
                                        self.widget_view = false;
                                        return;
                                    }
                                }

                                if view_val < 0.5 {
                                    let arrow_x = offset_x + w - 12.0 * scale;
                                    let arrow_y = island_y + h / 2.0;
                                    let adx = rel_x as f64 - arrow_x;
                                    let ady = rel_y as f64 - arrow_y;
                                    if adx * adx + ady * ady <= (20.0 * scale).powi(2) {
                                        self.widget_view = true;
                                        return;
                                    }
                                }

                                if (rel_y as f64) < island_y + 40.0 * scale {
                                    self.expanded = false;
                                    self.widget_view = false;
                                }
                            } else {
                                if is_hovering_visible || is_on_hidden_handle {
                                    self.is_dragging = true;
                                    self.drag_start_py = py;
                                    self.drag_start_hide_val = self.spring_hide.value;
                                    self.drag_has_moved = false;
                                }
                            }
                        } else if state == ElementState::Released {
                            if self.seeking_progress {
                                self.seeking_progress = false;
                                return;
                            }
                            if self.is_dragging {
                                self.is_dragging = false;
                                if !self.drag_has_moved {
                                    if self.auto_hidden || self.manually_hidden {
                                        self.auto_hidden = false;
                                        self.manually_hidden = false;
                                        self.spring_hide.velocity = -0.45;
                                        self.idle_timer = Instant::now();
                                    } else {
                                        self.expanded = true;
                                        
                                        
                                        
                                    }
                                } else {
                                    if self.spring_hide.value > 0.3 {
                                        self.manually_hidden = true;
                                        self.auto_hidden = false;
                                    } else {
                                        self.manually_hidden = false;
                                        self.auto_hidden = false;
                                    }
                                }
                            }
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        if let Some(surface) = self.surface.as_mut() {
                            let sigmas = if self.config.motion_blur {
                                calculate_blur_sigmas(
                                    self.spring_w.velocity,
                                    self.spring_h.velocity,
                                    self.spring_view.velocity,
                                    self.spring_w.value
                                )
                            } else {
                                (0.0, 0.0)
                            };
                            let total_h = (self.config.expanded_height - self.config.base_height).abs().max(1.0) * self.config.global_scale;
                            let dist_h = (self.spring_h.value - self.config.base_height * self.config.global_scale).abs();
                            let progress = (dist_h / total_h).clamp(0.0, 1.0);
                            let mut media_info = if self.config.smtc_enabled {
                                self.smtc.get_info()
                            } else {
                                crate::core::smtc::MediaInfo::default()
                            };
                            media_info.spectrum = self.audio.get_spectrum();
                            let mut music_active = false;
                            if self.config.smtc_enabled && !media_info.title.is_empty() {
                                if media_info.is_playing {
                                    music_active = true;
                                } else if self.last_playing_time.elapsed() < Duration::from_secs(5) {
                                    music_active = true;
                                }
                            }

                            draw_island(
                                surface,
                                self.spring_w.value,
                                self.spring_h.value,
                                self.spring_r.value,
                                self.os_w,
                                self.os_h,
                                self.border_weights,
                                sigmas,
                                progress,
                                self.spring_view.value,
                                &media_info,
                                music_active,
                                self.config.global_scale,
                                &self.current_lyric_text,
                                &self.old_lyric_text,
                                self.lyric_transition,
                                self.config.motion_blur,
                                self.spring_hide.value,
                                self.lyric_scroll_offset,
                            );
                        }
                    }
                    _ => (),
                }
            }
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            Self::enforce_topmost(window, self.win_x, self.win_y, self.os_w, self.os_h);
            let frame_start = Instant::now();
            if let Some(tray) = &self.tray {
                if let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
                    match TrayAction::from_id(event.id, tray) {
                        Some(TrayAction::ToggleVisibility) => {
                            self.visible = !self.visible;
                            window.set_visible(self.visible);
                            tray.update_item_text(self.visible);
                        }
                        Some(TrayAction::OpenSettings) => {
                            let _ = std::process::Command::new(std::env::current_exe().unwrap())
                                .arg("--settings")
                                .spawn();
                        }
                        Some(TrayAction::Exit) => {
                            event_loop.exit();
                        }
                        None => (),
                    }
                }
            }
            if self.frame_count % 60 == 0 {
                let current_config = load_config();
                if current_config != self.config {
                    let old_scale = self.config.global_scale;
                    let old_max_w = self.config.expanded_width;
                    let old_max_h = self.config.expanded_height;

                    self.config = current_config;
                    self.smtc.set_lyrics_source(self.config.lyrics_source.clone());
                    self.smtc.set_lyrics_fallback(self.config.lyrics_fallback);
                    self.smtc.set_allowed_apps(self.config.smtc_apps.clone());

                    let max_w = self.config.expanded_width.max(450.0);
                    let new_os_w = (max_w * self.config.global_scale + PADDING) as u32;
                    let new_os_h = (self.config.expanded_height * self.config.global_scale + PADDING) as u32;

                    let size_changed = new_os_w != self.os_w || new_os_h != self.os_h ||
                       (old_scale - self.config.global_scale).abs() > 0.001 ||
                       (old_max_w - self.config.expanded_width).abs() > 0.1 ||
                       (old_max_h - self.config.expanded_height).abs() > 0.1;

                    if size_changed {
                        self.os_w = new_os_w;
                        self.os_h = new_os_h;
                        let _ = window.request_inner_size(PhysicalSize::new(self.os_w, self.os_h));
                        if let Some(surface) = self.surface.as_mut() {
                            let _ = surface.resize(
                                std::num::NonZeroU32::new(self.os_w).unwrap(),
                                std::num::NonZeroU32::new(self.os_h).unwrap(),
                            );
                        }
                    }

                    if let Some(monitor) = Self::get_target_monitor(window, self.config.monitor_index) {
                        let mon_size = monitor.size();
                        let mon_pos = monitor.position();
                        self.last_mon_size = (mon_size.width, mon_size.height);
                        self.last_mon_pos = (mon_pos.x, mon_pos.y);
                        let center_x = mon_pos.x + (mon_size.width as i32) / 2;
                        let top_y = mon_pos.y + TOP_OFFSET;
                        self.win_x = center_x - (self.os_w as i32) / 2 + self.config.position_x_offset;
                        self.win_y = top_y - (PADDING / 2.0) as i32 + self.config.position_y_offset;
                        window.set_outer_position(PhysicalPosition::new(self.win_x, self.win_y));
                    }
                } else if let Some(monitor) = Self::get_target_monitor(window, self.config.monitor_index) {
                    let mon_size = monitor.size();
                    let mon_pos = monitor.position();
                    let cur_mon_size = (mon_size.width, mon_size.height);
                    let cur_mon_pos = (mon_pos.x, mon_pos.y);
                    if cur_mon_size != self.last_mon_size || cur_mon_pos != self.last_mon_pos {
                        self.last_mon_size = cur_mon_size;
                        self.last_mon_pos = cur_mon_pos;
                        let center_x = mon_pos.x + (mon_size.width as i32) / 2;
                        let top_y = mon_pos.y + TOP_OFFSET;
                        self.win_x = center_x - (self.os_w as i32) / 2 + self.config.position_x_offset;
                        self.win_y = top_y - (PADDING / 2.0) as i32 + self.config.position_y_offset;
                        window.set_outer_position(PhysicalPosition::new(self.win_x, self.win_y));
                    }
                }
            }

            let dt = (self.last_frame_time.elapsed().as_secs_f32() * 60.0).clamp(0.1, 3.0);
            self.last_frame_time = Instant::now();

            if !self.visible {
                std::thread::sleep(Duration::from_millis(16));
                return;
            }
            let (px, py) = get_global_cursor_pos();
            let rel_x = px - self.win_x;
            let rel_y = py - self.win_y;
            let island_y = PADDING as f64 / 2.0;
            let offset_x = (self.os_w as f64 - self.spring_w.value as f64) / 2.0;
            let hidden_peek_h = (5.0 * self.config.global_scale as f64).max(3.0);
            let hide_distance =
                (self.spring_h.value as f64 - hidden_peek_h + TOP_OFFSET as f64).max(0.0);
            let hide_y_offset = self.spring_hide.value as f64 * hide_distance;
            let current_island_y = island_y - hide_y_offset;
            let is_hovering_visible = is_point_in_rect(
                rel_x as f64,
                rel_y as f64,
                offset_x,
                current_island_y,
                self.spring_w.value as f64,
                self.spring_h.value as f64,
            );
            let hidden_handle_h = (24.0 * self.config.global_scale as f64).max(14.0);
            let hidden_handle_y =
                (current_island_y + self.spring_h.value as f64 - hidden_peek_h - hidden_handle_h * 0.35).max(0.0);
            let is_on_hidden_handle = (self.auto_hidden || self.manually_hidden) && is_point_in_rect(
                rel_x as f64,
                rel_y as f64,
                offset_x,
                hidden_handle_y,
                self.spring_w.value as f64,
                hidden_handle_h,
            );
            let _ = window.set_cursor_hittest(is_hovering_visible || is_on_hidden_handle);

            let mut music_active = false;
            let media = self.smtc.get_info();
            if self.config.smtc_enabled && !media.title.is_empty() {
                self.last_media_playing = media.is_playing;
                if self.last_media_playing {
                    self.last_playing_time = Instant::now();
                    music_active = true;
                } else if self.last_playing_time.elapsed() < Duration::from_secs(5) {
                    music_active = true;
                }
                if media.title != self.last_media_title {
                    self.last_media_title = media.title.clone();
                    window.request_redraw();
                }
            }

            let is_idle = !is_hovering_visible && !self.expanded && !music_active && !self.is_dragging;
            if !self.config.auto_hide {
                self.auto_hidden = false;
                self.idle_timer = Instant::now();
            } else {
                if music_active && self.auto_hidden && !self.manually_hidden {
                    self.auto_hidden = false;
                    self.idle_timer = Instant::now();
                    self.spring_hide.velocity = -0.65;
                } else if self.auto_hidden {
                    if is_on_hidden_handle || is_hovering_visible {
                        self.auto_hidden = false;
                        self.idle_timer = Instant::now();
                        self.spring_hide.velocity = -0.45;
                    } else if !self.expanded && !music_active {
                        // Let idle_timer expire
                    }
                } else if is_idle && !self.manually_hidden {
                    if self.idle_timer.elapsed().as_secs_f32() > self.config.auto_hide_delay {
                        self.auto_hidden = true;
                    }
                } else if !self.manually_hidden && !is_idle {
                    self.idle_timer = Instant::now();
                }
            }

            // Handle dragging on the progress bar while mouse is held
            if self.seeking_progress && is_left_button_pressed() {
                let click_x = rel_x as f32;
                let ratio = ((click_x - self.seeking_bar_left) / (self.seeking_bar_right - self.seeking_bar_left)).clamp(0.0, 1.0);
                let seek_ms = (ratio as f64 * self.seeking_duration_ms as f64) as u64;
                self.smtc.request_seek(seek_ms);
                window.request_redraw();
            } else if self.seeking_progress {
                self.seeking_progress = false;
            }

            if self.is_dragging {
                let diff_y = self.drag_start_py - py;
                if diff_y.abs() > 3 {
                    self.drag_has_moved = true;
                }
                if hide_distance > 0.0 {
                    let mut new_val = self.drag_start_hide_val + (diff_y as f32 / hide_distance as f32);
                    new_val = new_val.clamp(0.0, 1.0);
                    self.spring_hide.value = new_val;
                    self.spring_hide.velocity = 0.0;
                    window.request_redraw();
                }
            } else {
                let hide_target = if self.auto_hidden || self.manually_hidden { 1.0 } else { 0.0 };
                let (stiffness, damping) = if self.auto_hidden || self.manually_hidden { (0.12, 0.70) } else { (0.08, 0.78) };
                self.spring_hide.update_dt(hide_target, stiffness, damping, dt);
            }

            if self.spring_hide.velocity.abs() > 0.001 || (self.spring_hide.value > 0.0 && self.spring_hide.value < 1.0) {
                window.request_redraw();
            }

            if self.expanded && !is_hovering_visible && is_left_button_pressed() {
                self.expanded = false;
                self.widget_view = false;
                window.request_redraw();
            }

            if !self.expanded && is_hovering_visible && is_left_button_pressed() {
                self.idle_timer = Instant::now();
            }

            if self.config.adaptive_border {
                if self.frame_count % 30 == 0 {
                    let island_cx = self.win_x + (self.os_w as i32 / 2);
                    let island_cy =
                        self.win_y + (PADDING as i32 / 2) + (self.spring_h.value as i32 / 2);
                    let raw_weights = get_island_border_weights(
                        island_cx,
                        island_cy,
                        self.spring_w.value,
                        self.spring_h.value,
                    );
                    self.target_border_weights = raw_weights.map(|w| if w > 0.85 { w } else { 0.0 });
                }
            } else {
                self.target_border_weights = [0.0; 4];
            }
            self.frame_count += 1;
            for i in 0..4 {
                let diff = self.target_border_weights[i] - self.border_weights[i];
                if diff.abs() > 0.005 {
                    self.border_weights[i] += diff * 0.1 * dt;
                } else {
                    self.border_weights[i] = self.target_border_weights[i];
                }
            }

            let current_lyric_opt = if self.config.show_lyrics { media.current_lyric((self.config.lyrics_delay * 1000.0) as i64) } else { None };
            if let Some(lyric) = current_lyric_opt {
                if lyric != self.current_lyric_text {
                    self.old_lyric_text = self.current_lyric_text.clone();
                    self.current_lyric_text = lyric.clone();
                    self.lyric_transition = 0.0;
                    self.lyric_scroll_offset = 0.0;
                    self.lyric_scroll_pause = 0.0;
                }
            } else if !self.current_lyric_text.is_empty() {
                self.old_lyric_text = self.current_lyric_text.clone();
                self.current_lyric_text = String::new();
                self.lyric_transition = 0.0;
                self.lyric_scroll_offset = 0.0;
                self.lyric_scroll_pause = 0.0;
            }

            if self.lyric_transition < 1.0 {
                self.lyric_transition += 0.05 * dt;
                if self.lyric_transition > 1.0 {
                    self.lyric_transition = 1.0;
                }
                window.request_redraw();
            }

            let is_currently_hidden = self.auto_hidden || self.manually_hidden || self.spring_hide.value > 0.1;
            let target_base_w = if music_active && !self.expanded && !is_currently_hidden {
                let has_visible_lyrics = self.config.show_lyrics && (!self.current_lyric_text.is_empty() || (!self.old_lyric_text.is_empty() && self.lyric_transition < 1.0));

                if has_visible_lyrics {
                    if self.config.lyrics_scroll {
                        let mut text_w: f32 = 0.0;
                        let display_text = if !self.current_lyric_text.is_empty() {
                            &self.current_lyric_text
                        } else {
                            &self.old_lyric_text
                        };
                        for c in display_text.chars() {
                            if c.is_ascii() {
                                text_w += 7.5;
                            } else {
                                text_w += 13.5;
                            }
                        }
                        let natural_w = 60.0 + text_w;
                        let max_w = self.config.lyrics_scroll_max_width;
                        if natural_w > max_w {
                            let fixed_w = max_w;
                            let available_text_w = (fixed_w - 59.0) * self.config.global_scale;
                            let full_text_w = text_w * self.config.global_scale;
                            let overflow = full_text_w - available_text_w;
                            if overflow > 0.0 && self.lyric_transition >= 1.0 {
                                if self.lyric_scroll_offset < overflow {
                                    if self.lyric_scroll_pause > 0.0 {
                                        self.lyric_scroll_pause -= dt / 60.0;
                                    } else {
                                        self.lyric_scroll_offset += 0.8 * dt;
                                        if self.lyric_scroll_offset >= overflow {
                                            self.lyric_scroll_offset = overflow;
                                        }
                                    }
                                    window.request_redraw();
                                }
                            } else {
                                self.lyric_scroll_offset = 0.0;
                            }
                            fixed_w
                        } else {
                            self.lyric_scroll_offset = 0.0;
                            let min_w = self.config.base_width + 35.0;
                            natural_w.clamp(min_w, max_w)
                        }
                    } else {
                        let mut text_w: f32 = 0.0;
                        let display_text = if !self.current_lyric_text.is_empty() {
                            &self.current_lyric_text
                        } else {
                            &self.old_lyric_text
                        };
                        for c in display_text.chars() {
                            if c.is_ascii() {
                                text_w += 7.5;
                            } else {
                                text_w += 13.5;
                            }
                        }
                        self.lyric_scroll_offset = 0.0;
                        let min_w = self.config.base_width + 35.0;
                        let w: f32 = 60.0 + text_w;
                        w.clamp(min_w, 450.0)
                    }
                } else {
                    self.config.base_width + 35.0
                }
            } else {
                self.lyric_scroll_offset = 0.0;
                self.config.base_width
            };
            let target_w = (if self.expanded { self.config.expanded_width } else { target_base_w }) * self.config.global_scale;
            let target_h = (if self.expanded { self.config.expanded_height } else { self.config.base_height }) * self.config.global_scale;
            let target_r = if self.expanded { 32.0 * self.config.global_scale } else { (self.config.base_height * self.config.global_scale) / 2.0 };
            let target_view = if self.widget_view { 1.0 } else { 0.0 };
            self.spring_w.update_dt(target_w, 0.10, 0.68, dt);
            self.spring_h.update_dt(target_h, 0.10, 0.68, dt);
            self.spring_r.update_dt(target_r, 0.10, 0.68, dt);
            self.spring_view.update_dt(target_view, 0.12, 0.68, dt);

            if self.expanded || music_active || self.spring_w.velocity.abs() > 0.001 || self.spring_h.velocity.abs() > 0.001 || self.spring_r.velocity.abs() > 0.001 || self.spring_view.velocity.abs() > 0.001 {
                window.request_redraw();
            }
            let elapsed = frame_start.elapsed();
            let target_frame_time = Duration::from_micros(6944);
            if elapsed < target_frame_time {
                std::thread::sleep(target_frame_time - elapsed);
            }
        }
    }
}
