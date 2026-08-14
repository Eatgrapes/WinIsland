mod time;

use crate::core::config::{COMPACT_WIDGET_SLOTS, CompactWidgetKind, CompactWidgetSlot};
use skia_safe::{Canvas, Color, Paint, Rect};

const CONTENT_EDGE_INSET: f32 = 9.0;
const CONTENT_GAP: f32 = 7.0;
const LEFT_SLOT: usize = 0;
const CENTER_SLOT: usize = 1;
const RIGHT_SLOT: usize = 2;

fn widget_width(widget: CompactWidgetKind) -> f32 {
    match widget {
        CompactWidgetKind::Time => 48.0,
    }
}

fn configured_widget(layout: &[CompactWidgetSlot], slot: usize) -> Option<CompactWidgetKind> {
    layout
        .iter()
        .find(|entry| entry.slot == slot)
        .and_then(|entry| entry.widget)
}

fn center_relocation_slot(layout: &[CompactWidgetSlot]) -> usize {
    if configured_widget(layout, RIGHT_SLOT).is_none() {
        RIGHT_SLOT
    } else if configured_widget(layout, LEFT_SLOT).is_none() {
        LEFT_SLOT
    } else {
        CENTER_SLOT
    }
}

fn slot_widget(
    layout: &[CompactWidgetSlot],
    slot: usize,
    move_center_aside: bool,
) -> Option<CompactWidgetKind> {
    let center_widget = configured_widget(layout, CENTER_SLOT);
    if !move_center_aside || center_widget.is_none() {
        return configured_widget(layout, slot);
    }
    let relocation_slot = center_relocation_slot(layout);
    if relocation_slot == CENTER_SLOT {
        return configured_widget(layout, slot);
    }
    if slot == CENTER_SLOT {
        None
    } else if slot == relocation_slot {
        center_widget
    } else {
        configured_widget(layout, slot)
    }
}

pub(crate) fn has_center_widget(layout: &[CompactWidgetSlot], move_center_aside: bool) -> bool {
    slot_widget(layout, CENTER_SLOT, move_center_aside).is_some()
}

pub(crate) fn side_extensions(
    layout: &[CompactWidgetSlot],
    has_center_content: bool,
    move_center_aside: bool,
) -> (f32, f32) {
    if !has_center_content {
        return (0.0, 0.0);
    }
    let extension = |slot| {
        slot_widget(layout, slot, move_center_aside)
            .map(|widget| CONTENT_EDGE_INSET + widget_width(widget) + CONTENT_GAP)
            .unwrap_or(0.0)
    };
    (extension(LEFT_SLOT), extension(RIGHT_SLOT))
}

fn minimum_layout_width(layout: &[CompactWidgetSlot], move_center_aside: bool) -> f32 {
    let left_width = slot_widget(layout, LEFT_SLOT, move_center_aside).map(widget_width);
    let center_width = slot_widget(layout, CENTER_SLOT, move_center_aside).map(widget_width);
    let right_width = slot_widget(layout, RIGHT_SLOT, move_center_aside).map(widget_width);

    if let Some(center_width) = center_width {
        let side_width = left_width.unwrap_or(0.0).max(right_width.unwrap_or(0.0));
        return center_width + 2.0 * (CONTENT_EDGE_INSET + side_width + CONTENT_GAP);
    }

    match (left_width, right_width) {
        (Some(left), Some(right)) => CONTENT_EDGE_INSET * 2.0 + left + CONTENT_GAP + right,
        (Some(width), None) | (None, Some(width)) => CONTENT_EDGE_INSET * 2.0 + width,
        (None, None) => 0.0,
    }
}

pub(crate) fn target_width(
    layout: &[CompactWidgetSlot],
    base_width: f32,
    center_content_width: Option<f32>,
) -> f32 {
    let move_center_aside = center_content_width.is_some();
    let center_occupied = has_center_widget(layout, move_center_aside);
    let has_center_content = center_occupied || center_content_width.is_some();
    let center_width = center_content_width.unwrap_or(base_width);
    let (left_extension, right_extension) =
        side_extensions(layout, has_center_content, move_center_aside);
    (center_width + left_extension + right_extension)
        .max(base_width)
        .max(minimum_layout_width(layout, move_center_aside))
}

pub(crate) fn draw(
    canvas: &Canvas,
    layout: &[CompactWidgetSlot],
    rect: Rect,
    scale: f32,
    alpha: u8,
    has_mini_content: bool,
) {
    let center_occupied = has_center_widget(layout, has_mini_content);
    let has_center_content = center_occupied || has_mini_content;
    for slot in 0..COMPACT_WIDGET_SLOTS {
        let Some(widget) = slot_widget(layout, slot, has_mini_content) else {
            continue;
        };
        let width = widget_width(widget) * scale;
        let x = match slot {
            LEFT_SLOT => rect.left + CONTENT_EDGE_INSET * scale,
            CENTER_SLOT => rect.center_x() - width / 2.0,
            RIGHT_SLOT => rect.right - CONTENT_EDGE_INSET * scale - width,
            _ => continue,
        };
        draw_widget(
            canvas,
            widget,
            Rect::from_xywh(x, rect.top, width, rect.height()),
            scale,
            alpha,
        );
    }

    if has_center_content {
        draw_separators(canvas, layout, rect, scale, alpha, has_mini_content);
    }
}

fn draw_separators(
    canvas: &Canvas,
    layout: &[CompactWidgetSlot],
    rect: Rect,
    scale: f32,
    alpha: u8,
    move_center_aside: bool,
) {
    let (left_extension, right_extension) = side_extensions(layout, true, move_center_aside);
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_stroke_width(scale.max(1.0));
    paint.set_color(Color::from_argb((alpha as f32 * 0.16) as u8, 255, 255, 255));
    let top = rect.top + rect.height() * 0.25;
    let bottom = rect.bottom - rect.height() * 0.25;

    if left_extension > 0.0 {
        let x = rect.left + (left_extension - CONTENT_GAP / 2.0) * scale;
        canvas.draw_line((x, top), (x, bottom), &paint);
    }
    if right_extension > 0.0 {
        let x = rect.right - (right_extension - CONTENT_GAP / 2.0) * scale;
        canvas.draw_line((x, top), (x, bottom), &paint);
    }
}

pub(crate) fn draw_widget(
    canvas: &Canvas,
    widget: CompactWidgetKind,
    rect: Rect,
    scale: f32,
    alpha: u8,
) {
    match widget {
        CompactWidgetKind::Time => time::draw(canvas, rect, scale, alpha),
    }
}

pub(crate) fn next_refresh_delay(layout: &[CompactWidgetSlot]) -> Option<std::time::Duration> {
    layout
        .iter()
        .any(|entry| entry.widget == Some(CompactWidgetKind::Time))
        .then(crate::ui::widget::time_text::until_next_minute)
}
