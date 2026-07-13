use crate::utils::settings_ui::items::SIDEBAR_PAD;
use crate::utils::settings_ui::{WidgetPreviewHit, hover_test};

use super::pages::PageInput;
use super::{POPUP_OPACITY_KEY, SIDEBAR_ROW_H, SIDEBAR_W, SettingsApp};

impl SettingsApp {
    pub(super) fn handle_click(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let (mouse_x, mouse_y) = self.logical_mouse_pos;

        if self.popup.is_some() {
            let selection = self.popup.as_ref().and_then(|popup| {
                popup
                    .hit_test_item(mouse_x, mouse_y)
                    .and_then(|index| popup.values.get(index))
                    .map(|value| (popup.on_select, value.clone()))
            });
            self.popup = None;
            self.anim.set_with_speed(POPUP_OPACITY_KEY, 0.0, 0.3);
            if let Some((on_select, value)) = selection {
                on_select(self, &value);
                self.persist_settings_change();
            } else if let Some(window) = &self.window {
                window.request_redraw();
            }
            return;
        }

        if mouse_x < SIDEBAR_W {
            let start_y = 60.0;
            for page in 0..4 {
                let row_y = start_y + page as f32 * (SIDEBAR_ROW_H + 2.0);
                if mouse_y >= row_y
                    && mouse_y <= row_y + SIDEBAR_ROW_H
                    && (SIDEBAR_PAD..=SIDEBAR_W - SIDEBAR_PAD).contains(&mouse_x)
                {
                    if self.active_page != page {
                        self.active_page = page;
                        self.reset_scroll();
                    }
                    return;
                }
            }
            return;
        }

        let scale = self
            .window
            .as_ref()
            .map(|window| window.scale_factor() as f32)
            .unwrap_or(1.0);
        let content_width = self.win_w / scale - SIDEBAR_W;

        let start_y = if self.active_page == 0 { 100.0 } else { 50.0 };
        let input = PageInput {
            x: mouse_x - SIDEBAR_W,
            y: mouse_y + self.scroll_y,
            width: content_width,
            start_y,
        };

        match self.active_page {
            0 => self.handle_general_click(input),
            1 => self.handle_music_click(input),
            2 => {
                if self.handle_widget_click()
                    && let Some(window) = &self.window
                {
                    window.request_redraw();
                }
            }
            3 => self.handle_about_click(input),
            _ => {}
        }
    }

    fn reset_scroll(&mut self) {
        self.scroll_y = 0.0;
        self.target_scroll_y = 0.0;
        self.scroll_vel_y = 0.0;
        self.mark_items_dirty();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    pub(super) fn get_hover_state(&mut self) -> bool {
        let (mouse_x, mouse_y) = self.logical_mouse_pos;
        let over_window_control = [(20.0_f32, 24.0_f32), (40.0, 24.0), (60.0, 24.0)]
            .iter()
            .any(|&(x, y)| (mouse_x - x).powi(2) + (mouse_y - y).powi(2) <= 36.0);
        if over_window_control {
            return true;
        }

        if let Some(popup) = &self.popup {
            let menu = popup.menu_rect();
            if mouse_x >= menu.left
                && mouse_x <= menu.right
                && mouse_y >= menu.top
                && mouse_y <= menu.bottom
            {
                return true;
            }
        }

        if mouse_x < SIDEBAR_W {
            let start_y = 60.0;
            for page in 0..4 {
                let row_y = start_y + page as f32 * (SIDEBAR_ROW_H + 2.0);
                if mouse_y >= row_y
                    && mouse_y <= row_y + SIDEBAR_ROW_H
                    && (SIDEBAR_PAD..=SIDEBAR_W - SIDEBAR_PAD).contains(&mouse_x)
                {
                    return true;
                }
            }
            return false;
        }

        let scale = self
            .window
            .as_ref()
            .map(|window| window.scale_factor() as f32)
            .unwrap_or(1.0);
        let content_width = self.win_w / scale - SIDEBAR_W;
        if self.widget_dragging.is_some() {
            return true;
        }
        if self
            .widget_preview_hit_at_mouse()
            .is_some_and(|hit| hit != WidgetPreviewHit::None)
        {
            return true;
        }

        let start_y = if self.active_page == 0 { 100.0 } else { 50.0 };
        self.ensure_items_cache();
        hover_test(
            &self.cached_items,
            mouse_x - SIDEBAR_W,
            mouse_y + self.scroll_y,
            start_y,
            content_width,
        )
    }
}
