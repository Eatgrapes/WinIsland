use skia_safe::{Canvas, Color, Paint};
use crate::icons::arrows::draw_arrow_left;
use crate::icons::settings::draw_settings_icon;
use crate::icons::music::draw_music_icon;
use crate::icons::notification::draw_notification_icon;
pub fn draw_tools_page(canvas: &Canvas, ox: f32, oy: f32, w: f32, h: f32, alpha: u8, view_offset: f32, scale: f32, tool_hovers: &[f32; 15], tool_presses: &[f32; 15]) {
    let arrow_alpha = (alpha as f32 * (view_offset - 0.8).max(0.0) * 5.0).clamp(0.0, 255.0) as u8;
    if arrow_alpha > 0 {
        draw_arrow_left(canvas, ox + 12.0 * scale, oy + h / 2.0, arrow_alpha, scale);
    }
    draw_watch_grid_tools(canvas, ox, oy, w, h, alpha, scale, tool_hovers, tool_presses);
}

fn draw_watch_grid_tools(canvas: &Canvas, ox: f32, oy: f32, w: f32, h: f32, alpha: u8, scale: f32, tool_hovers: &[f32; 15], tool_presses: &[f32; 15]) {
    let grid_cols = 5;
    let grid_rows = 3;
    let bubble_r = 18.0 * scale;
    let grid_w = w - 80.0 * scale;
    let grid_h = h - 40.0 * scale;
    let start_x = ox + 40.0 * scale + (grid_w / (grid_cols as f32)) / 2.0;
    let start_y = oy + 20.0 * scale + (grid_h / (grid_rows as f32)) / 2.0;
    let x_step = grid_w / (grid_cols as f32);
    let y_step = grid_h / (grid_rows as f32);

    for r in 0..grid_rows {
        for c in 0..grid_cols {
            let idx = r * grid_cols + c;
            let cx = start_x + (c as f32 * x_step);
            let cy = start_y + (r as f32 * y_step);
            
            let is_settings = r == 0 && c == 0;
            let is_music = r == 0 && c == 1;
            let is_notification = r == 0 && c == 2;
            
            let final_alpha = if is_settings || is_music || is_notification { alpha } else { (alpha as f32 * 0.2) as u8 };
            let hover_progress = tool_hovers[idx];
            let press_progress = tool_presses[idx];
            
            draw_tool_bubble(canvas, cx, cy, bubble_r, final_alpha, hover_progress, press_progress, |canvas, x, y, a, s| {
                if is_settings {
                    draw_settings_icon(canvas, x, y, a, s);
                } else if is_music {
                    draw_music_icon(canvas, x, y, a, s);
                } else if is_notification {
                    draw_notification_icon(canvas, x, y, a, s);
                }
            }, scale);
        }
    }
}

fn draw_tool_bubble<F>(canvas: &Canvas, cx: f32, cy: f32, r: f32, alpha: u8, hover_progress: f32, press_progress: f32, draw_content: F, scale: f32)
where F: FnOnce(&Canvas, f32, f32, u8, f32) {
    let base_scale = 1.0 + 0.1 * hover_progress - 0.15 * press_progress;
    let current_r = r * base_scale;
    let bg_alpha_mult = 0.15 + 0.15 * hover_progress;

    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(Color::from_argb((alpha as f32 * bg_alpha_mult) as u8, 255, 255, 255));
    canvas.draw_circle((cx, cy), current_r, &paint);

    paint.set_style(skia_safe::paint::Style::Stroke);
    paint.set_stroke_width(1.0 * scale);
    paint.set_color(Color::from_argb((alpha as f32 * (bg_alpha_mult + 0.05)) as u8, 255, 255, 255));
    canvas.draw_circle((cx, cy), current_r, &paint);

    draw_content(canvas, cx, cy, alpha, scale * base_scale);
}

