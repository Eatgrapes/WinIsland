use skia_safe::{Canvas, Color, FontStyle, Paint, Point, Rect};

use crate::core::config::{WIDGET_GRID_SLOTS, WidgetKind, WidgetSlot, widget_footprint};
use crate::core::i18n::tr;
use crate::ui::widget::{draw_mini_card, draw_widget_preview as draw_widget_card_preview};
use crate::utils::color::SettingsTheme;
use crate::utils::font::{DrawTextCachedParams, FontManager};
use crate::utils::shape::g3_rounded_rect_path;

use super::super::input::{
    WIDGET_ISLAND_PANEL_H, WIDGET_LIBRARY_HEADER_H, WIDGET_PANEL_GAP, WIDGET_PREVIEW_H,
    WidgetGridGeom, widget_delete_button_center, widget_grid_geom, widget_library_items,
    widget_source_rect,
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
    pub(super) expanded_width: f32,
    pub(super) expanded_height: f32,
    pub(super) widget_layout: &'a [WidgetSlot],
    pub(super) widget_dragging: Option<WidgetKind>,
    pub(super) widget_drag_hover_slot: Option<usize>,
    pub(super) widget_preview_hover_slot: Option<usize>,
    pub(super) theme: &'a SettingsTheme,
}

fn draw_panel(canvas: &Canvas, rect: Rect, theme: &SettingsTheme) {
    let mut shadow = Paint::default();
    shadow.set_anti_alias(true);
    shadow.set_color(theme.shadow);
    canvas.draw_round_rect(
        Rect::from_xywh(rect.left, rect.top + 2.0, rect.width(), rect.height()),
        14.0,
        14.0,
        &shadow,
    );

    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(theme.group_bg);
    canvas.draw_round_rect(rect, 14.0, 14.0, &paint);
    paint.set_style(skia_safe::paint::Style::Stroke);
    paint.set_stroke_width(0.75);
    paint.set_color(theme.group_border);
    canvas.draw_round_rect(
        Rect::from_xywh(
            rect.left + 0.375,
            rect.top + 0.375,
            rect.width() - 0.75,
            rect.height() - 0.75,
        ),
        14.0,
        14.0,
        &paint,
    );
}

fn draw_label(canvas: &Canvas, text: &str, x: f32, y: f32, size: f32, bold: bool, color: Color) {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(color);
    FontManager::global().draw_text_cached(DrawTextCachedParams {
        canvas,
        text,
        x,
        y,
        size,
        bold,
        paint: &paint,
    });
}

fn draw_centered_label(canvas: &Canvas, text: &str, rect: Rect, size: f32, color: Color) {
    let font_manager = FontManager::global();
    let text_width = font_manager.measure_text_cached(text, size, FontStyle::normal());
    draw_label(
        canvas,
        text,
        rect.center_x() - text_width / 2.0,
        rect.center_y() + size * 0.35,
        size,
        false,
        color,
    );
}

fn draw_island_background(canvas: &Canvas, rect: Rect, island_style: &str, theme: &SettingsTheme) {
    let mut shadow = Paint::default();
    shadow.set_anti_alias(true);
    shadow.set_color(Color::from_argb(72, 0, 0, 0));
    let shadow_path = g3_rounded_rect_path(
        Rect::from_xywh(rect.left, rect.top + 4.0, rect.width(), rect.height()),
        28.0,
    );
    canvas.draw_path(&shadow_path, &shadow);

    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    if island_style == "glass" || island_style == "mica" {
        paint.set_color(Color::from_argb(220, 24, 24, 28));
    } else if island_style == "dynamic" {
        let colors = [Color::from_rgb(18, 12, 36), Color::from_rgb(8, 24, 48)];
        #[allow(deprecated)]
        if let Some(shader) = skia_safe::gradient_shader::linear(
            (
                Point::new(rect.left, rect.top),
                Point::new(rect.right, rect.bottom),
            ),
            &colors[..],
            None,
            skia_safe::TileMode::Clamp,
            None,
            None,
        ) {
            paint.set_shader(Some(shader));
        } else {
            paint.set_color(Color::from_rgb(12, 12, 16));
        }
    } else {
        paint.set_color(Color::from_rgb(10, 10, 10));
    }
    let island_path = g3_rounded_rect_path(rect, 28.0);
    canvas.draw_path(&island_path, &paint);

    paint.set_shader(None);
    paint.set_style(skia_safe::paint::Style::Stroke);
    paint.set_stroke_width(1.0);
    paint.set_color(Color::from_argb(
        if island_style == "glass" || island_style == "mica" {
            52
        } else {
            38
        },
        theme.text_pri.r(),
        theme.text_pri.g(),
        theme.text_pri.b(),
    ));
    canvas.draw_path(&island_path, &paint);
}

fn draw_grid(
    canvas: &Canvas,
    geometry: &WidgetGridGeom,
    dragging: bool,
    drop_cells: &[usize],
    theme: &SettingsTheme,
) {
    let slot_radius = 12.0 * geometry.cap_scale;
    for slot in 0..WIDGET_GRID_SLOTS {
        let (x, y, width, height) = geometry.slot_rect(slot);
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_style(skia_safe::paint::Style::Stroke);
        paint.set_stroke_width(if dragging { 1.0 } else { 0.75 });
        paint.set_color(Color::from_argb(
            if dragging { 52 } else { 24 },
            255,
            255,
            255,
        ));
        canvas.draw_round_rect(
            Rect::from_xywh(x, y, width, height),
            slot_radius,
            slot_radius,
            &paint,
        );
    }

    for slot in drop_cells {
        let (x, y, width, height) = geometry.slot_rect(*slot);
        let rect = Rect::from_xywh(x, y, width, height);
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_color(Color::from_argb(
            100,
            theme.accent.r(),
            theme.accent.g(),
            theme.accent.b(),
        ));
        canvas.draw_round_rect(rect, slot_radius, slot_radius, &paint);
        paint.set_style(skia_safe::paint::Style::Stroke);
        paint.set_stroke_width(2.0);
        paint.set_color(theme.accent);
        canvas.draw_round_rect(rect, slot_radius, slot_radius, &paint);
    }
}

fn draw_delete_button(canvas: &Canvas, x: f32, y: f32, scale: f32) {
    let radius = (8.0 * scale).max(7.0);
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(Color::from_rgb(255, 59, 48));
    canvas.draw_circle((x, y), radius, &paint);

    paint.set_color(Color::WHITE);
    paint.set_style(skia_safe::paint::Style::Stroke);
    paint.set_stroke_width((1.5 * scale).max(1.25));
    paint.set_stroke_cap(skia_safe::paint::Cap::Round);
    let arm = (3.0 * scale).max(2.5);
    canvas.draw_line((x - arm, y - arm), (x + arm, y + arm), &paint);
    canvas.draw_line((x + arm, y - arm), (x - arm, y + arm), &paint);
}

fn draw_library_tile(canvas: &Canvas, kind: WidgetKind, rect: Rect) {
    let preview_rect = Rect::from_xywh(
        rect.left + 7.0,
        rect.top + 6.0,
        rect.width() - 14.0,
        rect.height() - 12.0,
    );
    let (preview_width, preview_height) = match kind {
        WidgetKind::Clock => (98.0, 46.0),
        WidgetKind::Calendar => (60.0, 60.0),
        WidgetKind::ResourceUsage => (98.0, 46.0),
        WidgetKind::Settings => (54.0, 54.0),
    };
    draw_mini_card(
        canvas,
        kind,
        preview_rect.center_x() - preview_width / 2.0,
        preview_rect.center_y() - preview_height / 2.0,
        preview_width,
        preview_height,
    );
}

pub(super) fn draw_widget_preview(params: WidgetPreviewParams<'_>) {
    let WidgetPreviewParams {
        canvas,
        item_y,
        width,
        content_width,
        visible_min_y,
        visible_max_y,
        island_style,
        expanded_width,
        expanded_height,
        widget_layout,
        widget_dragging,
        widget_drag_hover_slot,
        widget_preview_hover_slot,
        theme,
    } = params;
    let preview_height = WIDGET_PREVIEW_H;
    let top_padding = (SettingsItem::WidgetPreview.height() - preview_height) / 2.0;
    let y = item_y + top_padding;
    if y + preview_height < visible_min_y || y > visible_max_y {
        return;
    }

    let panel_x = CONTENT_PADDING + GROUP_INNER_PAD;
    let panel_width = content_width - GROUP_INNER_PAD * 2.0;
    let library_y = y + WIDGET_ISLAND_PANEL_H + WIDGET_PANEL_GAP;
    let library_height = preview_height - WIDGET_ISLAND_PANEL_H - WIDGET_PANEL_GAP;
    draw_panel(
        canvas,
        Rect::from_xywh(panel_x, y, panel_width, WIDGET_ISLAND_PANEL_H),
        theme,
    );
    draw_panel(
        canvas,
        Rect::from_xywh(panel_x, library_y, panel_width, library_height),
        theme,
    );

    draw_label(
        canvas,
        &tr("widget_layout_title"),
        panel_x + 16.0,
        y + 25.0,
        13.0,
        true,
        theme.text_pri,
    );
    draw_label(
        canvas,
        &tr("widget_layout_hint"),
        panel_x + 16.0,
        y + 44.0,
        11.0,
        false,
        theme.text_sec,
    );

    let geometry = widget_grid_geom(item_y, width, expanded_width, expanded_height);
    let island_rect = Rect::from_xywh(
        geometry.cap_x,
        geometry.cap_y,
        geometry.cap_w,
        geometry.cap_h,
    );
    draw_island_background(canvas, island_rect, island_style, theme);

    let dragging = widget_dragging.is_some();
    let drop_cells = match (widget_dragging, widget_drag_hover_slot) {
        (Some(widget), Some(slot)) => widget_footprint(widget, slot),
        _ => Vec::new(),
    };
    draw_grid(canvas, &geometry, dragging, &drop_cells, theme);

    for entry in widget_layout {
        let Some(kind) = entry.widget else { continue };
        if widget_dragging == Some(kind) {
            continue;
        }
        let (x, y, width, height) = geometry.footprint_rect(kind, entry.slot);
        draw_widget_card_preview(
            canvas,
            kind,
            x,
            y,
            width,
            height,
            geometry.cap_scale,
            255,
            Color::WHITE,
        );

        let hovered = widget_preview_hover_slot
            .is_some_and(|slot| widget_footprint(kind, entry.slot).contains(&slot));
        if kind != WidgetKind::Settings && (dragging || hovered) {
            let (button_x, button_y) =
                widget_delete_button_center(x, y, width, height, geometry.cap_scale);
            draw_delete_button(canvas, button_x, button_y, geometry.cap_scale);
        }
    }

    draw_label(
        canvas,
        &tr("widget_library_title"),
        panel_x + 16.0,
        library_y + 25.0,
        13.0,
        true,
        theme.text_pri,
    );
    draw_label(
        canvas,
        &tr("widget_library_hint"),
        panel_x + 16.0,
        library_y + 43.0,
        11.0,
        false,
        theme.text_sec,
    );

    let source_y = library_y + WIDGET_LIBRARY_HEADER_H;
    let library_items = widget_library_items(widget_layout, widget_dragging);
    if library_items.is_empty() {
        if widget_dragging.is_none() {
            draw_centered_label(
                canvas,
                &tr("widget_library_empty"),
                Rect::from_xywh(
                    panel_x + 12.0,
                    source_y,
                    panel_width - 24.0,
                    library_height - WIDGET_LIBRARY_HEADER_H,
                ),
                12.0,
                theme.text_sec,
            );
        }
    } else {
        for (index, kind) in library_items.iter().enumerate() {
            let (x, y, width, height) = widget_source_rect(panel_x, source_y, index, *kind);
            let rect = Rect::from_xywh(x, y, width, height);
            draw_library_tile(canvas, *kind, rect);
        }
    }
}
