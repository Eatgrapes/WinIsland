use std::sync::Arc;
use std::time::Duration;

use softbuffer::{Context, Surface};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_MAXIMIZEBOX, WS_THICKFRAME,
};
use windows::core::PCWSTR;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::platform::windows::WindowAttributesExtWindows;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{Window, WindowButtons, WindowLevel};

use crate::core::config::{PADDING, WINDOW_TITLE};
use crate::utils::icon::get_app_icon;
use crate::window::tray::TrayManager;

use super::App;

impl App {
    pub(super) fn on_resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
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
                .with_resizable(true)
                .with_enabled_buttons(WindowButtons::empty())
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_skip_taskbar(true)
                .with_window_icon(get_app_icon());
            let window = Arc::new(event_loop.create_window(attrs).unwrap());

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
                self.update_animation_frame_interval(&monitor);
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
                if self.config.island_style == "mica" {
                    crate::utils::backdrop::clear_mica_cache();
                }
                if self.config.island_style == "glass" || self.config.island_style == "dynamic" {
                    crate::utils::glass::clear_glass_cache();
                }
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
                            "WinIsland 閸掓繂顫愰崠?GPU 婢惰精瑙﹂敍灞藉讲閼宠姤妲告す鍗炲З閺嗗倹妞傛稉宥呭讲閻劊鈧繐n鐠囬鈼㈤崥搴″晙鐠囨洏鈧繐n\n闁挎瑨顕? {:?}",
                            last_err
                        );
                        let msg_wide: Vec<u16> =
                            msg.encode_utf16().chain(std::iter::once(0)).collect();
                        let title_wide: Vec<u16> = "WinIsland - 閸氼垰濮╅柨娆掝嚖"
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
            if let Ok(handle) = window.window_handle()
                && let RawWindowHandle::Win32(win32_handle) = handle.as_raw()
            {
                let hwnd = HWND(win32_handle.hwnd.get() as _);
                crate::utils::win32::modify_window_ex_style(
                    hwnd,
                    WS_EX_TOOLWINDOW.0 as isize | WS_EX_NOACTIVATE.0 as isize,
                    WS_EX_APPWINDOW.0 as isize,
                );
                crate::utils::win32::modify_window_style(
                    hwnd,
                    0,
                    WS_MAXIMIZEBOX.0 as isize | WS_THICKFRAME.0 as isize,
                );
            }
            window.request_redraw();
        }
    }
}
