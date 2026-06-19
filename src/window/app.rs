use crate::core::audio::AudioProcessor;
use crate::core::config::{AppConfig, PADDING, TOP_OFFSET, WINDOW_TITLE};
use crate::core::context::ContextManager;
use crate::core::persistence::load_config;
use crate::core::render::draw_island;
use crate::core::smtc::SmtcListener;
use crate::plugin::PluginManager;
use crate::plugin::zip_loader;
use crate::plugin::zip_loader::PluginManifest;
use crate::ui::expanded::music_view::{
    get_next_btn_rect, get_pause_btn_rect, get_prev_btn_rect, get_progress_bar_rect,
    set_progress_dragging, set_progress_hover, trigger_cover_flip, trigger_next_click,
    trigger_pause_click, trigger_prev_click,
};
use crate::utils::blur::calculate_blur_sigmas;
use crate::utils::color::get_island_border_weights;
use crate::utils::icon::get_app_icon;
use crate::utils::mouse::{
    get_global_cursor_pos, is_cursor_hidden, is_foreground_fullscreen, is_left_button_pressed,
    is_point_in_rect,
};
use crate::utils::physics::Spring;
use crate::window::tray::{TrayAction, TrayManager};
use softbuffer::{Context, Surface};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
use windows::Win32::UI::WindowsAndMessaging::{
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_MAXIMIZEBOX, WS_THICKFRAME,
};
use windows::core::PCWSTR;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, TouchPhase, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::platform::windows::WindowAttributesExtWindows;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{Window, WindowButtons, WindowId, WindowLevel};

type InstallResult = Result<(PluginManifest, PathBuf, Vec<String>), String>;

pub struct App {
    window: Option<Arc<Window>>,
    context: Option<Context<Arc<Window>>>,
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
    seeking_preview_ms: u64,
    is_fullscreen_suppressed: bool,
    is_cursor_suppressed: bool,
    touch_id: Option<u64>,
    touch_pos: PhysicalPosition<f64>,
    ctx_mgr: ContextManager,
    plugin_mgr: PluginManager,
    plugin_media_source: Option<crate::core::smtc::MediaInfo>,
    pending_install: Option<mpsc::Receiver<InstallResult>>,
}

impl Default for App {
    fn default() -> Self {
        let config = load_config();
        Self {
            window: None,
            context: None,
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
            smtc: SmtcListener::new(
                config.lyrics_source.clone(),
                config.lyrics_fallback,
                config.smtc_apps.clone(),
            ),
            audio: AudioProcessor::new(),
            os_w: 0,
            os_h: 0,
            win_x: 0,
            win_y: 0,
            frame_count: 0,
            last_media_title: String::new(),
            last_media_playing: false,
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
            seeking_preview_ms: 0,
            is_fullscreen_suppressed: false,
            is_cursor_suppressed: false,
            touch_id: None,
            touch_pos: PhysicalPosition::new(0.0, 0.0),
            ctx_mgr: ContextManager::new(),
            plugin_mgr: PluginManager::default(),
            plugin_media_source: None,
            pending_install: None,
        }
    }
}

struct IslandLayout {
    dock_bottom: bool,
    offset_x: f64,
    island_y: f64,
    current_island_y: f64,
    hide_distance: f64,
    hidden_handle_y: f64,
    hidden_handle_h: f64,
}

impl App {
    fn set_aumid() {
        let aumid = "WinIsland.PluginManager";
        let wide: Vec<u16> = aumid.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: SetCurrentProcessExplicitAppUserModelID sets a process-wide string identifier.
        // The wide string is valid and null-terminated. Called once during init before any windows.
        unsafe {
            let _ = SetCurrentProcessExplicitAppUserModelID(PCWSTR::from_raw(wide.as_ptr()));
        }
    }

    fn show_toast(title: &str, message: &str) {
        use windows::UI::Notifications::{
            ToastNotification, ToastNotificationManager, ToastTemplateType,
        };
        use windows::core::HSTRING;
        Self::set_aumid();
        let tmpl =
            match ToastNotificationManager::GetTemplateContent(ToastTemplateType::ToastText02) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("Toast template failed: {:?}", e);
                    return;
                }
            };
        if let Ok(nodes) = tmpl.SelectNodes(&HSTRING::from("//text")) {
            if let Ok(node) = nodes.Item(0) {
                let _ = node.SetInnerText(&HSTRING::from(title));
            }
            if let Ok(node) = nodes.Item(1) {
                let _ = node.SetInnerText(&HSTRING::from(message));
            }
        }
        let toast = match ToastNotification::CreateToastNotification(&tmpl) {
            Ok(t) => t,
            Err(e) => {
                log::error!("CreateToastNotification failed: {:?}", e);
                return;
            }
        };
        let notifier = match ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(
            "WinIsland.PluginManager",
        )) {
            Ok(n) => n,
            Err(e) => {
                log::error!("CreateToastNotifier failed: {:?}", e);
                return;
            }
        };
        if let Err(e) = notifier.Show(&toast) {
            log::error!("Toast Show failed: {:?}", e);
        }
    }

    fn install_zip_drop(&mut self, path: &Path) {
        if self.pending_install.is_some() {
            Self::show_toast("Plugin Info", "Another installation is already in progress");
            return;
        }

        let plugin_dir = self.plugin_mgr.plugin_dir().to_path_buf();
        let zip_path = path.to_path_buf();
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            let result = zip_loader::extract_plugin(&zip_path, &plugin_dir);
            let _ = tx.send(result);
        });

        self.pending_install = Some(rx);
        log::info!("Plugin extraction started in background thread");
    }

    fn get_target_monitor(
        window: &Window,
        monitor_index: i32,
    ) -> Option<winit::monitor::MonitorHandle> {
        if monitor_index < 0 {
            return window
                .primary_monitor()
                .or_else(|| window.current_monitor());
        }
        use windows::Win32::Graphics::Gdi::*;
        let mut win32_names: Vec<String> = Vec::new();
        // SAFETY: EnumDisplayDevicesW reads display device info. We provide a zeroed
        // DISPLAY_DEVICEW with correct cb size. idx increments safely. No mutable global state.
        unsafe {
            let mut idx = 0u32;
            loop {
                let mut dd: DISPLAY_DEVICEW = std::mem::zeroed();
                dd.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
                if EnumDisplayDevicesW(None, idx, &mut dd, 0).as_bool() {
                    if (dd.StateFlags & DISPLAY_DEVICE_ACTIVE) != 0 {
                        let name = String::from_utf16_lossy(&dd.DeviceName)
                            .trim_end_matches('\0')
                            .to_string();
                        win32_names.push(name);
                    }
                    idx += 1;
                } else {
                    break;
                }
            }
        }
        let target_name = win32_names.get(monitor_index as usize);
        let monitors: Vec<_> = window.available_monitors().collect();
        if let Some(name) = target_name {
            for mon in &monitors {
                if let Some(mon_name) = mon.name()
                    && (mon_name.contains(name.trim_start_matches("\\\\.\\"))
                        || name.contains(&mon_name))
                {
                    return Some(mon.clone());
                }
            }
        }
        let idx = monitor_index as usize;
        if idx < monitors.len() {
            monitors.get(idx).cloned()
        } else {
            window
                .primary_monitor()
                .or_else(|| window.current_monitor())
        }
    }

    fn enforce_topmost(window: &Window, win_x: i32, win_y: i32, os_w: u32, os_h: u32) {
        if let Ok(handle) = window.window_handle()
            && let RawWindowHandle::Win32(raw) = handle.as_raw()
        {
            let hwnd = HWND(raw.hwnd.get() as *mut core::ffi::c_void);
            crate::utils::win32::set_window_topmost(hwnd, win_x, win_y, os_w as i32, os_h as i32);
        }
    }

    fn compute_window_position(
        &self,
        mon_pos: PhysicalPosition<i32>,
        mon_size: PhysicalSize<u32>,
    ) -> (i32, i32) {
        let center_x = mon_pos.x + (mon_size.width as i32) / 2;
        let top_y = mon_pos.y + TOP_OFFSET;
        let bottom_y = mon_pos.y + mon_size.height as i32 - TOP_OFFSET;

        let win_x = if self.config.dock_position.is_left() {
            mon_pos.x - (PADDING / 2.0) as i32 + TOP_OFFSET + self.config.position_x_offset
        } else if self.config.dock_position.is_right() {
            mon_pos.x + mon_size.width as i32 - self.os_w as i32 + (PADDING / 2.0) as i32
                - TOP_OFFSET
                + self.config.position_x_offset
        } else {
            center_x - (self.os_w as i32) / 2 + self.config.position_x_offset
        };

        let win_y = if self.config.dock_position.is_bottom() {
            bottom_y - self.os_h as i32 + (PADDING / 2.0) as i32 + self.config.position_y_offset
        } else {
            top_y - (PADDING / 2.0) as i32 + self.config.position_y_offset
        };

        (win_x, win_y)
    }

    fn compute_island_layout(&self) -> IslandLayout {
        let dock_bottom = self.config.dock_position.is_bottom();
        let island_y = if dock_bottom {
            self.os_h as f64 - PADDING as f64 / 2.0 - self.spring_h.value as f64
        } else {
            PADDING as f64 / 2.0
        };

        let offset_x = if self.config.dock_position.is_left() {
            PADDING as f64 / 2.0
        } else if self.config.dock_position.is_right() {
            (self.os_w as f64 - PADDING as f64 / 2.0 - self.spring_w.value as f64).max(0.0)
        } else {
            (self.os_w as f64 - self.spring_w.value as f64) / 2.0
        };

        let scale = self.config.global_scale as f64;
        let hidden_peek_h = (5.0 * scale).max(3.0);
        let hide_distance = if dock_bottom {
            (self.spring_h.value as f64 - hidden_peek_h).max(0.0)
        } else {
            (self.spring_h.value as f64 - hidden_peek_h + TOP_OFFSET as f64).max(0.0)
        };
        let hide_y_offset = self.spring_hide.value as f64 * hide_distance;
        let current_island_y = if dock_bottom {
            island_y + hide_y_offset
        } else {
            island_y - hide_y_offset
        };

        let hidden_handle_h = (24.0 * scale).max(14.0);
        let hidden_handle_y = if dock_bottom {
            (self.os_h as f64 - PADDING as f64 / 2.0 - hidden_handle_h).max(0.0)
        } else {
            (current_island_y + self.spring_h.value as f64 - hidden_peek_h - hidden_handle_h * 0.35)
                .max(0.0)
        };

        IslandLayout {
            dock_bottom,
            offset_x,
            island_y,
            current_island_y,
            hide_distance,
            hidden_handle_y,
            hidden_handle_h,
        }
    }

    fn measure_lyric_text_width(&self, text: &str) -> f32 {
        let mut text_w: f32 = 0.0;
        for c in text.chars() {
            if c.is_ascii() {
                text_w += 7.5;
            } else {
                text_w += 13.5;
            }
        }
        text_w
    }

    fn handle_input(&mut self, state: ElementState, px: i32, py: i32) {
        if self.is_fullscreen_suppressed || self.is_cursor_suppressed {
            return;
        }
        let rel_x = px - self.win_x;
        let rel_y = py - self.win_y;
        let layout = self.compute_island_layout();

        if state == ElementState::Pressed {
            self.handle_press(rel_x, rel_y, &layout);
        } else if state == ElementState::Released {
            self.handle_release(py);
        }
    }

    fn handle_press(&mut self, rel_x: i32, rel_y: i32, layout: &IslandLayout) {
        let island_y = layout.island_y;
        let offset_x = layout.offset_x;
        let current_island_y = layout.current_island_y;

        let is_hovering_visible = is_point_in_rect(
            rel_x as f64,
            rel_y as f64,
            offset_x,
            current_island_y,
            self.spring_w.value as f64,
            self.spring_h.value as f64,
        );

        let hidden_handle_h = layout.hidden_handle_h;
        let hidden_handle_y = layout.hidden_handle_y;
        let is_on_hidden_handle = (self.auto_hidden || self.manually_hidden)
            && is_point_in_rect(
                rel_x as f64,
                rel_y as f64,
                offset_x,
                hidden_handle_y,
                self.spring_w.value as f64,
                hidden_handle_h,
            );

        if self.expanded {
            let view_val = self.spring_view.value as f64;
            let w = self.spring_w.value as f64;
            let h = self.spring_h.value as f64;
            let page_shift = view_val * w;
            let scale = self.config.global_scale as f64;

            if view_val < 0.5 {
                let media = self.smtc.get_info();
                let music_on = self.config.smtc_enabled && !media.title.is_empty();

                let (bx, by, bw, bh) = get_pause_btn_rect(
                    offset_x as f32,
                    island_y as f32,
                    w as f32,
                    h as f32,
                    self.config.global_scale,
                    &self.config.expanded_cover_shape,
                );
                let cx = rel_x as f32 - (page_shift as f32);
                let cy = rel_y as f32;
                if music_on && cx >= bx && cx <= bx + bw && cy >= by && cy <= by + bh {
                    trigger_pause_click(media.is_playing);
                    self.smtc.request_toggle_play();
                    return;
                }

                let (px, py, pw, ph) = get_prev_btn_rect(
                    offset_x as f32,
                    island_y as f32,
                    w as f32,
                    h as f32,
                    self.config.global_scale,
                    &self.config.expanded_cover_shape,
                );
                if music_on && cx >= px && cx <= px + pw && cy >= py && cy <= py + ph {
                    trigger_cover_flip();
                    trigger_prev_click();
                    self.smtc.request_prev();
                    return;
                }

                let (nx, ny, nw, nh) = get_next_btn_rect(
                    offset_x as f32,
                    island_y as f32,
                    w as f32,
                    h as f32,
                    self.config.global_scale,
                    &self.config.expanded_cover_shape,
                );
                if music_on && cx >= nx && cx <= nx + nw && cy >= ny && cy <= ny + nh {
                    trigger_cover_flip();
                    trigger_next_click();
                    self.smtc.request_next();
                    return;
                }

                if let Some((bar_left, bar_right, bar_top, bar_hit_h)) = get_progress_bar_rect(
                    offset_x as f32,
                    island_y as f32,
                    w as f32,
                    &media,
                    music_on,
                    self.config.global_scale,
                    &self.config.expanded_cover_shape,
                ) && cx >= bar_left
                    && cx <= bar_right
                    && cy >= bar_top
                    && cy <= bar_top + bar_hit_h
                {
                    let ratio = ((cx - bar_left) / (bar_right - bar_left)).clamp(0.0, 1.0);
                    let duration_ms = media.effective_duration_ms();
                    let seek_ms = (ratio as f64 * duration_ms as f64) as u64;
                    self.seeking_progress = true;
                    self.seeking_bar_left = bar_left;
                    self.seeking_bar_right = bar_right;
                    self.seeking_duration_ms = duration_ms;
                    self.seeking_preview_ms = seek_ms;
                    return;
                }
            }

            if view_val > 0.5 {
                let gear_x = offset_x + w - 28.0 * scale + w - page_shift;
                let gear_y = island_y + h - 28.0 * scale;
                let dist_sq = (rel_x as f64 - gear_x).powi(2) + (rel_y as f64 - gear_y).powi(2);
                if dist_sq <= (20.0 * scale).powi(2) {
                    if let Ok(exe) = std::env::current_exe() {
                        let _ = std::process::Command::new(exe).arg("--settings").spawn();
                    }
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
        } else if is_hovering_visible || is_on_hidden_handle {
            self.is_dragging = true;
            self.drag_start_py = rel_y + self.win_y;
            self.drag_start_hide_val = self.spring_hide.value;
            self.drag_has_moved = false;
        }
    }

    fn handle_release(&mut self, _py: i32) {
        if self.seeking_progress {
            self.seeking_progress = false;
            if self.seeking_duration_ms > 0 {
                self.smtc.request_seek(self.seeking_preview_ms);
            }
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
            } else if self.spring_hide.value > 0.3 {
                self.manually_hidden = true;
                self.auto_hidden = false;
            } else {
                self.manually_hidden = false;
                self.auto_hidden = false;
            }
        }
    }

    fn close_settings_window() {
        crate::utils::win32::close_window("WinIsland Settings");
    }

    fn handle_tray_events(&mut self, window: &Window, event_loop: &ActiveEventLoop) {
        if let Some(tray) = &self.tray
            && let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv()
        {
            match TrayAction::from_id(event.id, tray) {
                Some(TrayAction::ToggleVisibility) => {
                    self.visible = !self.visible;
                    window.set_visible(self.visible);
                    tray.update_item_text(self.visible);
                    log::info!("Tray: visibility toggled to {}", self.visible);
                }
                Some(TrayAction::OpenSettings) => {
                    log::info!("Tray: opening settings");
                    if let Ok(exe) = std::env::current_exe() {
                        let _ = std::process::Command::new(exe).arg("--settings").spawn();
                    }
                }
                Some(TrayAction::Restart) => {
                    log::info!("Tray: restarting application");
                    Self::close_settings_window();
                    if let Ok(exe) = std::env::current_exe() {
                        let _ = std::process::Command::new(exe).arg("--restart").spawn();
                    }
                    event_loop.exit();
                }
                Some(TrayAction::Exit) => {
                    log::info!("Tray: exiting application");
                    Self::close_settings_window();
                    event_loop.exit();
                }
                None => (),
            }
        }
    }

    fn reload_config_if_changed(&mut self, window: &Window) {
        if !self.frame_count.is_multiple_of(30) {
            return;
        }
        let current_config = load_config();
        if current_config != self.config {
            let old_scale = self.config.global_scale;
            let old_max_w = self.config.expanded_width;
            let old_max_h = self.config.expanded_height;
            let old_style = self.config.island_style.clone();
            let old_mini_shape = self.config.mini_cover_shape.clone();
            let old_expanded_shape = self.config.expanded_cover_shape.clone();
            let old_font = self.config.custom_font_path.clone();

            log::info!("Config changed, reloaded");
            self.config = current_config;
            self.smtc
                .set_lyrics_source(self.config.lyrics_source.clone());
            self.smtc.set_lyrics_fallback(self.config.lyrics_fallback);
            self.smtc
                .set_lyrics_local_dir(self.config.lyrics_local_dir.clone());
            self.smtc.set_allowed_apps(self.config.smtc_apps.clone());

            if old_style != self.config.island_style {
                crate::utils::backdrop::clear_dynamic_bg_cache();
            }

            if old_mini_shape != self.config.mini_cover_shape
                || old_expanded_shape != self.config.expanded_cover_shape
            {
                crate::ui::expanded::music_view::clear_cover_cache();
            }

            if old_font != self.config.custom_font_path {
                crate::utils::font::FontManager::global().refresh_custom_font();
            }

            let max_w = self.config.expanded_width.max(450.0);
            let new_os_w = (max_w * self.config.global_scale + PADDING) as u32;
            let new_os_h =
                (self.config.expanded_height * self.config.global_scale + PADDING) as u32;

            let size_changed = new_os_w != self.os_w
                || new_os_h != self.os_h
                || (old_scale - self.config.global_scale).abs() > 0.001
                || (old_max_w - self.config.expanded_width).abs() > 0.1
                || (old_max_h - self.config.expanded_height).abs() > 0.1;

            if size_changed {
                self.os_w = new_os_w;
                self.os_h = new_os_h;
                let _ = window.request_inner_size(PhysicalSize::new(self.os_w, self.os_h));
                if let Some(surface) = self.surface.as_mut() {
                    let _ = surface.resize(
                        std::num::NonZeroU32::new(self.os_w.max(1)).unwrap(),
                        std::num::NonZeroU32::new(self.os_h.max(1)).unwrap(),
                    );
                }
            }

            if let Some(monitor) = Self::get_target_monitor(window, self.config.monitor_index) {
                let mon_size = monitor.size();
                let mon_pos = monitor.position();
                if mon_size.width > 0 && mon_size.height > 0 {
                    self.last_mon_size = (mon_size.width, mon_size.height);
                    self.last_mon_pos = (mon_pos.x, mon_pos.y);
                    (self.win_x, self.win_y) = self.compute_window_position(mon_pos, mon_size);
                    window.set_outer_position(PhysicalPosition::new(self.win_x, self.win_y));
                }
            }
        } else if let Some(monitor) = Self::get_target_monitor(window, self.config.monitor_index) {
            let mon_size = monitor.size();
            let mon_pos = monitor.position();
            let cur_mon_size = (mon_size.width, mon_size.height);
            let cur_mon_pos = (mon_pos.x, mon_pos.y);
            if (cur_mon_size != self.last_mon_size || cur_mon_pos != self.last_mon_pos)
                && cur_mon_size.0 > 0
                && cur_mon_size.1 > 0
            {
                self.last_mon_size = cur_mon_size;
                self.last_mon_pos = cur_mon_pos;
                (self.win_x, self.win_y) = self.compute_window_position(mon_pos, mon_size);
                window.set_outer_position(PhysicalPosition::new(self.win_x, self.win_y));
            }
        }
    }

    fn compute_lyric_target_width(
        &mut self,
        window: &Window,
        music_active: bool,
        is_paused: bool,
        dt: f32,
    ) -> f32 {
        let is_currently_hidden =
            self.auto_hidden || self.manually_hidden || self.spring_hide.value > 0.1;
        let target_base_w = if music_active && !self.expanded && !is_currently_hidden {
            let has_visible_lyrics = self.config.show_lyrics
                && (!self.current_lyric_text.is_empty()
                    || (!self.old_lyric_text.is_empty() && self.lyric_transition < 1.0));

            if has_visible_lyrics {
                if self.config.lyrics_scroll {
                    let display_text = if !self.current_lyric_text.is_empty() {
                        &self.current_lyric_text
                    } else {
                        &self.old_lyric_text
                    };
                    let text_w = self.measure_lyric_text_width(display_text);
                    let natural_w = 60.0 + text_w;
                    let max_w = self.config.lyrics_scroll_max_width;
                    if natural_w > max_w {
                        let fixed_w = max_w;
                        let available_text_w = (fixed_w - 59.0) * self.config.global_scale;
                        let full_text_w = text_w * self.config.global_scale;
                        let overflow = full_text_w - available_text_w;
                        if overflow > 0.0 && self.lyric_transition >= 1.0 && !is_paused {
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
                        natural_w.clamp(min_w.min(max_w), max_w)
                    }
                } else {
                    let display_text = if !self.current_lyric_text.is_empty() {
                        &self.current_lyric_text
                    } else {
                        &self.old_lyric_text
                    };
                    let text_w = self.measure_lyric_text_width(display_text);
                    self.lyric_scroll_offset = 0.0;
                    let min_w = self.config.base_width + 35.0;
                    let w: f32 = 60.0 + text_w;
                    w.clamp(min_w.min(450.0), 450.0)
                }
            } else {
                self.config.base_width + 35.0
            }
        } else {
            self.lyric_scroll_offset = 0.0;
            self.config.base_width
        };
        (if self.expanded {
            self.config.expanded_width
        } else {
            target_base_w
        }) * self.config.global_scale
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);
        if self.window.is_none() {
            Self::set_aumid();
            self.plugin_mgr.load_all();
            let plugin_count = self.plugin_mgr.list_content_providers().len()
                + self.plugin_mgr.list_theme_providers().len()
                + self.plugin_mgr.list_shortcut_providers().len();
            log::info!("{} plugin(s) loaded", plugin_count);
            let host_api = crate::plugin::manager::init_host_api();
            self.plugin_mgr.init_plugin_host_api(host_api);
            let max_w = self.config.expanded_width.max(450.0);
            self.os_w = (max_w * self.config.global_scale + PADDING) as u32;
            self.os_h = (self.config.expanded_height * self.config.global_scale + PADDING) as u32;
            let attrs = Window::default_attributes()
                .with_title(WINDOW_TITLE)
                .with_inner_size(PhysicalSize::new(self.os_w, self.os_h))
                .with_transparent(true)
                .with_visible(false)
                .with_decorations(false)
                .with_resizable(false)
                .with_enabled_buttons(WindowButtons::empty())
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_skip_taskbar(true)
                .with_window_icon(get_app_icon());
            let window = Arc::new(event_loop.create_window(attrs).unwrap());

            if let Ok(handle) = window.window_handle()
                && let RawWindowHandle::Win32(win32_handle) = handle.as_raw()
            {
                let hwnd = HWND(win32_handle.hwnd.get() as _);
                crate::utils::win32::modify_window_ex_style(
                    hwnd,
                    WS_EX_TOOLWINDOW.0 as isize | WS_EX_NOACTIVATE.0 as isize,
                    0,
                );
                crate::utils::win32::modify_window_style(
                    hwnd,
                    0,
                    WS_MAXIMIZEBOX.0 as isize | WS_THICKFRAME.0 as isize,
                );
            }

            self.window = Some(window.clone());
            log::info!(
                "Window created: {}x{} (base {}x{})",
                self.os_w,
                self.os_h,
                self.config.base_width,
                self.config.base_height
            );

            let mut monitor_opt = None;
            for _ in 0..10 {
                if let Some(monitor) = Self::get_target_monitor(&window, self.config.monitor_index)
                {
                    let size = monitor.size();
                    if size.width > 0 && size.height > 0 {
                        monitor_opt = Some(monitor);
                        break;
                    }
                }
                std::thread::sleep(Duration::from_millis(50));
            }

            if let Some(monitor) = monitor_opt {
                let mon_size = monitor.size();
                let mon_pos = monitor.position();
                self.last_mon_size = (mon_size.width, mon_size.height);
                self.last_mon_pos = (mon_pos.x, mon_pos.y);
                (self.win_x, self.win_y) = self.compute_window_position(mon_pos, mon_size);
                window.set_outer_position(PhysicalPosition::new(self.win_x, self.win_y));
                log::info!(
                    "Monitor: {}x{} @ ({}, {}); window @ ({}, {})",
                    mon_size.width,
                    mon_size.height,
                    mon_pos.x,
                    mon_pos.y,
                    self.win_x,
                    self.win_y
                );
            }
            // Retry GPU context creation up to 3 times with 500ms delay.
            // Handles transient GPU unavailability (e.g., after taskkill from mpv script).
            let (gpu_ctx, gpu_surface) = {
                let mut last_err = None;
                let mut created = None;
                for attempt in 0..3 {
                    if attempt > 0 {
                        std::thread::sleep(Duration::from_millis(500));
                        log::info!("Retrying softbuffer init (attempt {})", attempt + 1);
                    }
                    let ctx = match Context::new(window.clone()) {
                        Ok(c) => c,
                        Err(e) => {
                            last_err = Some(format!("Context::new: {e:?}"));
                            continue;
                        }
                    };
                    let mut surf = match Surface::new(&ctx, window.clone()) {
                        Ok(s) => s,
                        Err(e) => {
                            last_err = Some(format!("Surface::new: {e:?}"));
                            continue;
                        }
                    };
                    let w = std::num::NonZeroU32::new(self.os_w.max(1)).unwrap();
                    let h = std::num::NonZeroU32::new(self.os_h.max(1)).unwrap();
                    match surf.resize(w, h) {
                        Ok(()) => {
                            created = Some((ctx, surf));
                            break;
                        }
                        Err(e) => {
                            last_err = Some(format!("resize: {e:?}"));
                        }
                    }
                }
                match created {
                    Some(pair) => pair,
                    None => {
                        log::error!(
                            "Failed to create softbuffer surface after 3 retries: {:?}",
                            last_err
                        );
                        let msg = format!(
                            "WinIsland 初始化 GPU 失败，可能是驱动暂时不可用。\n请稍后再试。\n\n错误: {:?}",
                            last_err
                        );
                        let msg_wide: Vec<u16> =
                            msg.encode_utf16().chain(std::iter::once(0)).collect();
                        let title_wide: Vec<u16> = "WinIsland - 启动错误"
                            .encode_utf16()
                            .chain(std::iter::once(0))
                            .collect();
                        unsafe {
                            let _ = windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                                None,
                                PCWSTR::from_raw(msg_wide.as_ptr()),
                                PCWSTR::from_raw(title_wide.as_ptr()),
                                windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_STYLE(0),
                            );
                        }
                        event_loop.exit();
                        return;
                    }
                }
            };
            let context = gpu_ctx;
            let mut surface = gpu_surface;
            if let Ok(mut buf) = surface.buffer_mut() {
                for p in buf.iter_mut() {
                    *p = 0;
                }
                let _ = buf.present();
            }
            self.context = Some(context);
            self.surface = Some(surface);
            let is_light = window.theme() == Some(winit::window::Theme::Light);
            self.tray = Some(TrayManager::new(is_light));
            log::info!(
                "Tray icon created (theme={})",
                if is_light { "light" } else { "dark" }
            );
            Self::enforce_topmost(&window, self.win_x, self.win_y, self.os_w, self.os_h);
            window.set_visible(true);
            window.request_redraw();
        }
    }
    fn window_event(&mut self, _event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if let Some(win) = &self.window
            && win.id() == id
        {
            match event {
                WindowEvent::ThemeChanged(theme) => {
                    let is_light = theme == winit::window::Theme::Light;
                    log::info!("Window theme changed to {:?}", theme);
                    if let Some(tray) = self.tray.as_mut() {
                        tray.update_theme(is_light);
                    }
                }
                WindowEvent::Resized(_) if win.is_maximized() => {
                    win.set_maximized(false);
                }
                WindowEvent::CloseRequested => (),
                WindowEvent::DroppedFile(path)
                    if path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("zip")) =>
                {
                    log::info!("File dropped: {}", path.display());
                    self.install_zip_drop(&path);
                }
                WindowEvent::HoveredFile(_) => (),
                WindowEvent::HoveredFileCancelled => (),
                WindowEvent::MouseInput {
                    state,
                    button: MouseButton::Left,
                    ..
                } => {
                    let (px, py) = get_global_cursor_pos();
                    self.handle_input(state, px, py);
                }
                WindowEvent::Touch(touch) => {
                    let (px, py) = (
                        (touch.location.x + self.win_x as f64) as i32,
                        (touch.location.y + self.win_y as f64) as i32,
                    );
                    self.touch_pos = touch.location;
                    match touch.phase {
                        TouchPhase::Started => {
                            self.touch_id = Some(touch.id);
                            self.handle_input(ElementState::Pressed, px, py);
                        }
                        TouchPhase::Moved => {
                            self.touch_id = Some(touch.id);
                        }
                        TouchPhase::Ended | TouchPhase::Cancelled => {
                            self.handle_input(ElementState::Released, px, py);
                            self.touch_id = None;
                        }
                    }
                }
                WindowEvent::RedrawRequested => {
                    if let Some(surface) = self.surface.as_mut() {
                        let dt =
                            (self.last_frame_time.elapsed().as_secs_f32() * 60.0).clamp(0.1, 3.0);
                        let sigmas = if self.config.motion_blur {
                            calculate_blur_sigmas(
                                self.spring_w.velocity,
                                self.spring_h.velocity,
                                self.spring_view.velocity,
                                self.spring_w.value,
                            )
                        } else {
                            (0.0, 0.0)
                        };
                        let total_h = (self.config.expanded_height - self.config.base_height)
                            .abs()
                            .max(1.0)
                            * self.config.global_scale;
                        let dist_h = (self.spring_h.value
                            - self.config.base_height * self.config.global_scale)
                            .abs();
                        let progress = (dist_h / total_h).clamp(0.0, 1.0);
                        // Drain pending plugin media source (called once per frame)
                        if let Some(ps) = crate::plugin::manager::drain_pending_media_source() {
                            use std::sync::Arc;
                            let (cover, hash) = if !ps.cover_data.is_empty() {
                                use std::collections::hash_map::DefaultHasher;
                                use std::hash::{Hash, Hasher};
                                let mut hasher = DefaultHasher::new();
                                ps.cover_data.hash(&mut hasher);
                                (Some(Arc::new(ps.cover_data)), hasher.finish())
                            } else {
                                (None, 0)
                            };
                            self.plugin_media_source = Some(crate::core::smtc::MediaInfo {
                                title: ps.title,
                                artist: ps.artist,
                                album: ps.album,
                                duration_ms: ps.duration_ms,
                                duration_secs: ps.duration_ms / 1000,
                                position_ms: ps.position_ms,
                                is_playing: ps.is_playing,
                                last_update: Instant::now(),
                                thumbnail: cover,
                                thumbnail_hash: hash,
                                ..Default::default()
                            });
                        }
                        // Advance plugin media source position
                        if let Some(ref mut info) = self.plugin_media_source
                            && info.is_playing
                        {
                            let elapsed = info.last_update.elapsed().as_millis() as u64;
                            info.position_ms = info.position_ms.saturating_add(elapsed);
                            info.last_update = Instant::now();
                        }
                        let mut media_info = self
                            .plugin_media_source
                            .clone()
                            .or_else(|| {
                                if self.config.smtc_enabled {
                                    Some(self.smtc.get_info())
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_default();
                        if self.seeking_progress && self.seeking_duration_ms > 0 {
                            media_info.position_ms = self.seeking_preview_ms;
                            media_info.last_update = Instant::now();
                        }
                        let is_hidden = self.auto_hidden || self.manually_hidden;
                        self.audio.set_gate_override(!is_hidden);
                        media_info.spectrum = self.audio.get_spectrum();
                        let mut music_active = false;
                        if self.config.smtc_enabled && !media_info.title.is_empty() {
                            music_active = true;
                        }
                        self.ctx_mgr.set_smtc_active(music_active);
                        crate::plugin::manager::drain_pending_contexts(&mut self.ctx_mgr);
                        self.ctx_mgr.tick();
                        let mini_content = self.ctx_mgr.current_mini();

                        let widget_animating = draw_island(
                            surface,
                            crate::core::render::DrawIslandParams {
                                layout: crate::core::render::LayoutParams {
                                    current_w: self.spring_w.value,
                                    current_h: self.spring_h.value,
                                    current_r: self.spring_r.value,
                                    os_w: self.os_w,
                                    os_h: self.os_h,
                                    sigmas,
                                    expansion_progress: progress,
                                    view_offset: self.spring_view.value,
                                    global_scale: self.config.global_scale,
                                    hide_progress: self.spring_hide.value,
                                    dock_position: self.config.dock_position,
                                },
                                media: crate::core::render::MediaParams {
                                    media: &media_info,
                                    music_active,
                                },
                                lyrics: crate::core::render::LyricsParams {
                                    current_lyric: &self.current_lyric_text,
                                    old_lyric: &self.old_lyric_text,
                                    lyric_transition: self.lyric_transition,
                                    lyric_scroll_offset: self.lyric_scroll_offset,
                                },
                                window: crate::core::render::WindowParams {
                                    win_x: self.win_x,
                                    win_y: self.win_y,
                                    monitor_x: self.last_mon_pos.0,
                                    monitor_y: self.last_mon_pos.1,
                                    monitor_w: self.last_mon_size.0,
                                    monitor_h: self.last_mon_size.1,
                                },
                                style: crate::core::render::StyleParams {
                                    island_style: &self.config.island_style,
                                    use_blur: self.config.motion_blur,
                                    font_size: self.config.font_size,
                                    weights: self.border_weights,
                                    mini_cover_shape: &self.config.mini_cover_shape,
                                    expanded_cover_shape: &self.config.expanded_cover_shape,
                                    cover_rotate: self.config.cover_rotate,
                                    lyrics_delay: self.config.lyrics_delay,
                                    dt,
                                },
                                mini_content,
                            },
                        );
                        if widget_animating && let Some(win) = &self.window {
                            win.request_redraw();
                        }
                    }
                }
                _ => (),
            }
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let window = match self.window.clone() {
            Some(w) => w,
            None => return,
        };
        Self::enforce_topmost(&window, self.win_x, self.win_y, self.os_w, self.os_h);
        let frame_start = Instant::now();
        self.handle_tray_events(&window, event_loop);
        self.reload_config_if_changed(&window);

        if let Some(rx) = self.pending_install.take() {
            match rx.try_recv() {
                Ok(Ok((manifest, _dest, dll_paths))) => {
                    for dll in &dll_paths {
                        self.plugin_mgr.load_dll(Path::new(dll));
                    }
                    Self::show_toast(
                        "Plugin Installed",
                        &format!("{} loaded successfully!", manifest.name),
                    );
                    log::info!("Plugin '{}' installed via drop", manifest.name);
                }
                Ok(Err(e)) => {
                    Self::show_toast("Plugin Error", &e);
                    log::error!("Failed to install plugin from drop: {}", e);
                }
                Err(mpsc::TryRecvError::Empty) => {
                    self.pending_install = Some(rx);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    Self::show_toast("Plugin Error", "Installation thread crashed");
                    log::error!("Plugin installation thread disconnected unexpectedly");
                }
            }
        }

        let dt = (self.last_frame_time.elapsed().as_secs_f32() * 60.0).clamp(0.1, 3.0);
        self.last_frame_time = Instant::now();

        if !self.visible {
            std::thread::sleep(Duration::from_millis(16));
            return;
        }
        let (px, py) = if self.touch_id.is_some() {
            (
                (self.touch_pos.x + self.win_x as f64) as i32,
                (self.touch_pos.y + self.win_y as f64) as i32,
            )
        } else {
            get_global_cursor_pos()
        };
        let rel_x = px - self.win_x;
        let rel_y = py - self.win_y;
        let layout = self.compute_island_layout();
        let dock_bottom = layout.dock_bottom;
        let island_y = layout.island_y;
        let offset_x = layout.offset_x;
        let hide_distance = layout.hide_distance;
        let current_island_y = layout.current_island_y;
        let is_hovering_visible = is_point_in_rect(
            rel_x as f64,
            rel_y as f64,
            offset_x,
            current_island_y,
            self.spring_w.value as f64,
            self.spring_h.value as f64,
        );
        let hidden_handle_h = layout.hidden_handle_h;
        let hidden_handle_y = layout.hidden_handle_y;
        let is_on_hidden_handle = (self.auto_hidden || self.manually_hidden)
            && is_point_in_rect(
                rel_x as f64,
                rel_y as f64,
                offset_x,
                hidden_handle_y,
                self.spring_w.value as f64,
                hidden_handle_h,
            );

        if self.frame_count.is_multiple_of(10) {
            let prev_fullscreen = self.is_fullscreen_suppressed;
            self.is_fullscreen_suppressed = is_foreground_fullscreen();
            self.is_cursor_suppressed = is_cursor_hidden();
            if self.is_fullscreen_suppressed != prev_fullscreen {
                log::info!(
                    "Fullscreen state: {}",
                    if self.is_fullscreen_suppressed {
                        "suppressed"
                    } else {
                        "normal"
                    }
                );
            }
        }

        if self.is_fullscreen_suppressed || self.is_cursor_suppressed {
            let _ = window.set_cursor_hittest(false);
        } else {
            let _ = window.set_cursor_hittest(is_hovering_visible || is_on_hidden_handle);
        }

        let mut music_active = false;
        let media = self.smtc.get_info();
        if self.config.smtc_enabled && !media.title.is_empty() {
            self.last_media_playing = media.is_playing;
            music_active = true;
            if media.title != self.last_media_title {
                log::info!(
                    "Track changed: {} - {} / {}",
                    media.title,
                    media.artist,
                    media.album
                );
                self.last_media_title = media.title.clone();
                crate::ui::expanded::music_view::trigger_cover_flip();
                crate::utils::backdrop::clear_dynamic_bg_cache();
                window.request_redraw();
            }
        }

        let is_paused_idle = music_active && !media.is_playing;
        let is_idle = !is_hovering_visible
            && !self.expanded
            && !self.is_dragging
            && (!music_active || is_paused_idle);
        if !self.config.auto_hide {
            self.auto_hidden = false;
            self.idle_timer = Instant::now();
        } else if media.is_playing && self.auto_hidden && !self.manually_hidden {
            self.auto_hidden = false;
            self.idle_timer = Instant::now();
            self.spring_hide.velocity = -0.65;
            log::info!("Island un-hidden (media playing)");
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
                log::info!(
                    "Island auto-hidden (idle {:.1}s)",
                    self.config.auto_hide_delay
                );
            }
        } else if !self.manually_hidden && !is_idle {
            self.idle_timer = Instant::now();
        }

        if self.seeking_progress && (is_left_button_pressed() || self.touch_id.is_some()) {
            let page_shift = self.spring_view.value * self.spring_w.value;
            let click_x = rel_x as f32 - page_shift;
            let bar_width = self.seeking_bar_right - self.seeking_bar_left;
            let ratio = if bar_width > 0.0 {
                ((click_x - self.seeking_bar_left) / bar_width).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let seek_ms = (ratio as f64 * self.seeking_duration_ms as f64) as u64;
            self.seeking_preview_ms = seek_ms;
            window.request_redraw();
        } else if self.seeking_progress {
            self.seeking_progress = false;
            if self.seeking_duration_ms > 0 {
                self.smtc.request_seek(self.seeking_preview_ms);
                window.request_redraw();
            }
        }

        let progress_hover_active = if self.seeking_progress {
            true
        } else if self.expanded && (self.spring_view.value as f64) < 0.5 {
            if let Some((bar_left, bar_right, bar_top, bar_hit_h)) = get_progress_bar_rect(
                offset_x as f32,
                island_y as f32,
                self.spring_w.value,
                &media,
                music_active,
                self.config.global_scale,
                &self.config.expanded_cover_shape,
            ) {
                let page_shift = self.spring_view.value * self.spring_w.value;
                let cx = rel_x as f32 - page_shift;
                let cy = rel_y as f32;
                let margin = 4.0 * self.config.global_scale;
                cx >= bar_left - margin
                    && cx <= bar_right + margin
                    && cy >= bar_top - margin
                    && cy <= bar_top + bar_hit_h + margin
            } else {
                false
            }
        } else {
            false
        };
        set_progress_hover(progress_hover_active);
        set_progress_dragging(self.seeking_progress);

        if self.is_dragging {
            let diff_y = if dock_bottom {
                py - self.drag_start_py
            } else {
                self.drag_start_py - py
            };
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
            let hide_target = if self.auto_hidden || self.manually_hidden {
                1.0
            } else {
                0.0
            };
            let (stiffness, damping) = if self.auto_hidden || self.manually_hidden {
                (0.12, 0.70)
            } else {
                (0.08, 0.78)
            };
            self.spring_hide
                .update_dt(hide_target, stiffness, damping, dt);
        }

        if self.spring_hide.velocity.abs() > 0.001
            || (self.spring_hide.value > 0.0 && self.spring_hide.value < 1.0)
        {
            window.request_redraw();
        }

        if self.expanded
            && !is_hovering_visible
            && (is_left_button_pressed() || self.touch_id.is_some())
        {
            self.expanded = false;
            self.widget_view = false;
            window.request_redraw();
        }

        if !self.expanded
            && is_hovering_visible
            && (is_left_button_pressed() || self.touch_id.is_some())
        {
            self.idle_timer = Instant::now();
        }

        if self.config.adaptive_border {
            if self.frame_count.is_multiple_of(30) {
                let island_cx =
                    self.win_x + (offset_x + (self.spring_w.value as f64) / 2.0).round() as i32;
                let island_cy = self.win_y
                    + (current_island_y + (self.spring_h.value as f64) / 2.0).round() as i32;
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

        let is_paused = music_active && !media.is_playing;
        let current_lyric_opt = if self.config.show_lyrics && !is_paused {
            media.current_lyric((self.config.lyrics_delay * 1000.0) as i64)
        } else {
            None
        };
        if let Some(lyric) = current_lyric_opt {
            if lyric != self.current_lyric_text {
                self.old_lyric_text = self.current_lyric_text.clone();
                self.current_lyric_text = lyric.clone();
                self.lyric_transition = 0.0;
                self.lyric_scroll_offset = 0.0;
                self.lyric_scroll_pause = 0.0;
            }
        } else if !is_paused && !self.current_lyric_text.is_empty() {
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

        let target_w = self.compute_lyric_target_width(&window, music_active, is_paused, dt);
        let target_h = (if self.expanded {
            self.config.expanded_height
        } else {
            self.config.base_height
        }) * self.config.global_scale;
        let target_r = if self.expanded {
            32.0 * self.config.global_scale
        } else {
            (self.config.base_height * self.config.global_scale) / 2.0
        };
        let target_view = if self.widget_view { 1.0 } else { 0.0 };
        self.spring_w.update_dt(target_w, 0.10, 0.68, dt);
        self.spring_h.update_dt(target_h, 0.10, 0.68, dt);
        self.spring_r.update_dt(target_r, 0.10, 0.68, dt);
        self.spring_view.update_dt(target_view, 0.12, 0.68, dt);

        if self.expanded
            || (music_active && media.is_playing)
            || self.spring_w.velocity.abs() > 0.001
            || self.spring_h.velocity.abs() > 0.001
            || self.spring_r.velocity.abs() > 0.001
            || self.spring_view.velocity.abs() > 0.001
        {
            window.request_redraw();
        }
        let elapsed = frame_start.elapsed();
        let target_frame_time = Duration::from_micros(6944);
        if elapsed < target_frame_time {
            std::thread::sleep(target_frame_time - elapsed);
        }
    }
}
