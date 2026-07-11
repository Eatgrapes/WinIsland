use crate::core::i18n::tr;
use crate::utils::settings_ui::items::*;

use super::super::SettingsApp;

impl SettingsApp {
    pub(crate) fn build_widget_items(&self) -> Vec<SettingsItem> {
        vec![
            SettingsItem::PageTitle {
                text: tr("tab_widgets"),
            },
            SettingsItem::Spacer { height: 20.0 },
            SettingsItem::WidgetPreview,
        ]
    }
}
