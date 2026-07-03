use crate::core::config::DockPosition;
use crate::core::i18n::{available_langs, current_lang, tr};
use crate::utils::settings_ui::items::*;

use super::super::SettingsApp;

impl SettingsApp {
    pub(crate) fn build_general_items(&self) -> Vec<SettingsItem> {
        let mut items: Vec<SettingsItem> = vec![];

        match self.active_sub_page {
            0 => {
                items.push(SettingsItem::SectionHeader {
                    label: tr("section_appearance"),
                });
                items.push(SettingsItem::GroupStart);
                items.push(SettingsItem::RowStepper {
                    label: tr("global_scale"),
                    value: format!("{:.2}", self.config.global_scale),
                    enabled: true,
                });
                items.push(SettingsItem::RowStepper {
                    label: tr("base_width"),
                    value: self.config.base_width.to_string(),
                    enabled: true,
                });
                items.push(SettingsItem::RowStepper {
                    label: tr("base_height"),
                    value: self.config.base_height.to_string(),
                    enabled: true,
                });
                items.push(SettingsItem::RowStepper {
                    label: tr("expanded_width"),
                    value: self.config.expanded_width.to_string(),
                    enabled: true,
                });
                items.push(SettingsItem::RowStepper {
                    label: tr("expanded_height"),
                    value: self.config.expanded_height.to_string(),
                    enabled: true,
                });
                items.push(SettingsItem::RowStepper {
                    label: tr("position_x_offset"),
                    value: self.config.position_x_offset.to_string(),
                    enabled: true,
                });
                items.push(SettingsItem::RowStepper {
                    label: tr("position_y_offset"),
                    value: self.config.position_y_offset.to_string(),
                    enabled: true,
                });
                items.push(SettingsItem::RowStepper {
                    label: tr("font_size"),
                    value: format!("{:.0}", self.config.font_size),
                    enabled: true,
                });
                {
                    let monitors = self.get_monitor_list();
                    let selected_idx =
                        (self.config.monitor_index as usize).min(monitors.len().saturating_sub(1));
                    let options: Vec<(String, bool)> = monitors
                        .iter()
                        .enumerate()
                        .map(|(i, name)| (name.clone(), i == selected_idx))
                        .collect();
                    items.push(SettingsItem::RowSourceSelect {
                        label: tr("monitor"),
                        options,
                        enabled: true,
                    });
                }
                {
                    let dp = self.config.dock_position;
                    items.push(SettingsItem::RowSourceSelect {
                        label: tr("dock_position"),
                        options: vec![
                            (
                                tr("dock_position_top_center"),
                                dp == DockPosition::TopCenter,
                            ),
                            (tr("dock_position_top_left"), dp == DockPosition::TopLeft),
                            (tr("dock_position_top_right"), dp == DockPosition::TopRight),
                            (
                                tr("dock_position_bottom_center"),
                                dp == DockPosition::BottomCenter,
                            ),
                            (
                                tr("dock_position_bottom_left"),
                                dp == DockPosition::BottomLeft,
                            ),
                            (
                                tr("dock_position_bottom_right"),
                                dp == DockPosition::BottomRight,
                            ),
                        ],
                        enabled: true,
                    });
                }
                items.push(SettingsItem::GroupEnd);
            }
            1 => {
                items.push(SettingsItem::SectionHeader {
                    label: tr("section_effects"),
                });
                items.push(SettingsItem::GroupStart);
                items.push(SettingsItem::RowSourceSelect {
                    label: tr("settings_theme"),
                    options: vec![
                        (tr("theme_system"), self.config.settings_theme == "system"),
                        (tr("theme_light"), self.config.settings_theme == "light"),
                        (tr("theme_dark"), self.config.settings_theme == "dark"),
                    ],
                    enabled: true,
                });
                items.push(SettingsItem::RowSwitch {
                    label: tr("adaptive_border"),
                    on: self.config.adaptive_border,
                    enabled: true,
                });
                items.push(SettingsItem::RowSwitch {
                    label: tr("motion_blur"),
                    on: self.config.motion_blur,
                    enabled: true,
                });
                items.push(SettingsItem::GroupEnd);

                items.push(SettingsItem::GroupStart);
                items.push(SettingsItem::RowSourceSelect {
                    label: tr("island_style"),
                    options: vec![
                        (tr("style_default"), self.config.island_style == "default"),
                        (tr("style_glass"), self.config.island_style == "glass"),
                        (tr("style_mica"), self.config.island_style == "mica"),
                        (tr("style_dynamic"), self.config.island_style == "dynamic"),
                    ],
                    enabled: true,
                });
                items.push(SettingsItem::RowFontPicker {
                    label: tr("custom_font"),
                    btn_label: tr("font_select"),
                    reset_label: if self.config.custom_font_path.is_some() {
                        Some(tr("font_reset"))
                    } else {
                        None
                    },
                });
                items.push(SettingsItem::FontPreview {
                    has_custom_font: self.config.custom_font_path.is_some(),
                });
                items.push(SettingsItem::GroupEnd);
            }
            2 => {
                items.push(SettingsItem::SectionHeader {
                    label: tr("section_behavior"),
                });
                items.push(SettingsItem::GroupStart);
                items.push(SettingsItem::RowSwitch {
                    label: tr("start_boot"),
                    on: self.config.auto_start,
                    enabled: true,
                });
                items.push(SettingsItem::RowSwitch {
                    label: tr("auto_hide"),
                    on: self.config.auto_hide,
                    enabled: true,
                });
                items.push(SettingsItem::RowSwitch {
                    label: tr("right_click_drag"),
                    on: self.config.right_click_drag,
                    enabled: true,
                });
                if self.config.auto_hide {
                    items.push(SettingsItem::RowStepper {
                        label: tr("hide_delay"),
                        value: format!("{:.0}", self.config.auto_hide_delay),
                        enabled: true,
                    });
                }
                {
                    let cur = current_lang();
                    let options: Vec<_> = available_langs()
                        .iter()
                        .map(|l| (l.name.clone(), l.code == cur))
                        .collect();
                    items.push(SettingsItem::RowSourceSelect {
                        label: tr("language"),
                        options,
                        enabled: true,
                    });
                }
                items.push(SettingsItem::GroupEnd);

                items.push(SettingsItem::SectionHeader {
                    label: tr("section_updates"),
                });
                items.push(SettingsItem::GroupStart);
                items.push(SettingsItem::RowSwitch {
                    label: tr("check_updates"),
                    on: self.config.check_for_updates,
                    enabled: true,
                });
                if self.config.check_for_updates {
                    items.push(SettingsItem::RowSourceSelect {
                        label: tr("update_channel"),
                        options: vec![
                            (tr("channel_stable"), self.config.update_channel == "stable"),
                            (tr("channel_beta"), self.config.update_channel == "beta"),
                        ],
                        enabled: true,
                    });
                    items.push(SettingsItem::RowStepper {
                        label: tr("update_interval"),
                        value: format!("{:.0}", self.config.update_check_interval),
                        enabled: true,
                    });
                }
                items.push(SettingsItem::RowButton {
                    label: tr("check_updates_manual"),
                    btn_label: tr("update_check_btn"),
                    enabled: true,
                });
                items.push(SettingsItem::GroupEnd);

                items.push(SettingsItem::Spacer { height: 10.0 });
                items.push(SettingsItem::CenterLink {
                    label: tr("reset_defaults"),
                    color: self.theme().danger,
                });
            }
            _ => {}
        }
        items
    }
}
