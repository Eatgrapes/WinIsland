mod appearance;
mod behavior;
mod effects;

use crate::utils::settings_ui::items::SettingsItem;

use super::{PageInput, SettingsApp};

impl SettingsApp {
    pub(crate) fn build_general_items(&self) -> Vec<SettingsItem> {
        match self.active_sub_page {
            0 => self.build_appearance_page().into_items(),
            1 => self.build_effects_page().into_items(),
            2 => self.build_behavior_page().into_items(),
            _ => Vec::new(),
        }
    }

    pub(crate) fn handle_general_click(&mut self, input: PageInput) {
        match self.active_sub_page {
            0 => self.handle_appearance_click(input),
            1 => self.handle_effects_click(input),
            2 => self.handle_behavior_click(input),
            _ => {}
        }
    }
}
