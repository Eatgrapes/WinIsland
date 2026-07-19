use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use winit::dpi::PhysicalPosition;
use winit::event_loop::ActiveEventLoop;

use crate::core::config::MIN_HIDDEN_WIDTH;
use crate::ui::compact::CompactOverlayState;
use crate::ui::expanded::music_view::{
    get_progress_bar_rect, set_progress_dragging, set_progress_hover,
};
use crate::utils::color::get_island_border_weights;
use crate::utils::mouse::{
    get_global_cursor_pos, is_cursor_hidden, is_foreground_fullscreen, is_left_button_pressed,
    is_point_in_rect, is_point_in_rounded_rect,
};

use super::{App, HideEdge, RIGHT_DRAG_THRESHOLD};

impl App {
    pub(super) fn on_about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let window = match self.window.clone() {
            Some(w) => w,
            None => return,
        };
        Self::enforce_topmost(&window, self.win_x, self.win_y, self.os_w, self.os_h);
        let frame_start = Instant::now();
        self.handle_tray_events(&window, event_loop);
        self.reload_config_if_changed(&window);
        if self.is_hidden() && !self.can_hide_to_edge(self.current_hide_edge()) {
            self.reveal_island();
        }

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
        if let Some((start_cx, start_cy)) = self.right_press_cursor
            && let Some((start_ox, start_oy)) = self.right_drag_start_offset
        {
            let dx = px - start_cx;
            let dy = py - start_cy;
            if !self.is_right_dragging
                && (dx.abs() >= RIGHT_DRAG_THRESHOLD || dy.abs() >= RIGHT_DRAG_THRESHOLD)
            {
                self.is_right_dragging = true;
                log::info!(
                    "Right click drag started at offsets: ({}, {})",
                    start_ox,
                    start_oy
                );
            }
            if self.is_right_dragging {
                self.config.position_x_offset = start_ox + dx;
                self.config.position_y_offset = start_oy + dy;

                if let Some(monitor) = Self::get_target_monitor(&window, self.config.monitor_index)
                {
                    let mon_size = monitor.size();
                    let mon_pos = monitor.position();
                    let (new_x, new_y) = self.compute_window_position(mon_pos, mon_size);
                    if new_x != self.win_x || new_y != self.win_y {
                        self.win_x = new_x;
                        self.win_y = new_y;
                        window.set_outer_position(PhysicalPosition::new(self.win_x, self.win_y));
                    }
                }
                window.request_redraw();
            }
        }

        if self.frame_count.is_multiple_of(10) {
            let prev_fullscreen = self.is_fullscreen_suppressed;
            self.is_fullscreen_suppressed = is_foreground_fullscreen(
                self.last_mon_pos.0,
                self.last_mon_pos.1,
                self.last_mon_size.0,
                self.last_mon_size.1,
            );
            self.is_cursor_suppressed = is_cursor_hidden();
            if self.is_fullscreen_suppressed != prev_fullscreen {
                if self.is_fullscreen_suppressed {
                    let hide_started = if self.is_hidden() {
                        true
                    } else {
                        let hide_edge = self.nearest_hide_edge();
                        self.prepare_hide(&window, hide_edge)
                    };
                    if hide_started {
                        self.expanded = false;
                        self.widget_view = false;
                        self.fullscreen_hidden = true;
                    }
                } else {
                    let was_fullscreen_hidden = self.fullscreen_hidden;
                    self.fullscreen_hidden = false;
                    self.idle_timer = Instant::now();
                    if was_fullscreen_hidden && !self.is_hidden() {
                        self.spring_hide.velocity = -0.65;
                    }
                }
                log::info!(
                    "Fullscreen state: {}",
                    if self.is_fullscreen_suppressed {
                        "hidden"
                    } else {
                        "normal"
                    }
                );
                window.request_redraw();
            }
        }

        let rel_x = px - self.win_x;
        let rel_y = py - self.win_y;
        let layout = self.compute_island_layout();
        let island_y = layout.island_y;
        let offset_x = layout.offset_x;
        let current_island_x = layout.current_island_x;
        let current_island_y = layout.current_island_y;
        let is_hovering_visible = is_point_in_rounded_rect(
            rel_x as f64,
            rel_y as f64,
            current_island_x,
            current_island_y,
            self.spring_w.value as f64,
            self.spring_h.value as f64,
            self.spring_r.value as f64,
        );
        let is_on_hidden_reveal = self.is_hidden()
            && self.config.hidden_width <= MIN_HIDDEN_WIDTH
            && self.spring_hide.value >= 0.999
            && is_point_in_rect(
                rel_x as f64,
                rel_y as f64,
                layout.hidden_reveal_x,
                layout.hidden_reveal_y,
                layout.hidden_reveal_w,
                layout.hidden_reveal_h,
            );

        if self.is_cursor_suppressed {
            let _ = window.set_cursor_hittest(false);
        } else {
            let _ = window.set_cursor_hittest(is_hovering_visible || is_on_hidden_reveal);
        }

        let mut music_active = false;
        let media = self.smtc.get_info();
        self.audio.set_target_app_id(&media.source_app_id);
        if self.config.smtc_enabled && !media.title.is_empty() {
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
                crate::utils::backdrop::clear_blurred_cover_cache();
                window.request_redraw();
            }
        }

        let is_paused_idle = music_active && !media.is_playing;
        let compact_state = if !self.expanded && !self.is_hidden() {
            CompactOverlayState::Present
        } else if self.auto_hidden && !self.manually_hidden && !self.fullscreen_hidden {
            CompactOverlayState::Defer
        } else {
            CompactOverlayState::Discard
        };
        let compact_event = self
            .compact_overlay
            .update(compact_state, self.config.notification_display);
        if compact_event && self.auto_hidden && !self.manually_hidden {
            self.auto_hidden = false;
            self.idle_timer = Instant::now();
            self.spring_hide.velocity = -0.65;
            self.compact_overlay.update(
                CompactOverlayState::Present,
                self.config.notification_display,
            );
            log::info!("Island un-hidden (compact overlay event)");
        }
        let compact_overlay_visible = self.compact_overlay.is_visible();
        let is_idle = !is_hovering_visible
            && !self.expanded
            && !self.is_dragging
            && !compact_overlay_visible
            && (!music_active || is_paused_idle);
        if !self.config.auto_hide {
            let was_auto_hidden = self.auto_hidden;
            self.auto_hidden = false;
            self.idle_timer = Instant::now();
            if was_auto_hidden && !self.is_hidden() {
                self.spring_hide.velocity = -0.65;
            }
        } else if media.is_playing && self.auto_hidden && !self.manually_hidden {
            self.auto_hidden = false;
            self.idle_timer = Instant::now();
            if !self.is_hidden() {
                self.spring_hide.velocity = -0.65;
            }
            log::info!("Island un-hidden (media playing)");
        } else if !self.is_hidden() && is_idle {
            if self.idle_timer.elapsed().as_secs_f32() > self.config.auto_hide_delay {
                let hide_edge = self.nearest_hide_edge();
                if self.prepare_hide(&window, hide_edge) {
                    self.auto_hidden = true;
                    log::info!(
                        "Island auto-hidden (idle {:.1}s)",
                        self.config.auto_hide_delay
                    );
                }
            }
        } else if !self.is_hidden() && !is_idle {
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

        if self.is_dragging && !self.is_hidden() {
            let upward_distance = self.drag_start_py - py;
            let horizontal_distance = px - self.drag_start_px;
            if upward_distance.abs() > 3 || horizontal_distance.abs() > 3 {
                self.drag_has_moved = true;
            }
            if upward_distance > 3 && self.hide_origin.is_none() {
                self.prepare_hide(&window, HideEdge::Top);
            }
            if self.hide_origin.is_some() {
                let drag_layout = self.compute_island_layout();
                if drag_layout.hide_distance > 0.0 {
                    let mut new_val = self.drag_start_hide_val
                        + (upward_distance as f32 / drag_layout.hide_distance as f32);
                    new_val = new_val.clamp(0.0, 1.0);
                    self.spring_hide.value = new_val;
                    self.spring_hide.velocity = 0.0;
                    window.request_redraw();
                }
            }
        } else {
            let hide_target = if self.is_hidden() { 1.0 } else { 0.0 };
            let (stiffness, damping) = if self.is_hidden() {
                (0.12, 0.70)
            } else {
                (0.08, 0.78)
            };
            self.spring_hide
                .update_dt(hide_target, stiffness, damping, dt);
        }
        if !self.is_hidden() {
            self.restore_hide_origin(&window);
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
                let island_cx = self.win_x
                    + (current_island_x + (self.spring_w.value as f64) / 2.0).round() as i32;
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
                self.current_lyric_text = lyric.to_owned();
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

        let lyric_target_w = self.compute_lyric_target_width(&window, music_active, is_paused, dt);
        let default_target_h = (if self.expanded {
            self.config.expanded_height
        } else {
            self.config.base_height
        }) * self.config.global_scale;
        let default_target_r = if self.expanded {
            32.0 * self.config.global_scale
        } else {
            (self.config.base_height * self.config.global_scale) / 2.0
        };
        let (target_w, target_h, target_r) = if let Some(size) = self.compact_overlay.target_size(
            self.config.base_width,
            self.config.base_height,
            self.config.global_scale,
        ) {
            (size.width, size.height, size.height / 2.0)
        } else {
            (lyric_target_w, default_target_h, default_target_r)
        };
        let target_view = if self.widget_view { 1.0 } else { 0.0 };
        self.spring_w.update_dt(target_w, 0.10, 0.68, dt);
        self.spring_h.update_dt(target_h, 0.10, 0.68, dt);
        self.spring_r.update_dt(target_r, 0.10, 0.68, dt);
        self.spring_view.update_dt(target_view, 0.12, 0.68, dt);

        let is_glass_or_mica = self.config.island_style == "glass"
            || self.config.island_style == "dynamic"
            || self.config.island_style == "mica";
        let should_periodic_redraw = is_glass_or_mica
            && !self.is_hidden()
            && self.last_glass_refresh.elapsed().as_millis() >= 1000;

        if should_periodic_redraw {
            self.last_glass_refresh = Instant::now();
        }

        if self.expanded
            || (!self.is_hidden() && media.is_playing)
            || self.spring_w.velocity.abs() > 0.001
            || self.spring_h.velocity.abs() > 0.001
            || self.spring_r.velocity.abs() > 0.001
            || self.spring_view.velocity.abs() > 0.001
            || compact_overlay_visible
            || should_periodic_redraw
            || self.right_press_cursor.is_some()
            || self.is_right_dragging
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
