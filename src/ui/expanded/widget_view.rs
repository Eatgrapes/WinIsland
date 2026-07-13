use crate::core::config::{WIDGET_GRID_SLOTS, WidgetSlot};
use crate::core::smtc::MediaInfo;
use crate::icons::arrows::draw_arrow_left;
use crate::ui::widget::{draw_widget, widget_animates, widget_grid_layout};
use skia_safe::{Canvas, Color};

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
            ox + 7.5 * scale,
            oy + h / 2.0,
            alpha,
            scale,
            text_color,
        );
    }

    animating
}
