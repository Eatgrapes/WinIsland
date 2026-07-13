pub mod calendar;
pub mod settings;
pub mod time;

use crate::core::config::{WIDGET_GRID_COLS, WIDGET_GRID_ROWS, WidgetKind, widget_footprint};
use crate::utils::font::FontManager;
use skia_safe::{Canvas, Color, Paint, Rect};

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

    pub fn slot_at_point(&self, x: f32, y: f32, include_gaps: bool) -> Option<usize> {
        if include_gaps {
            let grid_w =
                self.slot_w * WIDGET_GRID_COLS as f32 + self.gap * (WIDGET_GRID_COLS as f32 - 1.0);
            let grid_h =
                self.slot_h * WIDGET_GRID_ROWS as f32 + self.gap * (WIDGET_GRID_ROWS as f32 - 1.0);
            if x < self.grid_x
                || x > self.grid_x + grid_w
                || y < self.grid_y
                || y > self.grid_y + grid_h
            {
                return None;
            }
            let col =
                ((x - self.grid_x + self.gap / 2.0) / (self.slot_w + self.gap)).floor() as usize;
            let row =
                ((y - self.grid_y + self.gap / 2.0) / (self.slot_h + self.gap)).floor() as usize;
            return Some(
                row.min(WIDGET_GRID_ROWS - 1) * WIDGET_GRID_COLS + col.min(WIDGET_GRID_COLS - 1),
            );
        }

        (0..WIDGET_GRID_COLS * WIDGET_GRID_ROWS).find(|slot| {
            let (sx, sy, sw, sh) = self.slot_rect(*slot);
            x >= sx && x <= sx + sw && y >= sy && y <= sy + sh
        })
    }
}

pub fn widget_grid_layout(x: f32, y: f32, w: f32, h: f32, scale: f32) -> WidgetGridLayout {
    let inset = 12.0 * scale;
    let gap = 7.0 * scale;
    let inner_w = (w - inset * 2.0).max(0.0);
    let inner_h = (h - inset * 2.0).max(0.0);
    let slot_from_width =
        (inner_w - gap * (WIDGET_GRID_COLS as f32 - 1.0)) / WIDGET_GRID_COLS as f32;
    let slot_from_height =
        (inner_h - gap * (WIDGET_GRID_ROWS as f32 - 1.0)) / WIDGET_GRID_ROWS as f32;
    let slot_size = slot_from_width.min(slot_from_height).max(0.0);
    let slot_w = slot_size;
    let slot_h = slot_size;
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

pub(crate) fn draw_widget_rounded_background(
    canvas: &Canvas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    scale: f32,
    alpha: u8,
) {
    let mut background = Paint::default();
    background.set_anti_alias(true);
    background.set_color(Color::from_argb((alpha as f32 * 0.05) as u8, 28, 28, 30));
    let rect = Rect::from_xywh(x, y, w, h);
    let radius = widget_corner_radius(w, h);
    canvas.draw_round_rect(rect, radius, radius, &background);

    let mut border = Paint::default();
    border.set_anti_alias(true);
    border.set_style(skia_safe::paint::Style::Stroke);
    border.set_stroke_width(1.0 * scale);
    border.set_color(Color::from_argb((alpha as f32 * 0.16) as u8, 255, 255, 255));
    canvas.draw_round_rect(rect, radius, radius, &border);
}

pub(crate) fn widget_corner_radius(w: f32, h: f32) -> f32 {
    let radius_ratio = if w >= h * 1.5 { 0.28 } else { 0.20 };
    w.min(h) * radius_ratio
}

pub(crate) fn draw_widget_text_centered(
    canvas: &Canvas,
    text: &str,
    bounds: Rect,
    size: f32,
    bold: bool,
    paint: &Paint,
) {
    let font = FontManager::global().get_font(size, bold);
    let (_, glyph_bounds) = font.measure_str(text, None);
    let text_x =
        bounds.left() + (bounds.width() - glyph_bounds.width()) / 2.0 - glyph_bounds.left();
    let baseline_y =
        bounds.top() + (bounds.height() - glyph_bounds.height()) / 2.0 - glyph_bounds.top();
    canvas.draw_str(text, (text_x, baseline_y), &font, paint);
}

pub fn widget_animates(kind: WidgetKind) -> bool {
    matches!(kind, WidgetKind::Clock | WidgetKind::Calendar)
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
        WidgetKind::Calendar => {
            calendar::draw_calendar_widget(canvas, x, y, w, h, scale, alpha, text_color)
        }
        WidgetKind::Settings => {
            settings::draw_settings_widget(canvas, x, y, w, h, scale, alpha, text_color)
        }
    }
}

pub fn draw_mini_card(canvas: &Canvas, kind: WidgetKind, x: f32, y: f32, w: f32, h: f32) {
    draw_widget(canvas, kind, x, y, w, h, 1.0, 255, Color::WHITE);
}
