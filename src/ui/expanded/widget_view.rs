use crate::core::config::{WidgetKind, WidgetSlot};
use crate::core::i18n::tr;
use crate::core::smtc::MediaInfo;
use crate::icons::arrows::draw_arrow_left;
use crate::utils::font::{DrawTextInRectParams, FontManager};
use skia_safe::{Canvas, Color, Paint, Rect};

fn widget_title(kind: WidgetKind) -> String {
    match kind {
        WidgetKind::Clock => tr("widget_clock"),
        WidgetKind::Status => tr("widget_status"),
        WidgetKind::Weather => tr("widget_weather"),
    }
}

fn widget_value(kind: WidgetKind, media: &MediaInfo) -> String {
    match kind {
        WidgetKind::Clock => tr("widget_now"),
        WidgetKind::Status => {
            if media.is_playing {
                tr("widget_playing")
            } else {
                tr("widget_ready")
            }
        }
        WidgetKind::Weather => tr("widget_weather_empty"),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_widget_page(
    canvas: &Canvas,
    ox: f32,
    oy: f32,
    w: f32,
    h: f32,
    alpha: u8,
    scale: f32,
    _media: &MediaInfo,
    _font_size: f32,
    _lyrics_delay: f64,
    _dt: f32,
    widget_layout: &[WidgetSlot],
    text_color: Color,
) -> bool {
    let arrow_alpha = alpha;
    if arrow_alpha > 0 {
        draw_arrow_left(
            canvas,
            ox + 12.0 * scale,
            oy + h / 2.0,
            arrow_alpha,
            scale,
            text_color,
        );
    }

    if alpha > 30 {
        let gear_size = 12.0 * scale;
        let gear_x = ox + w - 28.0 * scale;
        let gear_y = oy + h - 28.0 * scale;
        let mut gear_paint = Paint::default();
        gear_paint.set_anti_alias(true);
        gear_paint.set_color(Color::from_argb(
            (alpha as f32 * 0.5) as u8,
            text_color.r(),
            text_color.g(),
            text_color.b(),
        ));
        gear_paint.set_style(skia_safe::paint::Style::Stroke);
        gear_paint.set_stroke_width(1.5 * scale);
        canvas.draw_circle((gear_x, gear_y), gear_size * 0.5, &gear_paint);
        let inner_r = gear_size * 0.18;
        canvas.draw_circle((gear_x, gear_y), inner_r, &gear_paint);
        let tooth_count = 8;
        let outer_r = gear_size * 0.5;
        for t in 0..tooth_count {
            let angle = (t as f32 / tooth_count as f32) * std::f32::consts::TAU;
            let x1 = gear_x + angle.cos() * (outer_r - 1.5 * scale);
            let y1 = gear_y + angle.sin() * (outer_r - 1.5 * scale);
            let x2 = gear_x + angle.cos() * (outer_r + 2.0 * scale);
            let y2 = gear_y + angle.sin() * (outer_r + 2.0 * scale);
            canvas.draw_line((x1, y1), (x2, y2), &gear_paint);
        }
    }

    if alpha > 20 {
        let fm = FontManager::global();
        let content_x = ox + 44.0 * scale;
        let content_w = (w - 88.0 * scale).max(80.0 * scale);
        let gap = 6.0 * scale;
        let cols = 3usize;
        let rows = 3usize;
        let slot_w = (content_w - gap * (cols as f32 - 1.0)) / cols as f32;
        let grid_h = (h * 0.78).min(h - 20.0 * scale).max(0.0);
        let slot_h = (grid_h - gap * (rows as f32 - 1.0)) / rows as f32;
        let grid_y = oy + (h - grid_h) / 2.0;

        for slot in 0..(cols * rows) {
            let widget = widget_layout
                .iter()
                .find(|entry| entry.slot == slot)
                .and_then(|entry| entry.widget);

            // Empty cells render nothing.
            let Some(kind) = widget else {
                continue;
            };

            let col = (slot % cols) as f32;
            let row = (slot / cols) as f32;
            let slot_x = content_x + col * (slot_w + gap);
            let slot_y = grid_y + row * (slot_h + gap);

            let mut bg = Paint::default();
            bg.set_anti_alias(true);
            bg.set_color(Color::from_argb((alpha as f32 * 0.14) as u8, 255, 255, 255));
            canvas.draw_round_rect(
                Rect::from_xywh(slot_x, slot_y, slot_w, slot_h),
                10.0 * scale,
                10.0 * scale,
                &bg,
            );

            let mut border = Paint::default();
            border.set_anti_alias(true);
            border.set_style(skia_safe::paint::Style::Stroke);
            border.set_stroke_width(1.0 * scale);
            border.set_color(Color::from_argb((alpha as f32 * 0.18) as u8, 255, 255, 255));
            canvas.draw_round_rect(
                Rect::from_xywh(slot_x, slot_y, slot_w, slot_h),
                10.0 * scale,
                10.0 * scale,
                &border,
            );

            let mut text_paint = Paint::default();
            text_paint.set_anti_alias(true);
            text_paint.set_color(Color::from_argb(
                alpha,
                text_color.r(),
                text_color.g(),
                text_color.b(),
            ));
            fm.draw_text_in_rect(DrawTextInRectParams {
                canvas,
                text: &widget_title(kind),
                x: slot_x + 5.0 * scale,
                y: slot_y + slot_h * 0.42,
                w: slot_w - 10.0 * scale,
                size: 9.5 * scale,
                bold: true,
                paint: &text_paint,
            });
            text_paint.set_color(Color::from_argb(
                (alpha as f32 * 0.7) as u8,
                text_color.r(),
                text_color.g(),
                text_color.b(),
            ));
            fm.draw_text_in_rect(DrawTextInRectParams {
                canvas,
                text: &widget_value(kind, _media),
                x: slot_x + 5.0 * scale,
                y: slot_y + slot_h * 0.72,
                w: slot_w - 10.0 * scale,
                size: 9.0 * scale,
                bold: false,
                paint: &text_paint,
            });
        }
    }

    false
}
