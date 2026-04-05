use skia_safe::{
    Canvas, Paint, Color, FontStyle, Rect, RRect,
    Point, Data, Image, SamplingOptions, FilterMode, MipmapMode,
    gradient_shader, TileMode
};
use crate::icons::arrows::draw_arrow_right;
use crate::core::smtc::MediaInfo;
use std::cell::RefCell;
use std::collections::HashMap;
use crate::utils::font::FontManager;

thread_local! {
    static IMG_CACHE: RefCell<Option<(String, Image)>> = RefCell::new(None);
    static COLOR_CACHE: RefCell<HashMap<String, Vec<Color>>> = RefCell::new(HashMap::new());
    static VIZ_HEIGHTS: RefCell<[f32; 6]> = RefCell::new([3.0; 6]);
    static PROGRESS_SMOOTH: RefCell<f32> = RefCell::new(0.0);
}

pub fn get_progress_bar_rect(ox: f32, oy: f32, w: f32, _media: &MediaInfo, music_active: bool, scale: f32) -> Option<(f32, f32, f32, f32)> {
    if !music_active { return None; }
    let img_size = 72.0 * scale;
    let img_x = ox + 24.0 * scale;
    let img_y = oy + 24.0 * scale;
    let bar_y = img_y + img_size + 18.0 * scale;
    let time_w = 32.0 * scale;
    let bar_full_left = img_x;
    let bar_full_right = img_x + w - 48.0 * scale;
    let bar_left = bar_full_left + time_w + 4.0 * scale;
    let bar_right = bar_full_right - time_w - 4.0 * scale;
    let hit_h = 16.0 * scale;
    Some((bar_left, bar_right, bar_y - hit_h / 2.0, hit_h))
}

pub fn draw_text_cached(canvas: &Canvas, text: &str, pos: (f32, f32), size: f32, style: FontStyle, paint: &Paint, align_center: bool, max_w: f32) {
    FontManager::global().draw_text_cached(canvas, text, pos, size, style, paint, align_center, max_w);
}

pub fn get_cached_media_image(media: &MediaInfo) -> Option<Image> {
    if media.title.is_empty() { return None; }
    let cache_key = format!("{}-{}", media.title, media.album);
    let mut result = None;
    IMG_CACHE.with(|cache| {
        let mut cache_mut = cache.borrow_mut();
        if let Some((key, img)) = cache_mut.as_ref() {
            if key == &cache_key {
                result = Some(img.clone());
                return;
            }
        }
        if let Some(ref bytes_arc) = media.thumbnail {
            let data = Data::new_copy(&**bytes_arc);
            if let Some(image) = Image::from_encoded(data) {
                *cache_mut = Some((cache_key.clone(), image.clone()));
                result = Some(image);
            }
        }
    });
    result
}

pub fn get_media_palette(media: &MediaInfo) -> Vec<Color> {
    if let Some(img) = get_cached_media_image(media) {
        let cache_key = format!("{}-{}", media.title, media.album);
        get_palette_from_image(&img, &cache_key)
    } else {
        vec![Color::from_rgb(180, 180, 180), Color::from_rgb(100, 100, 100)]
    }
}

pub fn draw_main_page(canvas: &Canvas, ox: f32, oy: f32, w: f32, h: f32, alpha: u8, media: &MediaInfo, music_active: bool, view_offset: f32, scale: f32, expansion_progress: f32, viz_h_scale: f32) {
    let arrow_alpha = (alpha as f32 * (1.0 - view_offset * 5.0).clamp(0.0, 1.0)) as u8;
    if arrow_alpha > 0 {
        draw_arrow_right(canvas, ox + w - 12.0 * scale, oy + h / 2.0, arrow_alpha, scale);
    }
    let img_size = 72.0 * scale;
    let img_x = ox + 24.0 * scale;
    let img_y = oy + 24.0 * scale;
    let image_to_draw = if music_active { get_cached_media_image(media) } else { None };
    let cache_key = if music_active { format!("{}-{}", media.title, media.album) } else { "none".to_string() };
    let palette = if let Some(ref img) = image_to_draw {
        get_palette_from_image(img, &cache_key)
    } else {
        vec![Color::from_rgb(180, 180, 180), Color::from_rgb(100, 100, 100)]
    };
    canvas.save();
    canvas.clip_rrect(RRect::new_rect_xy(Rect::from_xywh(img_x, img_y, img_size, img_size), 14.0 * scale, 14.0 * scale), skia_safe::ClipOp::Intersect, true);
    if let Some(img) = image_to_draw {
        let mut img_paint = Paint::default();
        img_paint.set_anti_alias(true);
        img_paint.set_alpha_f(alpha as f32 / 255.0);
        canvas.draw_image_rect_with_sampling_options(
            &img, None, Rect::from_xywh(img_x, img_y, img_size, img_size),
            SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear), &img_paint
        );
    } else {
        draw_placeholder(canvas, img_x, img_y, img_size, alpha, scale);
    }
    canvas.restore();
    let text_x = img_x + img_size + 16.0 * scale;
    let max_text_w = w - (text_x - ox) - 100.0 * scale;
    let title_y = img_y + 26.0 * scale;
    let mut text_paint = Paint::default();
    text_paint.set_anti_alias(true);
    let title = if !music_active || media.title.is_empty() { "No Music playing" } else { &media.title };
    let artist = if !music_active || media.artist.is_empty() { "Unknown Artist" } else { &media.artist };
    text_paint.set_color(Color::from_argb(alpha, 255, 255, 255));
    draw_text_cached(canvas, title, (text_x, title_y), 15.0 * scale, FontStyle::bold(), &text_paint, false, max_text_w);
    text_paint.set_color(Color::from_argb((alpha as f32 * 0.6) as u8, 255, 255, 255));
    draw_text_cached(canvas, artist, (text_x, title_y + 22.0 * scale), 15.0 * scale, FontStyle::normal(), &text_paint, false, max_text_w);

    if music_active {
        let bar_y = img_y + img_size + 18.0 * scale;
        let time_font_size = 10.0 * scale;
        let time_w = 32.0 * scale;

        let current_pos_ms = if media.is_playing {
            media.position_ms + media.last_update.elapsed().as_millis() as u64
        } else {
            media.position_ms
        };
        let duration_ms = if media.duration_ms > 0 {
            media.duration_ms
        } else if media.duration_secs > 0 {
            media.duration_secs * 1000
        } else {
            0
        };
        let current_pos_ms = if duration_ms > 0 { current_pos_ms.min(duration_ms) } else { current_pos_ms };
        let raw_progress = if duration_ms > 0 { current_pos_ms as f32 / duration_ms as f32 } else { 0.0 };

        let progress = PROGRESS_SMOOTH.with(|cell| {
            let mut smooth = cell.borrow_mut();
            let diff = (raw_progress - *smooth).abs();
            if diff > 0.3 {
                *smooth = raw_progress;
            } else {
                *smooth += (raw_progress - *smooth) * 0.15;
            }
            *smooth
        });

        let elapsed_secs = (current_pos_ms / 1000) as u32;
        let elapsed_str = format!("{}:{:02}", elapsed_secs / 60, elapsed_secs % 60);
        let remaining_str = if duration_ms > 0 {
            let remaining_secs = (duration_ms.saturating_sub(current_pos_ms) / 1000) as u32;
            format!("-{}:{:02}", remaining_secs / 60, remaining_secs % 60)
        } else {
            "--:--".to_string()
        };

        let bar_full_left = img_x;
        let bar_full_right = img_x + w - 48.0 * scale;

        let bar_left = bar_full_left + time_w + 4.0 * scale;
        let bar_right = bar_full_right - time_w - 4.0 * scale;
        let bar_total_w = bar_right - bar_left;
        let bar_h = 5.5 * scale;
        let bar_center_y = bar_y;
        let bar_radius = bar_h / 2.0;

        let text_baseline_y = bar_center_y + time_font_size * 0.3 + bar_h * 0.1;

        let mut time_paint = Paint::default();
        time_paint.set_anti_alias(true);
        time_paint.set_color(Color::from_argb((alpha as f32 * 0.5) as u8, 255, 255, 255));
        draw_text_cached(canvas, &elapsed_str, (bar_full_left, text_baseline_y), time_font_size, FontStyle::normal(), &time_paint, false, time_w);
        draw_text_cached(canvas, &remaining_str, (bar_full_right - time_w, text_baseline_y), time_font_size, FontStyle::normal(), &time_paint, false, time_w);

        let mut track_paint = Paint::default();
        track_paint.set_anti_alias(true);
        track_paint.set_color(Color::from_argb((alpha as f32 * 0.25) as u8, 255, 255, 255));
        let track_rect = Rect::from_xywh(bar_left, bar_center_y - bar_h / 2.0, bar_total_w, bar_h);
        canvas.draw_round_rect(track_rect, bar_radius, bar_radius, &track_paint);

        let filled_w = (bar_total_w * progress).max(bar_h);
        let mut fill_paint = Paint::default();
        fill_paint.set_anti_alias(true);
        fill_paint.set_color(Color::from_argb(alpha, 255, 255, 255));
        let fill_rect = Rect::from_xywh(bar_left, bar_center_y - bar_h / 2.0, filled_w, bar_h);
        canvas.draw_round_rect(fill_rect, bar_radius, bar_radius, &fill_paint);
    }

    let viz_x_offset = 17.0 + (45.0 - 17.0) * expansion_progress;
    draw_visualizer(canvas, ox + w - viz_x_offset * scale, title_y - 4.0 * scale, alpha, music_active && media.is_playing, &palette, &media.spectrum, scale, viz_h_scale, (0.6, 0.08));
}

pub fn draw_visualizer(canvas: &Canvas, x: f32, y: f32, alpha: u8, is_playing: bool, palette: &[Color], spectrum: &[f32; 6], w_scale: f32, h_scale: f32, smooth_factors: (f32, f32)) {
    let (rise, fall) = smooth_factors;
    let bar_count = 6;
    let bar_w = 3.0 * w_scale;
    let spacing = 2.0 * w_scale;
    let max_h = 28.0 * h_scale;
    VIZ_HEIGHTS.with(|h_cell| {
        let mut heights = h_cell.borrow_mut();
        for i in 0..bar_count {
            let target = if is_playing { (spectrum[i] * max_h).max(3.0 * h_scale) } else { 3.0 * h_scale };
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
        let colors_with_alpha: Vec<Color> = palette.iter()
            .map(|c| Color::from_argb(alpha, c.r(), c.g(), c.b()))
            .collect();
        if colors_with_alpha.len() >= 2 {
            let shader = gradient_shader::linear(
                (Point::new(start_x, y - max_h/2.0), Point::new(start_x + (20.0 * w_scale), y + max_h/2.0)),
                colors_with_alpha.as_slice(), None, TileMode::Mirror, None, None
            ).unwrap();
            paint.set_shader(shader);
        } else {
            paint.set_color(colors_with_alpha.get(0).cloned().unwrap_or(Color::WHITE));
        }
        for i in 0..bar_count {
            let h = heights[i];
            let rect = Rect::from_xywh(start_x + i as f32 * (bar_w + spacing), y - h / 2.0, bar_w, h);
            let r = bar_w / 2.0;
            canvas.draw_round_rect(rect, r, r, &paint);
        }
    });
}

fn get_palette_from_image(img: &Image, cache_key: &str) -> Vec<Color> {
    COLOR_CACHE.with(|cache| {
        let mut cache_mut = cache.borrow_mut();
        if cache_mut.len() > 50 { cache_mut.clear(); }
        if let Some(palette) = cache_mut.get(cache_key) { return palette.clone(); }
        let mut palette = Vec::new();
        let info = skia_safe::ImageInfo::new(
            skia_safe::ISize::new(img.width(), img.height()),
            skia_safe::ColorType::RGBA8888,
            skia_safe::AlphaType::Premul,
            None,
        );
        let mut pixels = vec![0u8; (img.width() * img.height() * 4) as usize];
        if img.read_pixels(&info, &mut pixels, (img.width() * 4) as usize, (0, 0), skia_safe::image::CachingHint::Allow) {
            let step_x = img.width() / 4;
            let step_y = img.height() / 4;
            let mut r_total = 0u32;
            let mut g_total = 0u32;
            let mut b_total = 0u32;
            let mut count = 0u32;
            for y in 1..4 {
                for x in 1..4 {
                    let idx = ((y * step_y * img.width() + x * step_x) * 4) as usize;
                    if idx + 2 < pixels.len() {
                        r_total += pixels[idx] as u32;
                        g_total += pixels[idx+1] as u32;
                        b_total += pixels[idx+2] as u32;
                        count += 1;
                    }
                }
            }
            if count > 0 {
                let r_avg = r_total as f32 / count as f32;
                let g_avg = g_total as f32 / count as f32;
                let b_avg = b_total as f32 / count as f32;

                let brighten = |r: f32, g: f32, b: f32, factor: f32| -> Color {
                    let mut r = r * factor;
                    let mut g = g * factor;
                    let mut b = b * factor;

                    let brightness = r * 0.299 + g * 0.587 + b * 0.114;
                    if brightness < 80.0 {
                        let boost = 80.0 - brightness;
                        r += boost;
                        g += boost;
                        b += boost;
                    }

                    Color::from_rgb(
                        r.min(255.0) as u8,
                        g.min(255.0) as u8,
                        b.min(255.0) as u8
                    )
                };

                let primary = brighten(r_avg, g_avg, b_avg, 1.3);
                let secondary = brighten(r_avg, g_avg, b_avg, 1.8);

                palette.push(primary);
                palette.push(secondary);
                palette.push(primary);
            }
        }
        if palette.is_empty() { palette.push(Color::from_rgb(200, 200, 200)); }
        cache_mut.insert(cache_key.to_string(), palette.clone());
        palette
    })
}

fn draw_placeholder(canvas: &Canvas, x: f32, y: f32, size: f32, alpha: u8, scale: f32) {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(Color::from_argb((alpha as f32 * 0.1) as u8, 255, 255, 255));
    canvas.draw_round_rect(Rect::from_xywh(x, y, size, size), 14.0 * scale, 14.0 * scale, &paint);
    
    let cx = x + size / 2.0;
    let cy = y + size / 2.0;
    crate::icons::music::draw_music_icon(canvas, cx, cy, alpha, scale * 1.8);
}
