use crate::core::config::{WIDGET_GRID_SLOTS, WidgetSlot};
use crate::core::smtc::MediaInfo;
use crate::icons::arrows::draw_arrow_left;
use crate::ui::widget::{draw_widget, widget_animates, widget_grid_layout};
use skia_safe::{Canvas, Color, Paint};

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
    let mut animating = false;

    if alpha > 20 {
        let layout = widget_grid_layout(ox, oy, w, h, scale);

        for slot in 0..WIDGET_GRID_SLOTS {
            let Some(kind) = widget_layout
                .iter()
                .find(|entry| entry.slot == slot)
                .and_then(|entry| entry.widget)
            else {
                continue;
            };

            let (slot_x, slot_y, tile_w, tile_h) = layout.footprint_rect(kind, slot);

            draw_widget(
                canvas, kind, slot_x, slot_y, tile_w, tile_h, scale, alpha, text_color,
            );

            if widget_animates(kind) {
                animating = true;
            }
        }
    }

    if alpha > 0 {
        draw_arrow_left(
            canvas,
            ox + 12.0 * scale,
            oy + h / 2.0,
            alpha,
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

    animating
}
