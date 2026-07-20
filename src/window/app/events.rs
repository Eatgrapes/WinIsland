use std::time::Instant;

use winit::event::{ElementState, MouseButton, TouchPhase, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::core::render::draw_island;
use crate::utils::blur::calculate_blur_sigmas;
use crate::utils::mouse::get_global_cursor_pos;

use super::App;

impl App {
    pub(super) fn on_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: WindowId,
        event: WindowEvent,
    ) {
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
                WindowEvent::MouseInput { state, button, .. } => {
                    let (px, py) = get_global_cursor_pos();
                    if button == MouseButton::Left {
                        self.handle_input(event_loop, state, px, py);
                    } else if button == MouseButton::Right {
                        self.handle_right_input(state, px, py);
                    }
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
                            self.handle_input(event_loop, ElementState::Pressed, px, py);
                        }
                        TouchPhase::Moved => {
                            self.touch_id = Some(touch.id);
                        }
                        TouchPhase::Ended | TouchPhase::Cancelled => {
                            self.handle_input(event_loop, ElementState::Released, px, py);
                            self.touch_id = None;
                        }
                    }
                }
                WindowEvent::RedrawRequested => {
                    let island_layout = self.compute_island_layout();
                    let is_hidden = self.is_hidden();
                    if let Some(surface) = self.surface.as_mut() {
                        let dt =
                            (self.last_render_time.elapsed().as_secs_f32() * 60.0).clamp(0.1, 6.0);
                        self.last_render_time = Instant::now();
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
                        if let Some(ps) = crate::plugin::manager::drain_pending_media_source() {
                            let (cover, hash) = if !ps.cover_data.is_empty() {
                                use std::collections::hash_map::DefaultHasher;
                                use std::hash::{Hash, Hasher};
                                let mut hasher = DefaultHasher::new();
                                ps.cover_data.hash(&mut hasher);
                                (
                                    Some(skia_safe::Data::new_copy(&ps.cover_data)),
                                    hasher.finish(),
                                )
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
                        if let Some(ref mut info) = self.plugin_media_source
                            && info.is_playing
                        {
                            let elapsed = info.last_update.elapsed().as_millis() as u64;
                            info.position_ms = info.position_ms.saturating_add(elapsed);
                            info.last_update = Instant::now();
                        }
                        let spectrum = self.audio.get_spectrum();
                        let default_media_info = crate::core::smtc::MediaInfo::default();
                        let media_info = if let Some(info) = self.plugin_media_source.as_mut() {
                            info.spectrum = spectrum;
                            &*info
                        } else if self.config.smtc_enabled {
                            self.smtc_media_info.spectrum = spectrum;
                            &self.smtc_media_info
                        } else {
                            &default_media_info
                        };
                        let seeking_media_info =
                            if self.seeking_progress && self.seeking_duration_ms > 0 {
                                let mut preview = media_info.clone();
                                preview.position_ms = self.seeking_preview_ms;
                                preview.last_update = Instant::now();
                                Some(preview)
                            } else {
                                None
                            };
                        let media_info = seeking_media_info.as_ref().unwrap_or(media_info);
                        let music_active = self.config.smtc_enabled && !media_info.title.is_empty();
                        self.audio.set_gate_override(music_active && !is_hidden);
                        self.ctx_mgr.set_smtc_active(music_active);
                        crate::plugin::manager::drain_pending_contexts(&mut self.ctx_mgr);
                        self.ctx_mgr.tick();
                        let mini_content = self.ctx_mgr.current_mini();

                        let _ = draw_island(
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
                                    hide_progress: self.spring_hide.value
                                        * island_layout.content_hide_ratio,
                                    island_x: island_layout.current_island_x as f32,
                                    island_y: island_layout.current_island_y as f32,
                                    stable_island_y: island_layout.stable_island_y as f32,
                                    base_h: self.config.base_height * self.config.global_scale,
                                },
                                media: crate::core::render::MediaParams {
                                    media: media_info,
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
                                    lyrics_delay: self.config.lyrics_delay,
                                    dt,
                                    widget_layout: &self.config.widget_layout,
                                },
                                mini_content,
                                compact_overlay: &self.compact_overlay,
                            },
                        );
                    }
                }
                _ => (),
            }
        }
    }
}
