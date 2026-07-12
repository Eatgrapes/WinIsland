pub mod time;

use crate::core::config::{WIDGET_GRID_COLS, WIDGET_GRID_ROWS, WidgetKind, widget_footprint};
use skia_safe::{Canvas, Color};

#[derive(Debug, Clone, Copy)]
pub struct WidgetGridLayout {
    grid_x: f32,
    grid_y: f32,
    slot_w: f32,
    slot_h: f32,
    gap: f32,
}

impl WidgetGridLayout {
    pub fn slot_rect(&self, slot: usize) -> (f32, f32, f32, f32) {
        let col = (slot % WIDGET_GRID_COLS) as f32;
        let row = (slot / WIDGET_GRID_COLS) as f32;
        let x = self.grid_x + col * (self.slot_w + self.gap);
        let y = self.grid_y + row * (self.slot_h + self.gap);
        (x, y, self.slot_w, self.slot_h)
    }

    pub fn footprint_rect(&self, widget: WidgetKind, slot: usize) -> (f32, f32, f32, f32) {
        let (cols, rows) = widget.span();
        let anchor = widget_footprint(widget, slot)[0];
        let (x, y, _, _) = self.slot_rect(anchor);
        let w = self.slot_w * cols as f32 + self.gap * (cols as f32 - 1.0);
        let h = self.slot_h * rows as f32 + self.gap * (rows as f32 - 1.0);
        (x, y, w, h)
    }
}

pub fn widget_grid_layout(x: f32, y: f32, w: f32, h: f32, scale: f32) -> WidgetGridLayout {
    let inset = 12.0 * scale;
    let gap = 7.0 * scale;
    let inner_w = (w - inset * 2.0).max(0.0);
    let inner_h = (h - inset * 2.0).max(0.0);
    let slot_w = (inner_w - gap * (WIDGET_GRID_COLS as f32 - 1.0)) / WIDGET_GRID_COLS as f32;
    let slot_h = (inner_h - gap * (WIDGET_GRID_ROWS as f32 - 1.0)) / WIDGET_GRID_ROWS as f32;
    let grid_w = slot_w * WIDGET_GRID_COLS as f32 + gap * (WIDGET_GRID_COLS as f32 - 1.0);
    let grid_h = slot_h * WIDGET_GRID_ROWS as f32 + gap * (WIDGET_GRID_ROWS as f32 - 1.0);

    WidgetGridLayout {
        grid_x: x + (w - grid_w) / 2.0,
        grid_y: y + (h - grid_h) / 2.0,
        slot_w,
        slot_h,
        gap,
    }
}

pub fn widget_animates(kind: WidgetKind) -> bool {
    matches!(kind, WidgetKind::Clock)
}

#[allow(clippy::too_many_arguments)]
pub fn draw_widget(
    canvas: &Canvas,
    kind: WidgetKind,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    scale: f32,
    alpha: u8,
    text_color: Color,
) {
    match kind {
        WidgetKind::Clock => time::draw_time_widget(canvas, x, y, w, h, scale, alpha, text_color),
    }
}

pub fn draw_mini_card(canvas: &Canvas, kind: WidgetKind, x: f32, y: f32, w: f32, h: f32) {
    draw_widget(canvas, kind, x, y, w, h, 1.0, 255, Color::WHITE);
}
