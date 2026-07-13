use crate::core::config::{
    WidgetKind, clear_widget_slot, place_widget_in_layout, widget_covering_slot,
};
use crate::core::i18n::tr;
use crate::core::persistence::save_config;
use crate::utils::settings_ui::items::SettingsItem;
use crate::utils::settings_ui::{
    WidgetPreviewHit, widget_delete_button_hit, widget_grid_geom, widget_preview_hit_test,
};

use super::super::{SIDEBAR_W, SettingsApp};

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
            .map(|window| window.scale_factor() as f32)
            .unwrap_or(1.0);
        let width = self.win_w / scale - SIDEBAR_W;
        let (mouse_x, mouse_y) = self.logical_mouse_pos;
        if mouse_x < SIDEBAR_W {
            return None;
        }
        Some(widget_preview_hit_test(
            mouse_x - SIDEBAR_W,
            mouse_y + self.scroll_y,
            item_y,
            width,
            self.config.expanded_width,
            self.config.expanded_height,
            &self.config.widget_layout,
            self.widget_dragging,
        ))
    }

    pub(crate) fn handle_widget_drag_press(&mut self) -> bool {
        let Some(hit) = self.widget_preview_hit_at_mouse() else {
            return false;
        };
        let widget = match hit {
            WidgetPreviewHit::Source(widget) => widget,
            WidgetPreviewHit::Slot(slot) => {
                let Some((anchor, widget)) = widget_covering_slot(&self.config.widget_layout, slot)
                else {
                    return false;
                };
                let Some(item_y) = self.widget_preview_item_y() else {
                    return false;
                };
                let scale = self
                    .window
                    .as_ref()
                    .map(|window| window.scale_factor() as f32)
                    .unwrap_or(1.0);
                let width = self.win_w / scale - SIDEBAR_W;
                let geometry = widget_grid_geom(
                    item_y,
                    width,
                    self.config.expanded_width,
                    self.config.expanded_height,
                );
                let (x, y, width, height) = geometry.footprint_rect(widget, anchor);
                let (mouse_x, mouse_y) = self.logical_mouse_pos;
                if widget != WidgetKind::Settings
                    && widget_delete_button_hit(
                        mouse_x - SIDEBAR_W,
                        mouse_y + self.scroll_y,
                        x,
                        y,
                        width,
                        height,
                        geometry.cap_scale,
                    )
                {
                    return false;
                }
                widget
            }
            WidgetPreviewHit::None => return false,
        };
        self.widget_dragging = Some(widget);
        self.widget_drag_hover_slot = None;
        true
    }

    pub(crate) fn handle_widget_drag_release(&mut self) -> bool {
        let Some(widget) = self.widget_dragging.take() else {
            return false;
        };
        if let Some(slot) = self.widget_drag_hover_slot.take() {
            place_widget_in_layout(&mut self.config.widget_layout, widget, slot);
            save_config(&self.config);
            self.mark_items_dirty();
        }
        true
    }

    pub(crate) fn handle_widget_click(&mut self) -> bool {
        let Some(item_y) = self.widget_preview_item_y() else {
            return false;
        };
        let scale = self
            .window
            .as_ref()
            .map(|window| window.scale_factor() as f32)
            .unwrap_or(1.0);
        let width = self.win_w / scale - SIDEBAR_W;
        let (mouse_x, mouse_y) = self.logical_mouse_pos;
        if mouse_x < SIDEBAR_W {
            return false;
        }
        let geometry = widget_grid_geom(
            item_y,
            width,
            self.config.expanded_width,
            self.config.expanded_height,
        );
        let anchor = self.config.widget_layout.iter().find_map(|entry| {
            let widget = entry.widget?;
            if widget == WidgetKind::Settings {
                return None;
            }
            let (x, y, width, height) = geometry.footprint_rect(widget, entry.slot);
            widget_delete_button_hit(
                mouse_x - SIDEBAR_W,
                mouse_y + self.scroll_y,
                x,
                y,
                width,
                height,
                geometry.cap_scale,
            )
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
}
