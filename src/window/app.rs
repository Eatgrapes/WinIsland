use std::sync::Arc;
use std::time::{Duration, Instant};
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::platform::windows::WindowAttributesExtWindows;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{Window, WindowId, WindowLevel};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE};
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

pub struct App {
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    tray: Option<TrayManager>,
    smtc: SmtcListener,
    audio: AudioProcessor,
    config: AppConfig,
    expanded: bool,
    tools_view: bool,
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
    tool_hovers: [f32; 15],
    tool_presses: [f32; 15],
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
            tools_view: false,
            visible: true,
            border_weights: [0.0; 4],
            target_border_weights: [0.0; 4],
            spring_w: Spring::new(config.base_width * config.global_scale),
            spring_h: Spring::new(config.base_height * config.global_scale),
            spring_r: Spring::new((config.base_height * config.global_scale) / 2.0),
            spring_view: Spring::new(0.0),
            smtc: SmtcListener::new(config.lyrics_source.clone(), config.lyrics_fallback),
            audio: AudioProcessor::new(),
            os_w: 0,
            os_h: 0,
            win_x: 0,
            win_y: 0,
            frame_count: 0,
            tool_hovers: [0.0; 15],
            tool_presses: [0.0; 15],
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
        }
    }
}

impl App {
    fn enforce_topmost(window: &Window) {
        if let Ok(handle) = window.window_handle() {
            if let RawWindowHandle::Win32(raw) = handle.as_raw() {
                let hwnd = HWND(raw.hwnd.get() as *mut core::ffi::c_void);
                unsafe {
                    let _ = SetWindowPos(
                        hwnd,
                        HWND_TOPMOST,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
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
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_skip_taskbar(true)
                .with_window_icon(get_app_icon());
            let window = Arc::new(event_loop.create_window(attrs).unwrap());
            self.window = Some(window.clone());
            if let Some(monitor) = window.current_monitor() {
                let mon_size = monitor.size();
                let mon_pos = monitor.position();
                let center_x = mon_pos.x + (mon_size.width as i32) / 2;
                let top_y = mon_pos.y + TOP_OFFSET;
                self.win_x = center_x - (self.os_w as i32) / 2;
                self.win_y = top_y - (PADDING / 2.0) as i32;
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
            Self::enforce_topmost(&window);
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
                                let center_y = island_y + self.spring_h.value as f64 / 2.0;
                                if !self.tools_view {
                                    let btn_x = offset_x + self.spring_w.value as f64 - 20.0 * scale;
                                    let dist_sq = (rel_x as f64 - btn_x).powi(2) + (rel_y as f64 - center_y).powi(2);
                                    if dist_sq <= (25.0 * scale).powi(2) {
                                        self.tools_view = true;
                                        self.spring_view.velocity *= 0.2;
                                        return;
                                    }
                                } else {
                                    let btn_x = offset_x + 20.0 * scale;
                                    let dist_sq = (rel_x as f64 - btn_x).powi(2) + (rel_y as f64 - center_y).powi(2);
                                    if dist_sq <= (25.0 * scale).powi(2) {
                                        self.tools_view = false;
                                        self.spring_view.velocity *= 0.2;
                                        return;
                                    }
                                    let grid_w = self.spring_w.value as f64 - 80.0 * scale;
                                    let grid_h = self.spring_h.value as f64 - 40.0 * scale;
                                    let x_step = grid_w / 5.0;
                                    let y_step = grid_h / 3.0;
                                    let start_x = offset_x + 40.0 * scale + x_step / 2.0;
                                    let start_y = island_y + 20.0 * scale + y_step / 2.0;
                                    let settings_cx = start_x + (0.0 * x_step);
                                    let settings_cy = start_y + (0.0 * y_step);
                                    let dist_sq_s = (rel_x as f64 - settings_cx).powi(2) + (rel_y as f64 - settings_cy).powi(2);
                                    if dist_sq_s <= (28.0 * scale).powi(2) {
                                        self.tool_presses[0] = 1.0;
                                        let _ = std::process::Command::new(std::env::current_exe().unwrap())
                                            .arg("--settings")
                                            .spawn();
                                        return;
                                    }
                                    let music_cx = start_x + (1.0 * x_step);
                                    let music_cy = start_y + (0.0 * y_step);
                                    let dist_sq_m = (rel_x as f64 - music_cx).powi(2) + (rel_y as f64 - music_cy).powi(2);
                                    if dist_sq_m <= (28.0 * scale).powi(2) {
                                        self.tool_presses[1] = 1.0;
                                        let _ = std::process::Command::new(std::env::current_exe().unwrap())
                                            .arg("--music-settings")
                                            .spawn();
                                        return;
                                    }
                                }
                                if (rel_y as f64) < island_y + 40.0 * scale {
                                    self.expanded = false;
                                    self.tools_view = false;
                                    self.spring_w.velocity *= 0.2;
                                    self.spring_h.velocity *= 0.2;
                                    self.spring_r.velocity *= 0.2;
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
                                        self.spring_w.velocity *= 0.2;
                                        self.spring_h.velocity *= 0.2;
                                        self.spring_r.velocity *= 0.2;
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
                                &self.tool_hovers,
                                &self.tool_presses,
                                &self.current_lyric_text,
                                &self.old_lyric_text,
                                self.lyric_transition,
                                self.config.motion_blur,
                                self.spring_hide.value,
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
            Self::enforce_topmost(window);
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
                        Some(TrayAction::OpenMusicSettings) => {
                            let _ = std::process::Command::new(std::env::current_exe().unwrap())
                                .arg("--music-settings")
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
                    
                    let max_w = self.config.expanded_width.max(450.0);
                    let new_os_w = (max_w * self.config.global_scale + PADDING) as u32;
                    let new_os_h = (self.config.expanded_height * self.config.global_scale + PADDING) as u32;
                    
                    if new_os_w != self.os_w || new_os_h != self.os_h || 
                       (old_scale - self.config.global_scale).abs() > 0.001 ||
                       (old_max_w - self.config.expanded_width).abs() > 0.1 ||
                       (old_max_h - self.config.expanded_height).abs() > 0.1 {
                        
                        self.os_w = new_os_w;
                        self.os_h = new_os_h;
                        let _ = window.request_inner_size(PhysicalSize::new(self.os_w, self.os_h));
                        if let Some(surface) = self.surface.as_mut() {
                            let _ = surface.resize(
                                std::num::NonZeroU32::new(self.os_w).unwrap(),
                                std::num::NonZeroU32::new(self.os_h).unwrap(),
                            );
                        }
                        if let Some(monitor) = window.current_monitor() {
                            let mon_size = monitor.size();
                            let mon_pos = monitor.position();
                            let center_x = mon_pos.x + (mon_size.width as i32) / 2;
                            self.win_x = center_x - (self.os_w as i32) / 2;
                            window.set_outer_position(PhysicalPosition::new(self.win_x, self.win_y));
                        }
                    }
                }
            }
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
                self.spring_hide.update(hide_target, stiffness, damping);
            }

            if self.spring_hide.velocity.abs() > 0.001 || (self.spring_hide.value > 0.0 && self.spring_hide.value < 1.0) {
                window.request_redraw();
            }

            if self.expanded && !is_hovering_visible && is_left_button_pressed() {
                self.expanded = false;
                self.tools_view = false;
                self.spring_w.velocity *= 0.2;
                self.spring_h.velocity *= 0.2;
                self.spring_r.velocity *= 0.2;
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
                    self.border_weights[i] += diff * 0.1;
                } else {
                    self.border_weights[i] = self.target_border_weights[i];
                }
            }
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

            let current_lyric_opt = if self.config.show_lyrics { media.current_lyric() } else { None };
            if let Some(lyric) = current_lyric_opt {
                if lyric != self.current_lyric_text {
                    self.old_lyric_text = self.current_lyric_text.clone();
                    self.current_lyric_text = lyric.clone();
                    self.lyric_transition = 0.0;
                }
            } else if !self.current_lyric_text.is_empty() {
                self.old_lyric_text = self.current_lyric_text.clone();
                self.current_lyric_text = String::new();
                self.lyric_transition = 0.0;
            }

            if self.lyric_transition < 1.0 {
                self.lyric_transition += 0.05;
                if self.lyric_transition > 1.0 {
                    self.lyric_transition = 1.0;
                }
                window.request_redraw();
            }

            let is_currently_hidden = self.auto_hidden || self.manually_hidden || self.spring_hide.value > 0.1;
            let target_base_w = if music_active && !self.expanded && !is_currently_hidden {
                let has_visible_lyrics = self.config.show_lyrics && (!self.current_lyric_text.is_empty() || (!self.old_lyric_text.is_empty() && self.lyric_transition < 1.0));
                
                if has_visible_lyrics {
                    let mut text_w = 0.0;
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
                    let min_w = self.config.base_width + 35.0;
                    let w: f32 = 60.0 + text_w;
                    w.clamp(min_w, 450.0)
                } else {
                    self.config.base_width + 35.0
                }
            } else {
                self.config.base_width
            };
            let target_w = (if self.expanded { self.config.expanded_width } else { target_base_w }) * self.config.global_scale;
            let target_h = (if self.expanded { self.config.expanded_height } else { self.config.base_height }) * self.config.global_scale;
            let target_r = if self.expanded { 32.0 * self.config.global_scale } else { (self.config.base_height * self.config.global_scale) / 2.0 };
            let target_view = if self.tools_view { 1.0 } else { 0.0 };
            self.spring_w.update(target_w, 0.10, 0.68);
            self.spring_h.update(target_h, 0.10, 0.68);
            self.spring_r.update(target_r, 0.10, 0.68);
            self.spring_view.update(target_view, 0.12, 0.68);

            if self.expanded && self.tools_view {
                let grid_w = self.spring_w.value - 80.0 * self.config.global_scale;
                let grid_h = self.spring_h.value - 40.0 * self.config.global_scale;
                let x_step = grid_w / 5.0;
                let y_step = grid_h / 3.0;
                let start_x = offset_x as f32 + 40.0 * self.config.global_scale + x_step / 2.0;
                let start_y = island_y as f32 + 20.0 * self.config.global_scale + y_step / 2.0;
                let bubble_r = 18.0 * self.config.global_scale;

                for r in 0..3 {
                    for c in 0..5 {
                        let idx = r * 5 + c;
                        let cx = start_x + (c as f32 * x_step);
                        let cy = start_y + (r as f32 * y_step);
                        let dx = rel_x as f32 - cx;
                        let dy = rel_y as f32 - cy;
                        let dist_sq = dx * dx + dy * dy;
                        let is_hover = dist_sq < (bubble_r * 1.2).powi(2);
                        
                        let target = if is_hover { 1.0 } else { 0.0 };
                        let diff = target - self.tool_hovers[idx];
                        if diff.abs() > 0.001 {
                            self.tool_hovers[idx] += diff * 0.15;
                            window.request_redraw();
                        } else {
                            self.tool_hovers[idx] = target;
                        }

                        if self.tool_presses[idx] > 0.0 {
                            self.tool_presses[idx] -= 0.1;
                            if self.tool_presses[idx] < 0.0 { self.tool_presses[idx] = 0.0; }
                            window.request_redraw();
                        }
                    }
                }
            }

            if self.expanded || music_active || self.spring_w.velocity.abs() > 0.01 || self.spring_h.velocity.abs() > 0.01 {
                window.request_redraw();
            }
            let elapsed = frame_start.elapsed();
            let target_frame_time = Duration::from_micros(16666);
            if elapsed < target_frame_time {
                std::thread::sleep(target_frame_time - elapsed);
            }
        }
    }
}
