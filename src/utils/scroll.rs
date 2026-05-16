use crate::utils::font::{DrawTextCachedParams, FontManager};
use skia_safe::{Canvas, ClipOp, FontStyle, Paint, Rect};
use std::time::Instant;

pub struct ScrollDrawParams<'a> {
    pub canvas: &'a Canvas,
    pub text: &'a str,
    pub x: f32,
    pub y: f32,
    pub max_w: f32,
    pub size: f32,
    pub style: FontStyle,
    pub paint: &'a Paint,
    pub scale: f32,
}

pub struct ScrollText {
    last_text: String,
    offset: f32,
    pause: f32,
    last_time: Instant,
}

impl ScrollText {
    pub fn new() -> Self {
        Self {
            last_text: String::new(),
            offset: 0.0,
            pause: 2.0,
            last_time: Instant::now(),
        }
    }

    pub fn draw(&mut self, params: ScrollDrawParams<'_>) {
        let canvas = params.canvas;
        let text = params.text;
        let x = params.x;
        let y = params.y;
        let max_w = params.max_w;
        let size = params.size;
        let style = params.style;
        let paint = params.paint;
        let scale = params.scale;

        if self.last_text != text {
            self.last_text = text.to_string();
            self.offset = 0.0;
            self.pause = 2.0;
        }

        let dt = self.last_time.elapsed().as_secs_f32().clamp(0.0, 0.1);
        self.last_time = Instant::now();

        let full_w = FontManager::global().measure_text_cached(text, size, style);

        if full_w > max_w {
            if self.pause > 0.0 {
                self.pause -= dt;
            } else {
                self.offset += 35.0 * scale * dt;
                let reset_w = full_w + 50.0 * scale;
                if self.offset >= reset_w {
                    self.offset = 0.0;
                    self.pause = 2.0;
                }
            }

            canvas.save();
            canvas.clip_rect(
                Rect::from_xywh(x, y - size * 1.2, max_w, size * 1.5),
                ClipOp::Intersect,
                true,
            );

            FontManager::global().draw_text_cached(DrawTextCachedParams {
                canvas,
                text,
                pos: (x - self.offset, y),
                size,
                style,
                paint,
                align_center: false,
                max_w: f32::MAX,
            });
            let next_x = x - self.offset + full_w + 50.0 * scale;
            if next_x < x + max_w {
                FontManager::global().draw_text_cached(DrawTextCachedParams {
                    canvas,
                    text,
                    pos: (next_x, y),
                    size,
                    style,
                    paint,
                    align_center: false,
                    max_w: f32::MAX,
                });
            }
            canvas.restore();
        } else {
            self.offset = 0.0;
            FontManager::global().draw_text_cached(DrawTextCachedParams {
                canvas,
                text,
                pos: (x, y),
                size,
                style,
                paint,
                align_center: false,
                max_w,
            });
        }
    }
}
