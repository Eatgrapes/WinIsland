mod appearance;
mod behavior;
mod effects;

use crate::utils::settings_ui::items::SettingsItem;

use super::{PageInput, SettingsApp};

impl SettingsApp {
    pub(crate) fn build_general_items(&self) -> Vec<SettingsItem> {
        let mut items = self.build_appearance_page().into_items();
        items.extend(self.build_effects_page().into_items());
        items.extend(self.build_behavior_page().into_items());
        items
    }

    pub(crate) fn handle_general_click(&mut self, input: PageInput) {
        let appearance = self.build_appearance_page();
        let effects = self.build_effects_page();
        let appearance_height = appearance
            .items()
            .iter()
            .map(SettingsItem::height)
            .sum::<f32>();
        let effects_height = effects
            .items()
            .iter()
            .map(SettingsItem::height)
            .sum::<f32>();

        self.handle_appearance_click(input);
        self.handle_effects_click(PageInput {
            start_y: input.start_y + appearance_height,
            ..input
        });
        self.handle_behavior_click(PageInput {
            start_y: input.start_y + appearance_height + effects_height,
            ..input
        });
    }
}
