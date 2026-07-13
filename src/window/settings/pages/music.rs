use crate::core::i18n::tr;
use crate::utils::settings_ui::items::*;

use super::super::SettingsApp;

impl SettingsApp {
    pub(crate) fn build_music_items(&self) -> Vec<SettingsItem> {
        let show_lyrics = self.config.show_lyrics;
        let enabled = self.config.smtc_enabled;
        let source = &self.config.lyrics_source;

        let mut items = vec![
            SettingsItem::PageTitle {
                text: tr("tab_music"),
            },
            SettingsItem::SectionHeader {
                label: tr("section_playback"),
            },
            SettingsItem::GroupStart,
            SettingsItem::RowSwitch {
                label: tr("smtc_control"),
                on: self.config.smtc_enabled,
                enabled: true,
            },
            SettingsItem::GroupEnd,
            SettingsItem::SectionHeader {
                label: tr("section_lyrics"),
            },
            SettingsItem::GroupStart,
            SettingsItem::RowSwitch {
                label: tr("show_lyrics"),
                on: self.config.show_lyrics,
                enabled: true,
            },
            SettingsItem::RowSourceSelect {
                label: tr("lyrics_source"),
                options: vec![
                    ("163".to_string(), source == "163"),
                    ("LRCLIB".to_string(), source == "lrclib"),
                ],
                enabled: show_lyrics,
            },
            SettingsItem::RowSwitch {
                label: tr("lyrics_fallback"),
                on: if show_lyrics {
                    self.config.lyrics_fallback
                } else {
                    false
                },
                enabled: show_lyrics,
            },
            SettingsItem::RowStepper {
                label: tr("lyrics_delay"),
                value: format!("{:.1}", self.config.lyrics_delay),
                enabled: show_lyrics,
            },
            SettingsItem::RowSwitch {
                label: tr("lyrics_scroll"),
                on: if show_lyrics {
                    self.config.lyrics_scroll
                } else {
                    false
                },
                enabled: show_lyrics,
            },
            SettingsItem::RowStepper {
                label: tr("lyrics_scroll_max_width"),
                value: format!("{}", self.config.lyrics_scroll_max_width as i32),
                enabled: show_lyrics && self.config.lyrics_scroll,
            },
            SettingsItem::RowFolderPicker {
                label: tr("lyrics_local_dir"),
                btn_label: tr("folder_select"),
                clear_label: self
                    .config
                    .lyrics_local_dir
                    .as_ref()
                    .filter(|p| !p.is_empty())
                    .map(|_| tr("folder_clear")),
                current_path: self
                    .config
                    .lyrics_local_dir
                    .clone()
                    .filter(|p| !p.is_empty()),
                enabled: show_lyrics,
            },
            SettingsItem::GroupEnd,
            SettingsItem::SectionHeader {
                label: tr("media_apps"),
            },
            SettingsItem::GroupStart,
        ];

        if self.detected_apps.is_empty() {
            items.push(SettingsItem::RowLabel {
                label: tr("no_sessions"),
            });
        } else {
            for app in &self.detected_apps {
                let display_name = app.split('!').next().unwrap_or(app).to_string();
                let active = self.config.smtc_apps.contains(app);
                items.push(SettingsItem::RowAppItem {
                    label: display_name,
                    active,
                    enabled,
                });
            }
        }
        items.push(SettingsItem::GroupEnd);
        items
    }
}
