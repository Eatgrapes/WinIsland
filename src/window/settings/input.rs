use super::{
    CONTENT_START_Y, POPUP_OPACITY_KEY, SIDEBAR_ROW_H, SIDEBAR_W, SUB_TAB_H, SUB_TAB_START_Y,
};
use super::{PopupKind, PopupState, SettingsApp};
use crate::core::config::{
    APP_HOMEPAGE, AppConfig, DockPosition, clear_widget_slot, place_widget_in_layout,
};
use crate::core::i18n::{available_langs, current_lang, init_i18n, set_lang, tr};
use crate::core::persistence::save_config;
use crate::utils::autostart::set_autostart;
use crate::utils::font::FontManager;
use crate::utils::settings_ui::items::*;
use crate::utils::settings_ui::*;
use skia_safe::Rect;

impl SettingsApp {
    fn widget_preview_item_y(&mut self) -> Option<f32> {
        if self.active_page != 2 {
            return None;
        }
        self.ensure_items_cache();
        let mut y = 50.0;
        for item in &self.cached_items {
            if matches!(item, SettingsItem::WidgetPreview) {
                return Some(y);
            }
            y += item.height();
        }
        None
    }

    pub(crate) fn widget_preview_hit_at_mouse(&mut self) -> Option<WidgetPreviewHit> {
        let item_y = self.widget_preview_item_y()?;
        let scale = self
            .window
            .as_ref()
            .map(|w| w.scale_factor() as f32)
            .unwrap_or(1.0);
        let width = self.win_w / scale - SIDEBAR_W;
        let (mx, my) = self.logical_mouse_pos;
        if mx < SIDEBAR_W {
            return None;
        }
        Some(widget_preview_hit_test(
            mx - SIDEBAR_W,
            my + self.scroll_y,
            item_y,
            width,
            self.config.expanded_width,
            self.config.expanded_height,
        ))
    }

    pub(crate) fn handle_widget_drag_press(&mut self) -> bool {
        let Some(hit) = self.widget_preview_hit_at_mouse() else {
            return false;
        };
        if let WidgetPreviewHit::Source(widget) = hit {
            self.widget_dragging = Some(widget);
            self.widget_drag_hover_slot = None;
            return true;
        }
        false
    }

    pub(crate) fn handle_widget_drag_release(&mut self) -> bool {
        let Some(widget) = self.widget_dragging.take() else {
            return false;
        };
        let target_slot = self.widget_drag_hover_slot.take();
        if let Some(slot) = target_slot {
            place_widget_in_layout(&mut self.config.widget_layout, widget, slot);
            save_config(&self.config);
            self.mark_items_dirty();
        }
        true
    }

    fn handle_widget_click(&mut self) -> bool {
        let Some(item_y) = self.widget_preview_item_y() else {
            return false;
        };
        let scale = self
            .window
            .as_ref()
            .map(|w| w.scale_factor() as f32)
            .unwrap_or(1.0);
        let width = self.win_w / scale - SIDEBAR_W;
        let (mx, my) = self.logical_mouse_pos;
        if mx < SIDEBAR_W {
            return false;
        }
        let geom = widget_grid_geom(
            item_y,
            width,
            self.config.expanded_width,
            self.config.expanded_height,
        );
        let anchor = self.config.widget_layout.iter().find_map(|entry| {
            let widget = entry.widget?;
            let (x, y, w, _) = geom.footprint_rect(widget, entry.slot);
            widget_delete_button_hit(mx - SIDEBAR_W, my + self.scroll_y, x, y, w, geom.cap_scale)
                .then_some(entry.slot)
        });
        let Some(anchor) = anchor else {
            return false;
        };

        clear_widget_slot(&mut self.config.widget_layout, anchor);
        save_config(&self.config);
        self.mark_items_dirty();
        true
    }

    pub(super) fn handle_click(&mut self, _el: &winit::event_loop::ActiveEventLoop) {
        let (mx, my) = self.logical_mouse_pos;

        if let Some(popup) = &self.popup {
            if let Some(i) = popup.hit_test_item(mx, my) {
                let value = popup.values[i].clone();
                match popup.kind {
                    PopupKind::LyricsSource => {
                        self.config.lyrics_source = value;
                    }
                    PopupKind::Language => {
                        self.config.language = value;
                        set_lang(&self.config.language);
                    }
                    PopupKind::Monitor => {
                        self.config.monitor_index = value.parse::<i32>().unwrap_or(0);
                    }
                    PopupKind::IslandStyle => {
                        self.config.island_style = value;
                    }
                    PopupKind::DockPositionPopup => {
                        self.config.dock_position = value
                            .parse::<DockPosition>()
                            .unwrap_or(DockPosition::TopCenter);
                    }
                    PopupKind::SettingsTheme => {
                        self.config.settings_theme = value.clone();
                        self.update_theme();
                    }
                    PopupKind::UpdateChannel => {
                        self.config.update_channel = value;
                    }
                }
                save_config(&self.config);
                self.mark_items_dirty();
            }
            self.popup = None;
            self.anim.set_with_speed(POPUP_OPACITY_KEY, 0.0, 0.3);
            if let Some(win) = &self.window {
                win.request_redraw();
            }
            return;
        }

        if mx < SIDEBAR_W {
            let pages = 4;
            let start_y = 60.0;
            for i in 0..pages {
                let row_y = start_y + i as f32 * (SIDEBAR_ROW_H + 2.0);
                if my >= row_y
                    && my <= row_y + SIDEBAR_ROW_H
                    && (SIDEBAR_PAD..=SIDEBAR_W - SIDEBAR_PAD).contains(&mx)
                {
                    if self.active_page != i as usize {
                        self.active_page = i as usize;
                        self.scroll_y = 0.0;
                        self.target_scroll_y = 0.0;
                        self.scroll_vel_y = 0.0;
                        self.mark_items_dirty();
                        if let Some(win) = &self.window {
                            win.request_redraw();
                        }
                    }
                    return;
                }
            }
            return;
        }

        let scale = self
            .window
            .as_ref()
            .map(|w| w.scale_factor() as f32)
            .unwrap_or(1.0);
        let content_w = self.win_w / scale - SIDEBAR_W;

        if self.active_page == 0 && (SUB_TAB_START_Y..=SUB_TAB_START_Y + SUB_TAB_H).contains(&my) {
            let tabs = [
                tr("section_appearance"),
                tr("section_effects"),
                tr("section_behavior"),
            ];
            let tab_count = tabs.len();
            let tab_w = content_w / tab_count as f32;
            let rel_x = mx - SIDEBAR_W;
            let tab_idx = (rel_x / tab_w) as usize;
            if tab_idx < tab_count && self.active_sub_page != tab_idx {
                self.active_sub_page = tab_idx;
                self.scroll_y = 0.0;
                self.target_scroll_y = 0.0;
                self.scroll_vel_y = 0.0;
                self.mark_items_dirty();
                if let Some(win) = &self.window {
                    win.request_redraw();
                }
            }
            return;
        }

        let content_x = mx - SIDEBAR_W;
        let content_start_y = if self.active_page == 0 {
            SUB_TAB_START_Y + SUB_TAB_H + CONTENT_START_Y
        } else {
            50.0
        };
        let content_y = my + self.scroll_y;
        let items = self.build_current_items();

        match self.active_page {
            0 => {
                self.handle_general_click(&items, content_x, content_y, content_w, content_start_y)
            }
            1 => self.handle_music_click(&items, content_x, content_y, content_w, content_start_y),
            2 => {
                if self.handle_widget_click()
                    && let Some(win) = &self.window
                {
                    win.request_redraw();
                }
            }
            3 => self.handle_about_click(&items, content_x, content_y, content_w, content_start_y),
            _ => {}
        }
    }

    fn handle_general_click(
        &mut self,
        items: &[SettingsItem],
        mx: f32,
        my: f32,
        width: f32,
        start_y: f32,
    ) {
        let scale = self
            .window
            .as_ref()
            .map(|w| w.scale_factor() as f32)
            .unwrap_or(1.0);
        let result = hit_test(items, mx, my, start_y, width);
        let mut changed = false;

        match result {
            ClickResult::StepperDec(idx) | ClickResult::StepperInc(idx) => {
                let is_dec = matches!(result, ClickResult::StepperDec(_));
                if let Some(item) = items.get(idx)
                    && let SettingsItem::RowStepper { label, .. } = item
                {
                    let l = label.clone();
                    if l == tr("global_scale") {
                        if is_dec {
                            self.config.global_scale =
                                ((self.config.global_scale - 0.05) * 100.0).round() / 100.0;
                            self.config.global_scale = self.config.global_scale.max(0.5);
                        } else {
                            self.config.global_scale =
                                ((self.config.global_scale + 0.05) * 100.0).round() / 100.0;
                            self.config.global_scale = self.config.global_scale.min(5.0);
                        }
                        changed = true;
                    } else if l == tr("base_width") {
                        if is_dec {
                            self.config.base_width =
                                (self.config.base_width - 5.0).clamp(40.0, 400.0);
                        } else {
                            self.config.base_width = (self.config.base_width + 5.0).min(400.0);
                        }
                        changed = true;
                    } else if l == tr("base_height") {
                        if is_dec {
                            self.config.base_height =
                                (self.config.base_height - 2.0).clamp(15.0, 200.0);
                        } else {
                            self.config.base_height = (self.config.base_height + 2.0).min(200.0);
                        }
                        changed = true;
                    } else if l == tr("expanded_width") {
                        if is_dec {
                            self.config.expanded_width =
                                (self.config.expanded_width - 10.0).clamp(200.0, 2000.0);
                        } else {
                            self.config.expanded_width =
                                (self.config.expanded_width + 10.0).min(2000.0);
                        }
                        changed = true;
                    } else if l == tr("expanded_height") {
                        if is_dec {
                            self.config.expanded_height =
                                (self.config.expanded_height - 10.0).clamp(100.0, 1000.0);
                        } else {
                            self.config.expanded_height =
                                (self.config.expanded_height + 10.0).min(1000.0);
                        }
                        changed = true;
                    } else if l == tr("position_x_offset") {
                        if is_dec {
                            self.config.position_x_offset -= 5;
                        } else {
                            self.config.position_x_offset += 5;
                        }
                        changed = true;
                    } else if l == tr("position_y_offset") {
                        if is_dec {
                            self.config.position_y_offset -= 5;
                        } else {
                            self.config.position_y_offset += 5;
                        }
                        changed = true;
                    } else if l == tr("font_size") {
                        if is_dec {
                            self.config.font_size = (self.config.font_size - 1.0).max(0.0);
                        } else {
                            self.config.font_size = (self.config.font_size + 1.0).min(30.0);
                        }
                        changed = true;
                    } else if l == tr("hide_delay") {
                        if is_dec {
                            self.config.auto_hide_delay =
                                (self.config.auto_hide_delay - 1.0).max(1.0);
                        } else {
                            self.config.auto_hide_delay =
                                (self.config.auto_hide_delay + 1.0).min(60.0);
                        }
                        changed = true;
                    } else if l == tr("update_interval") {
                        if is_dec {
                            self.config.update_check_interval =
                                (self.config.update_check_interval - 1.0).max(1.0);
                        } else {
                            self.config.update_check_interval =
                                (self.config.update_check_interval + 1.0).min(24.0);
                        }
                        changed = true;
                    }
                }
            }
            ClickResult::Switch(idx) => {
                let label = items
                    .iter()
                    .filter_map(|item| match item {
                        SettingsItem::RowSwitch { label, .. } => Some(label.clone()),
                        _ => None,
                    })
                    .nth(idx);
                if let Some(label) = label {
                    match label.as_str() {
                        l if l == tr("adaptive_border") => {
                            self.config.adaptive_border = !self.config.adaptive_border
                        }
                        l if l == tr("motion_blur") => {
                            self.config.motion_blur = !self.config.motion_blur
                        }
                        l if l == tr("cover_rotate") => {
                            self.config.cover_rotate = !self.config.cover_rotate
                        }

                        l if l == tr("start_boot") => {
                            self.config.auto_start = !self.config.auto_start;
                            let _ = set_autostart(self.config.auto_start);
                        }
                        l if l == tr("auto_hide") => self.config.auto_hide = !self.config.auto_hide,
                        l if l == tr("right_click_drag") => {
                            self.config.right_click_drag = !self.config.right_click_drag
                        }
                        l if l == tr("check_updates") => {
                            self.config.check_for_updates = !self.config.check_for_updates
                        }
                        _ => {
                            log::warn!("WinIsland: unhandled switch label: {}", label);
                        }
                    }
                }
                self.sync_switch_targets();
                changed = true;
            }
            ClickResult::FontSelect(_) => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Fonts", &["ttf", "otf"])
                    .pick_file()
                {
                    self.config.custom_font_path = Some(path.to_string_lossy().into_owned());
                    FontManager::global().refresh_custom_font();
                    changed = true;
                }
            }
            ClickResult::FontReset(_) => {
                self.config.custom_font_path = None;
                FontManager::global().refresh_custom_font();
                changed = true;
            }
            ClickResult::SourceButton(idx) => {
                let content_w = width;
                let mut btn_content_y = start_y;
                for item in items.iter().take(idx) {
                    btn_content_y += item.height();
                }
                let cy = btn_content_y + ROW_HEIGHT / 2.0;
                let btn_x = SIDEBAR_W + CONTENT_PADDING + content_w - GROUP_INNER_PAD - POPUP_BTN_W;
                let btn_y = cy - POPUP_BTN_H / 2.0 - self.scroll_y;

                if let Some(SettingsItem::RowSourceSelect { label, .. }) = items.get(idx) {
                    if label == &tr("monitor") {
                        let monitors = self.get_monitor_list();
                        let selected_idx = (self.config.monitor_index as usize)
                            .min(monitors.len().saturating_sub(1));
                        let values: Vec<String> =
                            (0..monitors.len()).map(|i| i.to_string()).collect();
                        self.popup = Some(PopupState::new(
                            PopupKind::Monitor,
                            Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            monitors,
                            values,
                            selected_idx,
                            self.win_w / scale,
                            self.win_h / scale,
                        ));
                    } else if label == &tr("island_style") {
                        let selected_idx = match self.config.island_style.as_str() {
                            "glass" => 1,
                            "mica" => 2,
                            "dynamic" => 3,
                            _ => 0,
                        };
                        self.popup = Some(PopupState::new(
                            PopupKind::IslandStyle,
                            Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            vec![
                                tr("style_default"),
                                tr("style_glass"),
                                tr("style_mica"),
                                tr("style_dynamic"),
                            ],
                            vec![
                                "default".to_string(),
                                "glass".to_string(),
                                "mica".to_string(),
                                "dynamic".to_string(),
                            ],
                            selected_idx,
                            self.win_w / scale,
                            self.win_h / scale,
                        ));
                    } else if label == &tr("dock_position") {
                        let dp = self.config.dock_position;
                        let selected_idx = match dp {
                            DockPosition::TopCenter => 0,
                            DockPosition::TopLeft => 1,
                            DockPosition::TopRight => 2,
                            DockPosition::BottomCenter => 3,
                            DockPosition::BottomLeft => 4,
                            DockPosition::BottomRight => 5,
                        };
                        self.popup = Some(PopupState::new(
                            PopupKind::DockPositionPopup,
                            Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            vec![
                                tr("dock_position_top_center"),
                                tr("dock_position_top_left"),
                                tr("dock_position_top_right"),
                                tr("dock_position_bottom_center"),
                                tr("dock_position_bottom_left"),
                                tr("dock_position_bottom_right"),
                            ],
                            vec![
                                "top_center".to_string(),
                                "top_left".to_string(),
                                "top_right".to_string(),
                                "bottom_center".to_string(),
                                "bottom_left".to_string(),
                                "bottom_right".to_string(),
                            ],
                            selected_idx,
                            self.win_w / scale,
                            self.win_h / scale,
                        ));
                    } else if label == &tr("settings_theme") {
                        let selected_idx = match self.config.settings_theme.as_str() {
                            "light" => 1,
                            "dark" => 2,
                            _ => 0,
                        };
                        self.popup = Some(PopupState::new(
                            PopupKind::SettingsTheme,
                            Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            vec![tr("theme_system"), tr("theme_light"), tr("theme_dark")],
                            vec![
                                "system".to_string(),
                                "light".to_string(),
                                "dark".to_string(),
                            ],
                            selected_idx,
                            self.win_w / scale,
                            self.win_h / scale,
                        ));
                    } else if label == &tr("update_channel") {
                        let selected_idx = match self.config.update_channel.as_str() {
                            "beta" => 1,
                            _ => 0,
                        };
                        self.popup = Some(PopupState::new(
                            PopupKind::UpdateChannel,
                            Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            vec![tr("channel_stable"), tr("channel_beta")],
                            vec!["stable".to_string(), "beta".to_string()],
                            selected_idx,
                            self.win_w / scale,
                            self.win_h / scale,
                        ));
                    } else {
                        let lang = current_lang();
                        let all_langs = available_langs();
                        let options: Vec<String> =
                            all_langs.iter().map(|l| l.name.clone()).collect();
                        let values: Vec<String> =
                            all_langs.iter().map(|l| l.code.clone()).collect();
                        let selected_idx =
                            all_langs.iter().position(|l| l.code == lang).unwrap_or(0);
                        self.popup = Some(PopupState::new(
                            PopupKind::Language,
                            Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                            options,
                            values,
                            selected_idx,
                            self.win_w / scale,
                            self.win_h / scale,
                        ));
                    }
                    self.anim.set_with_speed(POPUP_OPACITY_KEY, 1.0, 0.25);
                    if let Some(win) = &self.window {
                        win.request_redraw();
                    }
                }
            }
            ClickResult::CenterLink(_) => {
                self.config = AppConfig::default();
                init_i18n(&self.config.language);
                FontManager::global().refresh_custom_font();
                self.switch_anim = SwitchAnimator::new(&[
                    self.config.adaptive_border,
                    self.config.motion_blur,
                    self.config.cover_rotate,
                    self.config.auto_start,
                    self.config.auto_hide,
                    self.config.check_for_updates,
                    self.config.smtc_enabled,
                    self.config.show_lyrics,
                    self.config.lyrics_fallback,
                    self.config.lyrics_scroll,
                ]);
                changed = true;
            }
            ClickResult::RowButton(idx) => {
                if let Some(SettingsItem::RowButton { label, .. }) = items.get(idx)
                    && label == &tr("check_updates_manual")
                {
                    crate::utils::updater::check_updates_manually();
                }
            }
            _ => {}
        }

        if changed {
            self.mark_items_dirty();
            save_config(&self.config);
            if let Some(win) = &self.window {
                win.request_redraw();
            }
        }
    }

    fn handle_music_click(
        &mut self,
        items: &[SettingsItem],
        mx: f32,
        my: f32,
        width: f32,
        start_y: f32,
    ) {
        let scale = self
            .window
            .as_ref()
            .map(|w| w.scale_factor() as f32)
            .unwrap_or(1.0);
        let result = hit_test(items, mx, my, start_y, width);
        let mut changed = false;

        match result {
            ClickResult::Switch(idx) => {
                let label = items
                    .iter()
                    .filter_map(|item| match item {
                        SettingsItem::RowSwitch { label, .. } => Some(label.clone()),
                        _ => None,
                    })
                    .nth(idx);
                if let Some(label) = label {
                    match label.as_str() {
                        l if l == tr("smtc_control") => {
                            self.config.smtc_enabled = !self.config.smtc_enabled
                        }
                        l if l == tr("show_lyrics") => {
                            self.config.show_lyrics = !self.config.show_lyrics
                        }
                        l if l == tr("lyrics_fallback") => {
                            if self.config.show_lyrics {
                                self.config.lyrics_fallback = !self.config.lyrics_fallback
                            }
                        }
                        l if l == tr("lyrics_scroll") => {
                            if self.config.show_lyrics {
                                self.config.lyrics_scroll = !self.config.lyrics_scroll
                            }
                        }
                        _ => {
                            log::warn!("WinIsland: unhandled switch label: {}", label);
                        }
                    }
                }
                self.sync_switch_targets();
                changed = true;
            }
            ClickResult::SourceButton(idx) => {
                let content_w = width;
                let mut btn_content_y = start_y;
                for item in items.iter().take(idx) {
                    btn_content_y += item.height();
                }
                let cy = btn_content_y + ROW_HEIGHT / 2.0;
                let btn_x = SIDEBAR_W + CONTENT_PADDING + content_w - GROUP_INNER_PAD - POPUP_BTN_W;
                let btn_y = cy - POPUP_BTN_H / 2.0 - self.scroll_y;

                let source = &self.config.lyrics_source;
                self.popup = Some(PopupState::new(
                    PopupKind::LyricsSource,
                    Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                    vec!["163".to_string(), "LRCLIB".to_string()],
                    vec!["163".to_string(), "lrclib".to_string()],
                    if source == "163" { 0 } else { 1 },
                    self.win_w / scale,
                    self.win_h / scale,
                ));
                self.anim.set_with_speed(POPUP_OPACITY_KEY, 1.0, 0.25);
                if let Some(win) = &self.window {
                    win.request_redraw();
                }
            }
            ClickResult::FolderSelect(idx) => {
                if let Some(SettingsItem::RowFolderPicker { label, .. }) = items.get(idx)
                    && label == &tr("lyrics_local_dir")
                    && let Some(path) = rfd::FileDialog::new().pick_folder()
                {
                    self.config.lyrics_local_dir = Some(path.to_string_lossy().into_owned());
                    changed = true;
                }
            }
            ClickResult::FolderClear(idx) => {
                if let Some(SettingsItem::RowFolderPicker { label, .. }) = items.get(idx)
                    && label == &tr("lyrics_local_dir")
                {
                    self.config.lyrics_local_dir = None;
                    changed = true;
                }
            }
            ClickResult::StepperDec(idx) | ClickResult::StepperInc(idx) => {
                let is_dec = matches!(result, ClickResult::StepperDec(_));
                if let Some(item) = items.get(idx)
                    && let SettingsItem::RowStepper { label, .. } = item
                {
                    if label == &tr("lyrics_delay") && self.config.show_lyrics {
                        if is_dec {
                            self.config.lyrics_delay =
                                ((self.config.lyrics_delay * 10.0 - 1.0).round() / 10.0).max(-10.0);
                        } else {
                            self.config.lyrics_delay =
                                ((self.config.lyrics_delay * 10.0 + 1.0).round() / 10.0).min(10.0);
                        }
                        changed = true;
                    } else if label == &tr("lyrics_scroll_max_width")
                        && self.config.show_lyrics
                        && self.config.lyrics_scroll
                    {
                        if is_dec {
                            self.config.lyrics_scroll_max_width =
                                (self.config.lyrics_scroll_max_width - 10.0).max(100.0);
                        } else {
                            self.config.lyrics_scroll_max_width =
                                (self.config.lyrics_scroll_max_width + 10.0).min(500.0);
                        }
                        changed = true;
                    }
                }
            }
            ClickResult::AppItem(idx)
                if self.config.smtc_enabled && !self.detected_apps.is_empty() =>
            {
                let app_start = items
                    .iter()
                    .position(|i| matches!(i, SettingsItem::RowAppItem { .. }))
                    .unwrap_or(items.len());
                let app_idx = idx - app_start;
                if app_idx < self.detected_apps.len() {
                    let app = &self.detected_apps[app_idx];
                    if self.config.smtc_apps.contains(app) {
                        self.config.smtc_apps.retain(|a| a != app);
                    } else {
                        self.config.smtc_apps.push(app.clone());
                        if !self.config.smtc_known_apps.contains(app) {
                            self.config.smtc_known_apps.push(app.clone());
                        }
                    }
                    changed = true;
                }
            }
            _ => {}
        }

        if changed {
            self.mark_items_dirty();
            save_config(&self.config);
            if let Some(win) = &self.window {
                win.request_redraw();
            }
        }
    }

    fn handle_about_click(
        &mut self,
        items: &[SettingsItem],
        mx: f32,
        my: f32,
        width: f32,
        start_y: f32,
    ) {
        let result = hit_test(items, mx, my, start_y, width);
        if let ClickResult::CenterLink(_) = result {
            let _ = open::that(APP_HOMEPAGE);
        }
    }

    pub(super) fn get_hover_state(&mut self) -> bool {
        let (mx, my) = self.logical_mouse_pos;

        // Hover over Apple dots
        let is_on_red = (mx - 20.0).powi(2) + (my - 24.0).powi(2) <= 36.0;
        let is_on_yellow = (mx - 40.0).powi(2) + (my - 24.0).powi(2) <= 36.0;
        let is_on_green = (mx - 60.0).powi(2) + (my - 24.0).powi(2) <= 36.0;
        if is_on_red || is_on_yellow || is_on_green {
            return true;
        }

        if let Some(popup) = &self.popup {
            let menu = popup.menu_rect();
            if mx >= menu.left && mx <= menu.right && my >= menu.top && my <= menu.bottom {
                return true;
            }
        }

        if mx < SIDEBAR_W {
            let start_y = 60.0;
            for i in 0..3 {
                let row_y = start_y + i as f32 * (SIDEBAR_ROW_H + 2.0);
                if my >= row_y
                    && my <= row_y + SIDEBAR_ROW_H
                    && (SIDEBAR_PAD..=SIDEBAR_W - SIDEBAR_PAD).contains(&mx)
                {
                    return true;
                }
            }
            return false;
        }

        let scale = self
            .window
            .as_ref()
            .map(|w| w.scale_factor() as f32)
            .unwrap_or(1.0);
        let content_w = self.win_w / scale - SIDEBAR_W;

        if self.active_page == 0 && (SUB_TAB_START_Y..=SUB_TAB_START_Y + SUB_TAB_H).contains(&my) {
            return true;
        }

        if self.widget_dragging.is_some() {
            return true;
        }
        if let Some(hit) = self.widget_preview_hit_at_mouse()
            && hit != WidgetPreviewHit::None
        {
            return true;
        }

        let content_x = mx - SIDEBAR_W;
        let content_start_y = if self.active_page == 0 {
            SUB_TAB_START_Y + SUB_TAB_H + CONTENT_START_Y
        } else {
            50.0
        };
        let content_y = my + self.scroll_y;
        self.ensure_items_cache();
        hover_test(
            &self.cached_items,
            content_x,
            content_y,
            content_start_y,
            content_w,
        )
    }
}
