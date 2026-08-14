use crate::core::config::{
    WidgetKind, clear_plugin_widget, clear_widget_slot, place_builtin_widget, place_plugin_widget,
    plugin_widget_covering_slot, widget_covering_slot,
};
use crate::utils::settings_ui::items::SettingsItem;
use crate::utils::settings_ui::{
    WidgetPreviewHit, WidgetSource, widget_delete_button_hit, widget_grid_geom,
    widget_library_items, widget_preview_height, widget_preview_hit_test,
};

use super::super::{SETTINGS_HEADER_H, SIDEBAR_W, SettingsApp};

impl SettingsApp {
    pub(crate) fn build_widget_items(&self) -> Vec<SettingsItem> {
        let library_count = widget_library_items(
            &self.config.widget_layout,
            &self.config.plugin_widget_layout,
            &self.plugin_widgets,
            self.widget_dragging.as_ref(),
        )
        .len();
        vec![SettingsItem::WidgetPreview {
            height: widget_preview_height(library_count),
        }]
    }

    fn widget_preview_item_y(&mut self) -> Option<f32> {
        if self.active_page != 2 {
            return None;
        }
        self.ensure_items_cache();
        let mut y = SETTINGS_HEADER_H;
        for item in &self.cached_items {
            if matches!(item, SettingsItem::WidgetPreview { .. }) {
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
            &self.config.plugin_widget_layout,
            &self.plugin_widgets,
            self.widget_dragging.as_ref(),
        ))
    }

    pub(crate) fn handle_widget_drag_press(&mut self) -> bool {
        let Some(hit) = self.widget_preview_hit_at_mouse() else {
            return false;
        };
        let source = match hit {
            WidgetPreviewHit::Source(source) => source,
            WidgetPreviewHit::Slot(slot) => {
                let (source, anchor, span, removable) = if let Some((anchor, widget)) =
                    widget_covering_slot(&self.config.widget_layout, slot)
                {
                    (
                        WidgetSource::BuiltIn(widget),
                        anchor,
                        widget.span(),
                        widget != WidgetKind::Settings,
                    )
                } else if let Some((entry, widget)) = plugin_widget_covering_slot(
                    &self.config.plugin_widget_layout,
                    &self.plugin_widgets,
                    slot,
                ) {
                    (
                        WidgetSource::Plugin(entry.id()),
                        entry.slot,
                        widget.span(),
                        true,
                    )
                } else {
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
                let (x, y, width, height) = geometry.footprint_rect(span, anchor);
                let (mouse_x, mouse_y) = self.logical_mouse_pos;
                if removable
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
                source
            }
            WidgetPreviewHit::None => return false,
        };
        self.widget_dragging = Some(source);
        self.widget_drag_hover_slot = None;
        self.mark_items_dirty();
        true
    }

    pub(crate) fn handle_widget_drag_release(&mut self) -> bool {
        let Some(source) = self.widget_dragging.take() else {
            return false;
        };
        let old_widget_layout = self.config.widget_layout.clone();
        let old_plugin_layout = self.config.plugin_widget_layout.clone();
        if let Some(slot) = self.widget_drag_hover_slot.take() {
            match source {
                WidgetSource::BuiltIn(widget) => place_builtin_widget(
                    &mut self.config.widget_layout,
                    &mut self.config.plugin_widget_layout,
                    &self.plugin_widgets,
                    widget,
                    slot,
                ),
                WidgetSource::Plugin(id) => {
                    place_plugin_widget(
                        &mut self.config.widget_layout,
                        &mut self.config.plugin_widget_layout,
                        &self.plugin_widgets,
                        &id,
                        slot,
                    );
                }
            }
        }
        self.mark_items_dirty();
        if old_widget_layout != self.config.widget_layout
            || old_plugin_layout != self.config.plugin_widget_layout
        {
            crate::core::persistence::save_config(&self.config);
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
        let mouse_x = mouse_x - SIDEBAR_W;
        let mouse_y = mouse_y + self.scroll_y;

        let built_in_anchor = self.config.widget_layout.iter().find_map(|entry| {
            let widget = entry.widget?;
            if widget == WidgetKind::Settings {
                return None;
            }
            let (x, y, width, height) = geometry.footprint_rect(widget.span(), entry.slot);
            widget_delete_button_hit(mouse_x, mouse_y, x, y, width, height, geometry.cap_scale)
                .then_some(entry.slot)
        });
        if let Some(anchor) = built_in_anchor {
            clear_widget_slot(&mut self.config.widget_layout, anchor);
            crate::core::persistence::save_config(&self.config);
            self.mark_items_dirty();
            return true;
        }

        let plugin_id = self.config.plugin_widget_layout.iter().find_map(|entry| {
            let widget = self
                .plugin_widgets
                .iter()
                .find(|widget| widget.layout_id().as_ref() == Some(&entry.id()))?;
            let (x, y, width, height) = geometry.footprint_rect(widget.span(), entry.slot);
            widget_delete_button_hit(mouse_x, mouse_y, x, y, width, height, geometry.cap_scale)
                .then(|| entry.id())
        });
        let Some(plugin_id) = plugin_id else {
            return false;
        };
        clear_plugin_widget(&mut self.config.plugin_widget_layout, &plugin_id);
        crate::core::persistence::save_config(&self.config);
        self.mark_items_dirty();
        true
    }
}
