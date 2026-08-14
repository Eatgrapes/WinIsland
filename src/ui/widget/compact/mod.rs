mod time;

use crate::core::config::CompactWidgetKind;
use skia_safe::{Canvas, Rect};

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
