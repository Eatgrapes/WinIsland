use super::{draw_widget_rounded_background, draw_widget_text_centered};
use crate::core::i18n::tr;
use skia_safe::{Canvas, Color, Paint, Rect};

fn weekday_name(day: u16) -> String {
    match day {
        0 => tr("widget_calendar_sun"),
        1 => tr("widget_calendar_mon"),
        2 => tr("widget_calendar_tue"),
        3 => tr("widget_calendar_wed"),
        4 => tr("widget_calendar_thu"),
        5 => tr("widget_calendar_fri"),
        _ => tr("widget_calendar_sat"),
    }
}

fn month_name(month: u16) -> String {
    match month {
        1 => tr("widget_calendar_month_1"),
        2 => tr("widget_calendar_month_2"),
        3 => tr("widget_calendar_month_3"),
        4 => tr("widget_calendar_month_4"),
        5 => tr("widget_calendar_month_5"),
        6 => tr("widget_calendar_month_6"),
        7 => tr("widget_calendar_month_7"),
        8 => tr("widget_calendar_month_8"),
        9 => tr("widget_calendar_month_9"),
        10 => tr("widget_calendar_month_10"),
        11 => tr("widget_calendar_month_11"),
        _ => tr("widget_calendar_month_12"),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_calendar_widget(
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
    let month = month_name(local_time.wMonth);
    let day = local_time.wDay.to_string();
    let weekday = weekday_name(local_time.wDayOfWeek);

    draw_widget_rounded_background(canvas, x, y, w, h, scale, alpha);

    let mut month_paint = Paint::default();
    month_paint.set_anti_alias(true);
    month_paint.set_color(Color::from_argb(
        (alpha as f32 * 0.78) as u8,
        text_color.r(),
        text_color.g(),
        text_color.b(),
    ));
    draw_widget_text_centered(
        canvas,
        &month,
        Rect::from_xywh(x, y + h * 0.08, w, h * 0.18),
        (h * 0.14).clamp(9.0 * scale, 15.0 * scale),
        true,
        &month_paint,
    );

    let day_size = (h * 0.53).min(w * 0.70).max(26.0 * scale);
    let mut day_paint = Paint::default();
    day_paint.set_anti_alias(true);
    day_paint.set_color(Color::from_argb(
        alpha,
        text_color.r(),
        text_color.g(),
        text_color.b(),
    ));
    draw_widget_text_centered(
        canvas,
        &day,
        Rect::from_xywh(x, y + h * 0.27, w, h * 0.48),
        day_size,
        true,
        &day_paint,
    );

    let mut weekday_paint = Paint::default();
    weekday_paint.set_anti_alias(true);
    weekday_paint.set_color(Color::from_argb(
        (alpha as f32 * 0.62) as u8,
        text_color.r(),
        text_color.g(),
        text_color.b(),
    ));
    draw_widget_text_centered(
        canvas,
        &weekday,
        Rect::from_xywh(x, y + h * 0.78, w, h * 0.14),
        (h * 0.12).clamp(8.0 * scale, 12.0 * scale),
        false,
        &weekday_paint,
    );
}
