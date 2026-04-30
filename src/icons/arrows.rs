use skia_safe::{Canvas, Color, Paint, Rect};
pub fn draw_arrow_right(canvas: &Canvas, cx: f32, cy: f32, alpha: u8, scale: f32) {
    let mut paint = Paint::default();
    paint.set_color(Color::from_argb((alpha as f32 * 0.4) as u8, 255, 255, 255));
    paint.set_anti_alias(true);
    let w = 4.0 * scale;
    let h = 20.0 * scale;
    let rect = Rect::from_xywh(cx - w / 2.0, cy - h / 2.0, w, h);
    canvas.draw_round_rect(rect, 2.0 * scale, 2.0 * scale, &paint);
}

pub fn draw_arrow_left(canvas: &Canvas, cx: f32, cy: f32, alpha: u8, scale: f32) {
    let mut paint = Paint::default();
    paint.set_color(Color::from_argb((alpha as f32 * 0.4) as u8, 255, 255, 255));
    paint.set_anti_alias(true);
    let w = 4.0 * scale;
    let h = 20.0 * scale;
    let rect = Rect::from_xywh(cx - w / 2.0, cy - h / 2.0, w, h);
    canvas.draw_round_rect(rect, 2.0 * scale, 2.0 * scale, &paint);
}
