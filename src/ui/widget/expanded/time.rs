use super::{draw_widget_rounded_background, draw_widget_text_centered};
use skia_safe::{Canvas, Color, Paint, Rect};
use std::cell::RefCell;

thread_local! {
    static TIME_TEXT: RefCell<(u16, u16, String)> = const {
        RefCell::new((u16::MAX, u16::MAX, String::new()))
    };
}

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
    // SAFETY: GetLocalTime writes a SYSTEMTIME value and has no preconditions.
    let local_time = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };

    draw_widget_rounded_background(canvas, x, y, w, h, scale, alpha);

    let size = (h * 0.60).min(w * 0.31).max(13.0 * scale);

    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(Color::from_argb(
        alpha,
        text_color.r(),
        text_color.g(),
        text_color.b(),
    ));

    TIME_TEXT.with(|cell| {
        let mut cache = cell.borrow_mut();
        if cache.0 != local_time.wHour || cache.1 != local_time.wMinute {
            cache.0 = local_time.wHour;
            cache.1 = local_time.wMinute;
            cache.2 = format!("{:02}:{:02}", local_time.wHour, local_time.wMinute);
        }
        draw_widget_text_centered(
            canvas,
            &cache.2,
            Rect::from_xywh(x, y, w, h),
            size,
            true,
            &paint,
        );
    });
}
