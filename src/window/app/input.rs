use std::time::Instant;

use winit::event::ElementState;

use crate::core::config::WidgetKind;
use crate::ui::expanded::music_view::{
    get_next_btn_rect, get_pause_btn_rect, get_prev_btn_rect, get_progress_bar_rect,
    trigger_cover_flip, trigger_next_click, trigger_pause_click, trigger_prev_click,
};
use crate::ui::widget::widget_grid_layout;
use crate::utils::mouse::is_point_in_rect;

use super::{App, IslandLayout, should_show_widget_view};

impl App {
    pub(super) fn handle_input(&mut self, state: ElementState, px: i32, py: i32) {
        if self.is_cursor_suppressed {
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

    pub(super) fn handle_right_input(&mut self, state: ElementState, px: i32, py: i32) {
        if !self.config.right_click_drag || self.expanded || self.is_cursor_suppressed {
            return;
        }
        match state {
            ElementState::Pressed => {
                let rel_x = px - self.win_x;
                let rel_y = py - self.win_y;
                let layout = self.compute_island_layout();
                let is_hovering = is_point_in_rect(
                    rel_x as f64,
                    rel_y as f64,
                    layout.current_island_x,
                    layout.current_island_y,
                    self.spring_w.value as f64,
                    self.spring_h.value as f64,
                );
                if is_hovering {
                    self.right_press_cursor = Some((px, py));
                    self.right_drag_start_offset =
                        Some((self.config.position_x_offset, self.config.position_y_offset));
                }
            }
            ElementState::Released => {
                if self.is_right_dragging {
                    self.is_right_dragging = false;
                    crate::core::persistence::save_config(&self.config);
                    log::info!(
                        "Right click drag offsets saved: ({}, {})",
                        self.config.position_x_offset,
                        self.config.position_y_offset
                    );
                }
                self.right_press_cursor = None;
                self.right_drag_start_offset = None;
            }
        }
    }

    pub(super) fn handle_press(&mut self, rel_x: i32, rel_y: i32, layout: &IslandLayout) {
        let island_y = layout.island_y;
        let offset_x = layout.offset_x;
        let current_island_x = layout.current_island_x;
        let current_island_y = layout.current_island_y;

        let is_hovering_visible = is_point_in_rect(
            rel_x as f64,
            rel_y as f64,
            current_island_x,
            current_island_y,
            self.spring_w.value as f64,
            self.spring_h.value as f64,
        );

        let hidden_handle_x = layout.hidden_handle_x;
        let hidden_handle_y = layout.hidden_handle_y;
        let hidden_handle_w = layout.hidden_handle_w;
        let hidden_handle_h = layout.hidden_handle_h;
        let is_on_hidden_handle = (self.auto_hidden || self.manually_hidden)
            && is_point_in_rect(
                rel_x as f64,
                rel_y as f64,
                hidden_handle_x,
                hidden_handle_y,
                hidden_handle_w,
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
                let settings_hit = self
                    .config
                    .widget_layout
                    .iter()
                    .find(|entry| entry.widget == Some(WidgetKind::Settings))
                    .is_some_and(|entry| {
                        let layout = widget_grid_layout(
                            offset_x as f32,
                            island_y as f32,
                            w as f32,
                            h as f32,
                            self.config.global_scale,
                        );
                        let (x, y, width, height) =
                            layout.footprint_rect(WidgetKind::Settings, entry.slot);
                        is_point_in_rect(
                            rel_x as f64,
                            rel_y as f64,
                            x as f64 + w - page_shift,
                            y as f64,
                            width as f64,
                            height as f64,
                        )
                    });
                if settings_hit {
                    if let Ok(exe) = std::env::current_exe() {
                        let _ = std::process::Command::new(exe).arg("--settings").spawn();
                    }
                    return;
                }

                let arrow_x = offset_x + 7.5 * scale + w - page_shift;
                let arrow_y = island_y + h / 2.0;
                let adx = rel_x as f64 - arrow_x;
                let ady = rel_y as f64 - arrow_y;
                if adx * adx + ady * ady <= (12.0 * scale).powi(2) {
                    self.widget_view = false;
                    return;
                }
            }

            if view_val < 0.5 {
                let arrow_x = offset_x + w - 7.5 * scale;
                let arrow_y = island_y + h / 2.0;
                let adx = rel_x as f64 - arrow_x;
                let ady = rel_y as f64 - arrow_y;
                if adx * adx + ady * ady <= (12.0 * scale).powi(2) {
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
            self.drag_start_px = rel_x + self.win_x;
            self.drag_start_py = rel_y + self.win_y;
            self.drag_start_hide_val = self.spring_hide.value;
            self.drag_has_moved = false;
        }
    }

    pub(super) fn handle_release(&mut self, _py: i32) {
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
                    let media = self.smtc.get_info();
                    self.widget_view = should_show_widget_view(
                        self.config.smtc_enabled,
                        !media.title.is_empty(),
                        media.is_playing,
                    );
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
}
