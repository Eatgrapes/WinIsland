use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use windows::ApplicationModel::Package;
use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
use windows::core::PCWSTR;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use crate::core::config::PADDING;
use crate::core::persistence::{get_config_path, load_config};
use crate::plugin::zip_loader;
use crate::window::tray::TrayAction;

use super::App;

impl App {
    pub(super) fn set_aumid() {
        if Package::Current().is_ok() {
            return;
        }
        let aumid = "WinIsland.PluginManager";
        let wide: Vec<u16> = aumid.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: SetCurrentProcessExplicitAppUserModelID sets a process-wide string identifier.
        // The wide string is valid and null-terminated. Called once during init before any windows.
        unsafe {
            let _ = SetCurrentProcessExplicitAppUserModelID(PCWSTR::from_raw(wide.as_ptr()));
        }
    }

    pub(super) fn show_toast(title: &str, message: &str) {
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
        let notifier_result = if Package::Current().is_ok() {
            ToastNotificationManager::CreateToastNotifier()
        } else {
            ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(
                "WinIsland.PluginManager",
            ))
        };
        let notifier = match notifier_result {
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

    pub(super) fn install_zip_drop(&mut self, path: &Path) {
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

    pub(super) fn open_settings(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(settings) = &self.settings {
            settings.bring_to_front();
            return;
        }

        let mut settings = crate::window::settings::SettingsApp::new(load_config());
        settings.create_window(event_loop);
        self.settings = Some(settings);
        log::info!("Settings window opened in main process");
    }

    pub(super) fn close_settings(&mut self) {
        if let Some(mut settings) = self.settings.take() {
            settings.close();
            log::info!("Settings window closed and resources released");
        }
    }

    pub(super) fn handle_tray_events(&mut self, window: &Window, event_loop: &ActiveEventLoop) {
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
                    self.open_settings(event_loop);
                }
                Some(TrayAction::Restart) => {
                    log::info!("Tray: restarting application");
                    self.close_settings();
                    if let Ok(exe) = std::env::current_exe() {
                        let _ = std::process::Command::new(exe).arg("--restart").spawn();
                    }
                    event_loop.exit();
                }
                Some(TrayAction::Exit) => {
                    log::info!("Tray: exiting application");
                    self.close_settings();
                    event_loop.exit();
                }
                None => (),
            }
        }
    }

    pub(super) fn reload_config_if_changed(&mut self, window: &Window) {
        let now = Instant::now();
        if now.duration_since(self.last_config_check) >= Duration::from_millis(500) {
            self.last_config_check = now;
            let modified = std::fs::metadata(get_config_path())
                .and_then(|metadata| metadata.modified())
                .ok();
            if modified != self.last_config_modified {
                self.last_config_modified = modified;
                let current_config = load_config();
                if current_config != self.config {
                    let old_scale = self.config.global_scale;
                    let old_max_w = self.config.expanded_width;
                    let old_max_h = self.config.expanded_height;
                    let old_style = self.config.island_style.clone();
                    let old_mini_shape = self.config.mini_cover_shape.clone();
                    let old_expanded_shape = self.config.expanded_cover_shape.clone();
                    let old_font = self.config.custom_font_path.clone();
                    let old_smtc_enabled = self.config.smtc_enabled;
                    let old_position_x_offset = self.config.position_x_offset;
                    let old_position_y_offset = self.config.position_y_offset;
                    let old_dock_position = self.config.dock_position;
                    let old_monitor_index = self.config.monitor_index;

                    log::info!("Config changed, reloaded");
                    self.config = current_config;
                    self.smtc
                        .set_lyrics_source(self.config.lyrics_source.clone());
                    self.smtc.set_lyrics_fallback(self.config.lyrics_fallback);
                    self.smtc
                        .set_lyrics_local_dir(self.config.lyrics_local_dir.clone());
                    self.smtc.set_allowed_apps(self.config.smtc_apps.clone());
                    if old_smtc_enabled != self.config.smtc_enabled {
                        self.audio.set_target_app_id(
                            if self.config.smtc_enabled && !self.smtc_media_info.title.is_empty() {
                                &self.smtc_media_info.source_app_id
                            } else {
                                ""
                            },
                        );
                    }

                    if old_style != self.config.island_style {
                        crate::utils::backdrop::clear_mica_cache();
                        crate::utils::glass::clear_glass_cache();
                        crate::utils::backdrop::clear_blurred_cover_cache();
                        if let Ok(handle) = window.window_handle() {
                            let raw = handle.as_raw();
                            if let RawWindowHandle::Win32(win32_handle) = raw {
                                let hwnd =
                                    windows::Win32::Foundation::HWND(win32_handle.hwnd.get() as _);
                                if old_style == "mica" {
                                    crate::utils::backdrop::disable_mica(hwnd);
                                }
                            }
                        }
                    }

                    if old_mini_shape != self.config.mini_cover_shape
                        || old_expanded_shape != self.config.expanded_cover_shape
                    {
                        crate::ui::expanded::music_view::clear_cover_cache();
                    }

                    if old_font != self.config.custom_font_path {
                        crate::utils::font::FontManager::global()
                            .set_custom_font_path(self.config.custom_font_path.as_deref());
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
                    let position_changed = old_position_x_offset != self.config.position_x_offset
                        || old_position_y_offset != self.config.position_y_offset
                        || old_dock_position != self.config.dock_position
                        || old_monitor_index != self.config.monitor_index;

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

                    if (size_changed || position_changed)
                        && let Some(monitor) =
                            Self::get_target_monitor(window, self.config.monitor_index)
                    {
                        let mon_size = monitor.size();
                        let mon_pos = monitor.position();
                        if mon_size.width > 0 && mon_size.height > 0 {
                            self.last_mon_size = (mon_size.width, mon_size.height);
                            self.last_mon_pos = (mon_pos.x, mon_pos.y);
                            (self.win_x, self.win_y) =
                                self.compute_window_position(mon_pos, mon_size);
                            window
                                .set_outer_position(PhysicalPosition::new(self.win_x, self.win_y));
                        }
                    }
                }
            }
        }

        if now.duration_since(self.last_monitor_check) < Duration::from_secs(1) {
            return;
        }
        self.last_monitor_check = now;
        if let Some(monitor) = Self::get_target_monitor(window, self.config.monitor_index) {
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
}
