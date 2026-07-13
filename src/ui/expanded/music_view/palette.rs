use std::cell::RefCell;
use std::collections::HashMap;

use skia_safe::{Color, Image};

thread_local! {
    static COLOR_CACHE: RefCell<HashMap<String, Vec<Color>>> = RefCell::new(HashMap::new());
}

pub(super) fn get_palette_from_image(img: &Image, cache_key: &str) -> Vec<Color> {
    COLOR_CACHE.with(|cache| {
        let mut cache_mut = cache.borrow_mut();
        if cache_mut.len() > 50
            && let Some(oldest_key) = cache_mut.keys().next().cloned()
        {
            cache_mut.remove(&oldest_key);
        }
        if let Some(palette) = cache_mut.get(cache_key) {
            return palette.clone();
        }
        let mut palette = Vec::new();
        let info = skia_safe::ImageInfo::new(
            skia_safe::ISize::new(img.width(), img.height()),
            skia_safe::ColorType::BGRA8888,
            skia_safe::AlphaType::Premul,
            None,
        );
        let mut pixels = vec![0u8; (img.width() * img.height() * 4) as usize];
        if img.read_pixels(
            &info,
            &mut pixels,
            (img.width() * 4) as usize,
            (0, 0),
            skia_safe::image::CachingHint::Allow,
        ) {
            let step_x = img.width() / 8;
            let step_y = img.height() / 8;
            let mut r_total = 0u32;
            let mut g_total = 0u32;
            let mut b_total = 0u32;
            let mut count = 0u32;
            for y in 1..8 {
                for x in 1..8 {
                    let idx = ((y * step_y * img.width() + x * step_x) * 4) as usize;
                    if idx + 2 < pixels.len() {
                        b_total += pixels[idx] as u32;
                        g_total += pixels[idx + 1] as u32;
                        r_total += pixels[idx + 2] as u32;
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

                    Color::from_rgb(r.min(255.0) as u8, g.min(255.0) as u8, b.min(255.0) as u8)
                };

                let primary = brighten(r_avg, g_avg, b_avg, 1.3);
                let secondary = brighten(r_avg, g_avg, b_avg, 1.5);

                palette.push(primary);
                palette.push(secondary);
                palette.push(primary);
            }
        }
        if palette.is_empty() {
            palette.push(Color::from_rgb(200, 200, 200));
        }
        cache_mut.insert(cache_key.to_string(), palette.clone());
        palette
    })
}
