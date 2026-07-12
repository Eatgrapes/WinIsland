use crate::utils::font::{DrawTextInRectParams, FontManager};
use skia_safe::{Canvas, Color, Paint, Rect};

#[allow(clippy::too_many_arguments)]
pub fn draw_time_widget(
    canvas: &Canvas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    scale: f32,
    alpha: u8,
    text_color: Color,
) {
    let fm = FontManager::global();
    // SAFETY: GetLocalTime writes a SYSTEMTIME value and has no preconditions.
    let local_time = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    let time = format!("{:02}:{:02}", local_time.wHour, local_time.wMinute);

    let mut background = Paint::default();
    background.set_anti_alias(true);
    background.set_color(Color::from_argb((alpha as f32 * 0.94) as u8, 28, 28, 30));
    canvas.draw_round_rect(Rect::from_xywh(x, y, w, h), h * 0.5, h * 0.5, &background);

    let size = (h * 0.60).min(w * 0.31).max(13.0 * scale);

    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(Color::from_argb(
        alpha,
        text_color.r(),
        text_color.g(),
        text_color.b(),
    ));

    fm.draw_text_in_rect(DrawTextInRectParams {
        canvas,
        text: &time,
        x,
        y: y + h * 0.5 + size * 0.35,
        w,
        size,
        bold: true,
        paint: &paint,
    });
}
