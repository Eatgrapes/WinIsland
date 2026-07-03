use crate::utils::settings_ui::content_height;
use crate::utils::settings_ui::items::SettingsItem;

use super::{CONTENT_START_Y, SUB_TAB_H, SUB_TAB_START_Y, SettingsApp};

impl SettingsApp {
    pub(crate) fn build_current_items(&self) -> Vec<SettingsItem> {
        match self.active_page {
            0 => self.build_general_items(),
            1 => self.build_music_items(),
            2 => self.build_widget_items(),
            3 => self.build_about_items(),
            _ => vec![],
        }
    }

    pub(crate) fn rebuild_items_cache(&mut self) {
        self.cached_items = self.build_current_items();
        let content_start_y = if self.active_page == 0 {
            SUB_TAB_START_Y + SUB_TAB_H + CONTENT_START_Y
        } else {
            50.0
        };
        self.cached_content_height = content_height(&self.cached_items, content_start_y);
        let scale = self
            .window
            .as_ref()
            .map(|w| w.scale_factor() as f32)
            .unwrap_or(1.0);
        let view_h = self.win_h / scale;
        self.cached_max_scroll = (self.cached_content_height - view_h + 20.0).max(0.0);
        self.cached_row_tops.clear();
        self.cached_row_heights.clear();
        let mut y = content_start_y;
        for item in &self.cached_items {
            if item.is_row() {
                self.cached_row_tops.push(y);
                self.cached_row_heights.push(item.height());
            }
            y += item.height();
        }
        self.total_rows = self.cached_row_tops.len();
        self.items_dirty = false;
    }

    pub(crate) fn ensure_items_cache(&mut self) {
        if self.items_dirty {
            self.rebuild_items_cache();
        }
    }

    pub(crate) fn mark_items_dirty(&mut self) {
        self.items_dirty = true;
    }
}
