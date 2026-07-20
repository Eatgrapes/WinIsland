use std::cell::RefCell;

use skia_safe::{Canvas, Color, Paint, Point, Rect, TileMode, gradient_shader};

thread_local! {
    static VIZ_HEIGHTS: RefCell<[f32; 6]> = const { RefCell::new([3.0; 6]) };
}

pub struct DrawVisualizerParams<'a> {
    pub canvas: &'a Canvas,
    pub x: f32,
    pub y: f32,
    pub alpha: u8,
    pub is_playing: bool,
    pub palette: &'a [Color],
    pub spectrum: &'a [f32; 6],
    pub w_scale: f32,
    pub h_scale: f32,
    pub smooth_factors: (f32, f32),
}

pub fn draw_visualizer(params: DrawVisualizerParams<'_>) {
    let DrawVisualizerParams {
        canvas,
        x,
        y,
        alpha,
        is_playing,
        palette,
        spectrum,
        w_scale,
        h_scale,
        smooth_factors,
    } = params;

    let (rise, fall) = smooth_factors;
    let bar_count = 6;
    let bar_w = 3.0 * w_scale;
    let spacing = 2.0 * w_scale;
    let max_h = 28.0 * h_scale;
    VIZ_HEIGHTS.with(|h_cell| {
        let mut heights = h_cell.borrow_mut();
        for i in 0..bar_count {
            let target = if is_playing {
                (spectrum[i] * max_h).max(3.0 * h_scale)
            } else {
                3.0 * h_scale
            };
            if target > heights[i] {
                heights[i] = heights[i] * (1.0 - rise) + target * rise;
            } else {
                heights[i] = heights[i] * (1.0 - fall) + target * fall;
            }
            heights[i] = heights[i].max(3.0 * h_scale);
        }
        let start_x = x - (bar_count as f32 * (bar_w + spacing)) / 2.0;
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        let first = palette.first().copied().unwrap_or(Color::WHITE);
        let second = palette.get(1).copied().unwrap_or(first);
        let colors_with_alpha = [
            Color::from_argb(alpha, first.r(), first.g(), first.b()),
            Color::from_argb(alpha, second.r(), second.g(), second.b()),
        ];
        if palette.len() >= 2 {
            let shader = gradient_shader::linear(
                (
                    Point::new(start_x, y - max_h / 2.0),
                    Point::new(start_x + (20.0 * w_scale), y + max_h / 2.0),
                ),
                &colors_with_alpha[..],
                None,
                TileMode::Mirror,
                None,
                None,
            )
            .unwrap();
            paint.set_shader(shader);
        } else {
            paint.set_color(colors_with_alpha[0]);
        }
        for i in 0..bar_count {
            let h = heights[i];
            let rect = Rect::from_xywh(
                start_x + i as f32 * (bar_w + spacing),
                y - h / 2.0,
                bar_w,
                h,
            );
            let r = bar_w / 2.0;
            canvas.draw_round_rect(rect, r, r, &paint);
        }
    });
}
