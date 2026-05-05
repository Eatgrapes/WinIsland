use skia_safe::{Canvas, Color, Paint, Rect, RRect, ClipOp};
use crate::icons::arrows::draw_arrow_left;
use crate::utils::font::FontManager;

pub fn draw_widget_page(canvas: &Canvas, ox: f32, oy: f32, w: f32, h: f32, alpha: u8, scale: f32, invalid_plugin_name: Option<&str>) {
    let arrow_alpha = alpha;
    if arrow_alpha > 0 {
        draw_arrow_left(canvas, ox + 12.0 * scale, oy + h / 2.0, arrow_alpha, scale);
    }

    if let Some(plugin_name) = invalid_plugin_name {
        let bar_h = 28.0 * scale;
        let bar_rect = Rect::from_xywh(ox, oy, w, bar_h);
        let mut bg_paint = Paint::default();
        bg_paint.set_anti_alias(true);
        bg_paint.set_color(Color::from_argb((alpha as f32 * 0.9) as u8, 255, 120, 50));
        let rrect = RRect::new_rect_xy(bar_rect, 0.0, 0.0);
        canvas.save();
        canvas.clip_rrect(rrect, ClipOp::Intersect, true);
        canvas.draw_rect(bar_rect, &bg_paint);

        let warn_text = format!("⚠ {} 无效", plugin_name);
        let font_sz = 11.0 * scale;
        let mut text_paint = Paint::default();
        text_paint.set_anti_alias(true);
        text_paint.set_color(Color::from_argb(alpha, 255, 255, 255));
        FontManager::global().draw_text_centered(canvas, &warn_text, ox + w / 2.0, oy + bar_h / 2.0 + font_sz / 3.0, font_sz, false, &text_paint);
        canvas.restore();
    }

    if alpha > 30 {
        let gear_size = 12.0 * scale;
        let gear_x = ox + w - 28.0 * scale;
        let gear_y = oy + h - 28.0 * scale;
        let mut gear_paint = Paint::default();
        gear_paint.set_anti_alias(true);
        gear_paint.set_color(Color::from_argb((alpha as f32 * 0.5) as u8, 255, 255, 255));
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
}
