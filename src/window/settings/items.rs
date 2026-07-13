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
        let switch_states: Vec<bool> = self
            .cached_items
            .iter()
            .filter_map(|item| match item {
                SettingsItem::RowSwitch { on, .. } => Some(*on),
                _ => None,
            })
            .collect();
        let switch_context = (self.active_page, self.active_sub_page);
        if self.switch_anim_context != switch_context
            || self.switch_anim.len() != switch_states.len()
        {
            self.switch_anim = crate::utils::settings_ui::SwitchAnimator::new(&switch_states);
            self.switch_anim_context = switch_context;
        } else {
            self.switch_anim.set_targets(&switch_states);
        }
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
