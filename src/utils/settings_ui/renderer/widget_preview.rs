use skia_safe::{Canvas, Color, Paint, Rect};

use crate::core::config::{WidgetKind, WidgetSlot, widget_footprint};
use crate::core::i18n::tr;
use crate::ui::widget::{draw_mini_card, draw_widget, draw_widget_text_centered};
use crate::utils::color::SettingsTheme;
use crate::utils::font::FontManager;

use super::super::input::{
    WIDGET_ISLAND_PANEL_H, WIDGET_PREVIEW_H, widget_delete_button_center, widget_grid_geom,
    widget_library_items, widget_source_rect,
};
use super::super::items::{CONTENT_PADDING, GROUP_INNER_PAD, SettingsItem};

pub(super) struct WidgetPreviewParams<'a> {
    pub(super) canvas: &'a Canvas,
    pub(super) item_y: f32,
    pub(super) width: f32,
    pub(super) content_width: f32,
    pub(super) visible_min_y: f32,
    pub(super) visible_max_y: f32,
    pub(super) island_style: &'a str,
    pub(super) adaptive_border: bool,
    pub(super) expanded_width: f32,
    pub(super) expanded_height: f32,
    pub(super) widget_layout: &'a [WidgetSlot],
    pub(super) widget_dragging: Option<WidgetKind>,
    pub(super) widget_drag_hover_slot: Option<usize>,
    pub(super) widget_preview_hover_slot: Option<usize>,
    pub(super) theme: &'a SettingsTheme,
}

pub(super) fn draw_widget_preview(params: WidgetPreviewParams<'_>) {
    let WidgetPreviewParams {
        canvas,
        item_y: y,
        width,
        content_width: content_w,
        visible_min_y,
        visible_max_y,
        island_style,
        adaptive_border,
        expanded_width,
        expanded_height,
        widget_layout,
        widget_dragging,
        widget_drag_hover_slot,
        widget_preview_hover_slot,
        theme,
    } = params;
    let fm = FontManager::global();
    let preview_h = WIDGET_PREVIEW_H;
    let top_pad = (SettingsItem::WidgetPreview.height() - preview_h) / 2.0;
    let py = y + top_pad;
    let visible = py + preview_h >= visible_min_y && py <= visible_max_y;
    if visible {
        let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
        let preview_w = content_w - GROUP_INNER_PAD * 2.0;
        let preview_panel_h = WIDGET_ISLAND_PANEL_H;
        let library_panel_y = py + preview_panel_h + 12.0;
        let library_panel_h = preview_h - preview_panel_h - 12.0;

        let mut bg_p = Paint::default();
        bg_p.set_anti_alias(true);
        bg_p.set_color(theme.sidebar_bg);
        canvas.draw_round_rect(
            Rect::from_xywh(row_x, library_panel_y, preview_w, library_panel_h),
            12.0,
            12.0,
            &bg_p,
        );

        let mut label_p = Paint::default();
        label_p.set_anti_alias(true);
        let geom = widget_grid_geom(y, width, expanded_width, expanded_height);
        let cap_x = geom.cap_x;
        let cap_y = geom.cap_y;
        let cap_w = geom.cap_w;
        let cap_h = geom.cap_h;
        let cap_scale = geom.cap_scale;

        let mut cap_bg = Paint::default();
        cap_bg.set_anti_alias(true);

        if island_style == "glass" || island_style == "mica" {
            cap_bg.set_color(Color::from_argb(170, 24, 24, 28));
            canvas.draw_round_rect(
                Rect::from_xywh(cap_x, cap_y, cap_w, cap_h),
                28.0,
                28.0,
                &cap_bg,
            );
        } else if island_style == "dynamic" {
            let colors = [Color::from_rgb(18, 12, 36), Color::from_rgb(8, 24, 48)];
            #[allow(deprecated)]
            if let Some(shader) = skia_safe::gradient_shader::linear(
                (
                    skia_safe::Point::new(cap_x, cap_y),
                    skia_safe::Point::new(cap_x + cap_w, cap_y + cap_h),
                ),
                &colors[..],
                None,
                skia_safe::TileMode::Clamp,
                None,
                None,
            ) {
                cap_bg.set_shader(Some(shader));
            } else {
                cap_bg.set_color(Color::from_rgb(12, 12, 16));
            }
            canvas.draw_round_rect(
                Rect::from_xywh(cap_x, cap_y, cap_w, cap_h),
                28.0,
                28.0,
                &cap_bg,
            );
            cap_bg.set_shader(None);
        } else {
            cap_bg.set_color(Color::from_rgb(10, 10, 10));
            canvas.draw_round_rect(
                Rect::from_xywh(cap_x, cap_y, cap_w, cap_h),
                28.0,
                28.0,
                &cap_bg,
            );
        }

        let dragging = widget_dragging.is_some();
        let slot_radius = 12.0 * cap_scale;

        let drop_cells: Vec<usize> = match (widget_dragging, widget_drag_hover_slot) {
            (Some(widget), Some(slot)) => widget_footprint(widget, slot),
            _ => Vec::new(),
        };

        if dragging {
            for slot in drop_cells.iter().copied() {
                let (sx, sy, sw, sh) = geom.slot_rect(slot);
                let mut slot_p = Paint::default();
                slot_p.set_anti_alias(true);
                slot_p.set_color(Color::from_argb(
                    110,
                    theme.accent.r(),
                    theme.accent.g(),
                    theme.accent.b(),
                ));
                canvas.draw_round_rect(
                    Rect::from_xywh(sx, sy, sw, sh),
                    slot_radius,
                    slot_radius,
                    &slot_p,
                );
                let mut slot_border = Paint::default();
                slot_border.set_anti_alias(true);
                slot_border.set_style(skia_safe::paint::Style::Stroke);
                slot_border.set_stroke_width(2.0);
                slot_border.set_color(theme.accent);
                canvas.draw_round_rect(
                    Rect::from_xywh(sx, sy, sw, sh),
                    slot_radius,
                    slot_radius,
                    &slot_border,
                );
            }
        }

        for entry in widget_layout.iter() {
            let Some(kind) = entry.widget else { continue };
            if widget_dragging == Some(kind) {
                continue;
            }
            let (tx, ty, tw, th) = geom.footprint_rect(kind, entry.slot);

            draw_widget(canvas, kind, tx, ty, tw, th, cap_scale, 255, Color::WHITE);

            let hovered = widget_preview_hover_slot
                .map(|s| widget_footprint(kind, entry.slot).contains(&s))
                .unwrap_or(false);
            if kind != WidgetKind::Settings && (dragging || hovered) {
                let (bx, by) = widget_delete_button_center(tx, ty, tw, th);
                let mut xbg = Paint::default();
                xbg.set_anti_alias(true);
                xbg.set_color(Color::from_argb(235, 255, 59, 48));
                canvas.draw_circle((bx, by), 7.0 * cap_scale, &xbg);
                label_p.set_color(Color::WHITE);
                let diameter = 14.0 * cap_scale;
                draw_widget_text_centered(
                    canvas,
                    "×",
                    Rect::from_xywh(bx - diameter / 2.0, by - diameter / 2.0, diameter, diameter),
                    10.0 * cap_scale,
                    true,
                    &label_p,
                );
            }
        }

        let mut border_p = Paint::default();
        border_p.set_anti_alias(true);
        border_p.set_style(skia_safe::paint::Style::Stroke);
        border_p.set_stroke_width(1.0);
        if adaptive_border {
            border_p.set_color(Color::from_argb(120, 255, 255, 255));
        } else {
            border_p.set_color(Color::from_argb(40, 255, 255, 255));
        }
        canvas.draw_round_rect(
            Rect::from_xywh(cap_x, cap_y, cap_w, cap_h),
            28.0,
            28.0,
            &border_p,
        );

        label_p.set_color(theme.text_sec);
        fm.draw_text_with_default_font(
            canvas,
            &tr("tab_widgets"),
            (row_x + 14.0, library_panel_y + 20.0),
            11.0,
            false,
            &label_p,
        );

        let source_y = library_panel_y + 32.0;
        for (idx, kind) in widget_library_items(widget_layout, widget_dragging)
            .iter()
            .enumerate()
        {
            let (source_x, source_y, source_w, source_h) =
                widget_source_rect(row_x, source_y, idx, *kind);
            draw_mini_card(canvas, *kind, source_x, source_y, source_w, source_h);
        }
    }
}
