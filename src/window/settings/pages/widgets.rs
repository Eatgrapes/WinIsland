use crate::core::i18n::tr;
use crate::utils::settings_ui::items::*;

use super::super::SettingsApp;

impl SettingsApp {
    pub(crate) fn build_widget_items(&self) -> Vec<SettingsItem> {
        let theme = self.theme();
        vec![
            SettingsItem::PageTitle {
                text: tr("tab_widgets"),
            },
            SettingsItem::Spacer { height: 20.0 },
            SettingsItem::CenterText {
                text: tr("widgets_placeholder"),
                size: 13.0,
                color: theme.text_sec,
            },
        ]
    }
}
