use skia_safe::{Canvas, Color, Paint, FontStyle, Rect, ClipOp};
use std::cell::RefCell;
use crate::icons::arrows::draw_arrow_left;
use crate::core::smtc::MediaInfo;
use crate::ui::expanded::main_view::draw_text_cached;
use crate::utils::font::FontManager;

thread_local! {
    static LYRIC_SCROLL_STATE: RefCell<LyricScrollState> = RefCell::new(LyricScrollState::new());
    static CURRENT_LINE_SCROLL: RefCell<CurrentLineScrollState> = RefCell::new(CurrentLineScrollState::new());
}

struct LyricScrollState {
    current_idx: usize,
    old_idx: usize,
    scroll_progress: f32,
}

impl LyricScrollState {
    fn new() -> Self {
        Self {
            current_idx: 0,
            old_idx: 0,
            scroll_progress: 1.0,
        }
    }

    fn update(&mut self, new_idx: usize) {
        if self.current_idx != new_idx {
            self.old_idx = self.current_idx;
            self.current_idx = new_idx;
            self.scroll_progress = 0.0;
        }
        if self.scroll_progress < 1.0 {
            self.scroll_progress += 0.08;
            if self.scroll_progress > 1.0 {
                self.scroll_progress = 1.0;
            }
        }
    }

    fn is_animating(&self) -> bool {
        self.scroll_progress < 1.0
    }
}

struct CurrentLineScrollState {
    text_hash: u64,
    offset: f32,
    pause: f32,
    direction: i32,
}

impl CurrentLineScrollState {
    fn new() -> Self {
        Self {
            text_hash: 0,
            offset: 0.0,
            pause: 0.0,
            direction: 1,
        }
    }

    fn update(&mut self, text: &str, overflow: f32, dt: f32) {
        let hash = Self::hash_text(text);
        if hash != self.text_hash {
            self.text_hash = hash;
            self.offset = 0.0;
            self.pause = 0.0;
            self.direction = 1;
        }
        
        if overflow <= 0.0 {
            self.offset = 0.0;
            return;
        }
        
        if self.pause > 0.0 {
            self.pause -= dt / 60.0;
            return;
        }
        
        let scroll_speed = 0.6 * dt;
        self.offset += scroll_speed * self.direction as f32;
        
        if self.offset >= overflow {
            self.offset = overflow;
            self.pause = 1.5;
            self.direction = -1;
        } else if self.offset <= 0.0 {
            self.offset = 0.0;
            self.pause = 1.5;
            self.direction = 1;
        }
    }
    
    fn hash_text(text: &str) -> u64 {
        let mut hash: u64 = 5381;
        for byte in text.bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
        }
        hash
    }
}

pub fn draw_widget_page(
    canvas: &Canvas,
    ox: f32,
    oy: f32,
    w: f32,
    h: f32,
    alpha: u8,
    scale: f32,
    media: &MediaInfo,
    _font_size: f32,
    dt: f32,
) -> bool {
    let arrow_alpha = alpha;
    if arrow_alpha > 0 {
        draw_arrow_left(canvas, ox + 12.0 * scale, oy + h / 2.0, arrow_alpha, scale);
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

    if alpha < 10 || media.lyrics.is_none() {
        return false;
    }

    let lyrics = media.lyrics.as_ref().unwrap();
    if lyrics.is_empty() {
        return false;
    }

    let current_pos = if media.is_playing {
        media.position_ms + media.last_update.elapsed().as_millis() as u64
    } else {
        media.position_ms
    };

    let current_idx = match lyrics.binary_search_by_key(&current_pos, |line| line.time_ms) {
        Ok(idx) => idx,
        Err(idx) => {
            if idx > 0 { idx - 1 } else { 0 }
        }
    };

    let lyric_area_left = ox + 40.0 * scale;
    let lyric_area_right = ox + w - 40.0 * scale;
    let lyric_area_top = oy + 12.0 * scale;
    let lyric_area_bottom = oy + h - 12.0 * scale;
    let lyric_area_w = lyric_area_right - lyric_area_left;
    let lyric_area_h = lyric_area_bottom - lyric_area_top;

    if lyric_area_w <= 0.0 || lyric_area_h <= 0.0 {
        return false;
    }

    let font_size = 16.0 * scale;
    let line_h = font_size * 2.0;
    let max_visible_lines = (lyric_area_h / line_h).floor() as usize;
    if max_visible_lines == 0 {
        return false;
    }

    let visible_count = max_visible_lines.min(lyrics.len());
    let half = visible_count / 2;

    let (old_idx, scroll_progress, is_animating) = LYRIC_SCROLL_STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        state.update(current_idx);
        (state.old_idx, state.scroll_progress, state.is_animating())
    });

    let center_y = oy + h / 2.0 + 4.0 * scale;
    let center_x = ox + w / 2.0;

    let idx_diff = current_idx as f32 - old_idx as f32;
    let ease_progress = scroll_progress * scroll_progress * (3.0 - 2.0 * scroll_progress);
    let scroll_offset = -idx_diff * line_h * (1.0 - ease_progress);

    let current_line_text = &lyrics[current_idx].text;
    let current_font_sz = font_size + 6.0 * scale;
    let current_text_w = FontManager::global().measure_text_cached(current_line_text, current_font_sz, FontStyle::normal());
    let current_overflow = (current_text_w - lyric_area_w).max(0.0);
    
    let current_scroll_offset = CURRENT_LINE_SCROLL.with(|cell| {
        let mut state = cell.borrow_mut();
        state.update(current_line_text, current_overflow, dt);
        state.offset
    });
    
    let is_current_scrolling = current_overflow > 0.0;

    canvas.save();
    canvas.clip_rect(
        Rect::from_xywh(lyric_area_left, lyric_area_top, lyric_area_w, lyric_area_h),
        ClipOp::Intersect,
        true,
    );

    let total_lines = lyrics.len();
    let extra_lines = 3;
    
    for i in 0..(visible_count + extra_lines) {
        let idx = (current_idx as isize - half as isize - extra_lines as isize / 2 + i as isize) as isize;
        if idx < 0 || idx >= total_lines as isize {
            continue;
        }
        let idx = idx as usize;

        let is_current = idx == current_idx;
        let is_old_current = idx == old_idx;
        let line = &lyrics[idx];
        if line.text.is_empty() {
            continue;
        }

        let line_y = center_y + (i as f32 - half as f32 - (extra_lines / 2) as f32) * line_h - scroll_offset;

        let (font_sz, text_alpha, should_scroll) = if is_current {
            let fade = if is_animating { ease_progress } else { 1.0 };
            (font_size + 6.0 * scale, (alpha as f32 / 255.0) * fade, is_current_scrolling)
        } else if is_old_current && is_animating {
            let fade = 1.0 - ease_progress;
            (font_size + 6.0 * scale, (alpha as f32 / 255.0) * fade, false)
        } else {
            let dist = (idx as f32 - current_idx as f32).abs();
            let scale_factor = 0.96_f32.powf(dist);
            let opacity_factor = 0.7_f32.powf(dist);
            (font_size * scale_factor, (alpha as f32 / 255.0) * opacity_factor, false)
        };

        if text_alpha < 0.05 {
            continue;
        }

        let mut text_paint = Paint::default();
        text_paint.set_anti_alias(true);
        text_paint.set_color(Color::from_argb(
            (text_alpha * 255.0).min(255.0) as u8,
            255,
            255,
            255,
        ));

        let style = FontStyle::normal();
        
        if should_scroll {
            let text_x = lyric_area_left + 2.0 * scale - current_scroll_offset;
            draw_text_cached(
                canvas,
                &line.text,
                (text_x, line_y),
                font_sz,
                style,
                &text_paint,
                false,
                10000.0,
            );
        } else {
            draw_text_cached(
                canvas,
                &line.text,
                (center_x, line_y),
                font_sz,
                style,
                &text_paint,
                true,
                lyric_area_w,
            );
        }
    }

    canvas.restore();

    is_animating || is_current_scrolling
}
