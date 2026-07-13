use crate::core::config::{APP_AUTHOR, APP_VERSION};
use crate::core::i18n::tr;
use crate::utils::settings_ui::items::*;

use super::super::SettingsApp;

impl SettingsApp {
    pub(crate) fn build_about_items(&self) -> Vec<SettingsItem> {
        let theme = self.theme();
        vec![
            SettingsItem::PageTitle {
                text: tr("tab_about"),
            },
            SettingsItem::Spacer { height: 20.0 },
            SettingsItem::CenterText {
                text: "WinIsland".to_string(),
                size: 28.0,
                color: theme.text_pri,
            },
            SettingsItem::CenterText {
                text: format!("Version {}", APP_VERSION),
                size: 14.0,
                color: theme.text_sec,
            },
            SettingsItem::CenterText {
                text: format!("{} {}", tr("created_by"), APP_AUTHOR),
                size: 14.0,
                color: theme.text_sec,
            },
            SettingsItem::Spacer { height: 10.0 },
            SettingsItem::CenterLink {
                label: tr("visit_homepage"),
                color: theme.accent,
            },
        ]
    }
}
