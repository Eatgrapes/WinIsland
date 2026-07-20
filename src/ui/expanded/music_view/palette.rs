use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use skia_safe::{
    AlphaType, Color, ColorType, FilterMode, ISize, Image, ImageInfo, MipmapMode, Paint, Rect,
    SamplingOptions, surfaces,
};

thread_local! {
    static COLOR_CACHE: RefCell<HashMap<u64, Arc<[Color]>>> = RefCell::new(HashMap::new());
}

pub(super) fn get_palette_from_image(img: &Image, cache_key: u64) -> Arc<[Color]> {
    COLOR_CACHE.with(|cache| {
        let mut cache_mut = cache.borrow_mut();
        if cache_mut.len() > 50
            && let Some(oldest_key) = cache_mut.keys().next().cloned()
        {
            cache_mut.remove(&oldest_key);
        }
        if let Some(palette) = cache_mut.get(&cache_key) {
            return palette.clone();
        }
        let mut palette = Vec::with_capacity(3);
        let info = ImageInfo::new(
            ISize::new(8, 8),
            ColorType::BGRA8888,
            AlphaType::Premul,
            None,
        );
        let mut sample_surface = surfaces::raster_n32_premul(ISize::new(8, 8)).unwrap();
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        sample_surface
            .canvas()
            .draw_image_rect_with_sampling_options(
                img,
                None,
                Rect::from_xywh(0.0, 0.0, 8.0, 8.0),
                SamplingOptions::new(FilterMode::Linear, MipmapMode::None),
                &paint,
            );
        let sampled = sample_surface.image_snapshot();
        let mut pixels = [0u8; 8 * 8 * 4];
        if sampled.read_pixels(
            &info,
            &mut pixels,
            8 * 4,
            (0, 0),
            skia_safe::image::CachingHint::Allow,
        ) {
            let mut r_total = 0u32;
            let mut g_total = 0u32;
            let mut b_total = 0u32;
            let mut count = 0u32;
            for pixel in pixels.chunks_exact(4) {
                b_total += pixel[0] as u32;
                g_total += pixel[1] as u32;
                r_total += pixel[2] as u32;
                count += 1;
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
        let palette: Arc<[Color]> = Arc::from(palette);
        cache_mut.insert(cache_key, palette.clone());
        palette
    })
}
