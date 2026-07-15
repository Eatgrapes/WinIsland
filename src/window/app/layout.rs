use windows::Win32::Foundation::HWND;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use crate::core::config::{PADDING, TOP_OFFSET};

use super::{App, HideEdge, IslandLayout};

impl App {
    pub(super) fn get_target_monitor(
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
                    if (dd.StateFlags & DISPLAY_DEVICE_ACTIVE) != DISPLAY_DEVICE_STATE_FLAGS(0) {
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

    pub(super) fn enforce_topmost(window: &Window, win_x: i32, win_y: i32, os_w: u32, os_h: u32) {
        if let Ok(handle) = window.window_handle()
            && let RawWindowHandle::Win32(raw) = handle.as_raw()
        {
            let hwnd = HWND(raw.hwnd.get() as *mut core::ffi::c_void);
            crate::utils::win32::set_window_topmost(hwnd, win_x, win_y, os_w as i32, os_h as i32);
        }
    }

    pub(super) fn compute_window_position(
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

    pub(super) fn nearest_hide_edge(&self) -> HideEdge {
        if self.last_mon_size.0 == 0 || self.last_mon_size.1 == 0 {
            return self.current_hide_edge();
        }
        let layout = self.compute_island_layout();
        let island_x = self.win_x + layout.current_island_x.round() as i32;
        let island_y = self.win_y + layout.current_island_y.round() as i32;
        let island_w = self.spring_w.value.round().max(1.0) as i32;
        let island_h = self.spring_h.value.round().max(1.0) as i32;
        let mon_right = self.last_mon_pos.0 + self.last_mon_size.0 as i32;
        let mon_bottom = self.last_mon_pos.1 + self.last_mon_size.1 as i32;
        [
            ((island_y - self.last_mon_pos.1).max(0), HideEdge::Top),
            ((mon_bottom - island_y - island_h).max(0), HideEdge::Bottom),
            ((island_x - self.last_mon_pos.0).max(0), HideEdge::Left),
            ((mon_right - island_x - island_w).max(0), HideEdge::Right),
        ]
        .into_iter()
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, edge)| edge)
        .unwrap_or(HideEdge::Top)
    }

    pub(super) fn current_hide_edge(&self) -> HideEdge {
        if self.auto_hidden
            || self.manually_hidden
            || self.hide_origin.is_some()
            || (self.config.fully_hide && self.is_dragging)
        {
            self.hide_edge
        } else if self.config.dock_position.is_bottom() {
            HideEdge::Bottom
        } else {
            HideEdge::Top
        }
    }

    pub(super) fn snap_to_hide_edge(&mut self, window: &Window) {
        let Some(monitor) = Self::get_target_monitor(window, self.config.monitor_index) else {
            return;
        };
        let layout = self.compute_island_layout();
        let mon_pos = monitor.position();
        let mon_size = monitor.size();
        let mon_right = mon_pos.x + mon_size.width as i32;
        let mon_bottom = mon_pos.y + mon_size.height as i32;
        let island_w = self.spring_w.value.round() as i32;
        let island_h = self.spring_h.value.round() as i32;

        match self.hide_edge {
            HideEdge::Top => self.win_y = mon_pos.y + TOP_OFFSET - layout.island_y.round() as i32,
            HideEdge::Bottom => {
                self.win_y = mon_bottom - TOP_OFFSET - island_h - layout.island_y.round() as i32
            }
            HideEdge::Left => self.win_x = mon_pos.x + TOP_OFFSET - layout.offset_x.round() as i32,
            HideEdge::Right => {
                self.win_x = mon_right - TOP_OFFSET - island_w - layout.offset_x.round() as i32
            }
        }
        window.set_outer_position(PhysicalPosition::new(self.win_x, self.win_y));
    }

    pub(super) fn restore_hide_origin(&mut self, window: &Window) {
        if self.spring_hide.value > 0.001 {
            return;
        }
        if let Some((win_x, win_y)) = self.hide_origin.take() {
            self.win_x = win_x;
            self.win_y = win_y;
            window.set_outer_position(PhysicalPosition::new(win_x, win_y));
        }
    }

    pub(super) fn capture_fully_hidden_hitbox(&mut self) {
        let layout = self.compute_island_layout();
        self.fully_hidden_hitbox = Some(super::IslandHitbox {
            x: layout.current_island_x,
            y: layout.current_island_y,
            width: self.spring_w.value as f64,
            height: self.spring_h.value as f64,
        });
    }

    pub(super) fn compute_island_layout(&self) -> IslandLayout {
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

        let hide_edge = self.current_hide_edge();
        let scale = self.config.global_scale as f64;
        let hidden_peek = (5.0 * scale).max(3.0);
        let hide_distance = match hide_edge {
            HideEdge::Top => {
                (self.spring_h.value as f64 - hidden_peek + TOP_OFFSET as f64).max(0.0)
            }
            HideEdge::Bottom => (self.spring_h.value as f64 - hidden_peek).max(0.0),
            HideEdge::Left => {
                (self.spring_w.value as f64 - hidden_peek + TOP_OFFSET as f64).max(0.0)
            }
            HideEdge::Right => (self.spring_w.value as f64 - hidden_peek).max(0.0),
        };
        let hide_offset = self.spring_hide.value as f64 * hide_distance;
        let (current_island_x, current_island_y) = match hide_edge {
            HideEdge::Top => (offset_x, island_y - hide_offset),
            HideEdge::Bottom => (offset_x, island_y + hide_offset),
            HideEdge::Left => (offset_x - hide_offset, island_y),
            HideEdge::Right => (offset_x + hide_offset, island_y),
        };
        let stable_base_y = if dock_bottom {
            self.os_h as f64 - PADDING as f64 / 2.0 - self.config.base_height as f64 * scale
        } else {
            PADDING as f64 / 2.0
        };
        let stable_island_y = match hide_edge {
            HideEdge::Top => stable_base_y - hide_offset,
            HideEdge::Bottom => stable_base_y + hide_offset,
            HideEdge::Left | HideEdge::Right => stable_base_y,
        };
        let hidden_handle_size = (24.0 * scale).max(14.0);
        let (hidden_handle_x, hidden_handle_y, hidden_handle_w, hidden_handle_h) = match hide_edge {
            HideEdge::Top => (
                current_island_x,
                (current_island_y + self.spring_h.value as f64
                    - hidden_peek
                    - hidden_handle_size * 0.35)
                    .max(0.0),
                self.spring_w.value as f64,
                hidden_handle_size,
            ),
            HideEdge::Bottom => (
                current_island_x,
                (self.os_h as f64 - PADDING as f64 / 2.0 - hidden_handle_size).max(0.0),
                self.spring_w.value as f64,
                hidden_handle_size,
            ),
            HideEdge::Left => (
                (current_island_x + self.spring_w.value as f64
                    - hidden_peek
                    - hidden_handle_size * 0.35)
                    .max(0.0),
                current_island_y,
                hidden_handle_size,
                self.spring_h.value as f64,
            ),
            HideEdge::Right => (
                (self.os_w as f64 - PADDING as f64 / 2.0 - hidden_handle_size).max(0.0),
                current_island_y,
                hidden_handle_size,
                self.spring_h.value as f64,
            ),
        };

        IslandLayout {
            offset_x,
            island_y,
            current_island_x,
            current_island_y,
            stable_island_y,
            hide_distance,
            hidden_handle_x,
            hidden_handle_y,
            hidden_handle_w,
            hidden_handle_h,
        }
    }

    pub(super) fn measure_lyric_text_width(&self, text: &str) -> f32 {
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

    pub(super) fn compute_lyric_target_width(
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
