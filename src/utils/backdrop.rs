use skia_safe::canvas::SrcRectConstraint;
use skia_safe::{
    AlphaType, Color, ColorType, Data, FilterMode, ISize, Image, ImageInfo, MipmapMode, Paint,
    Rect, SamplingOptions, image_filters, images, surfaces,
};
use std::cell::RefCell;
use std::time::Instant;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{
    DWMWA_SYSTEMBACKDROP_TYPE, DWMWINDOWATTRIBUTE, DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::*;

thread_local! {
    static DYNAMIC_BG_CACHE: RefCell<Option<(String, Color)>> = const { RefCell::new(None) };
    static LAST_VALID_COLOR: RefCell<Option<Color>> = const { RefCell::new(None) };
    static MICA_CACHE: RefCell<Option<MicaCache>> = const { RefCell::new(None) };
    // Smooth colour transition state for dynamic background (Apple HIG ~400ms).
    static BG_TRANSITION: RefCell<BgTransition> = const {
        RefCell::new(BgTransition {
            target: None,
            from: None,
            display: None,
            start: None,
        })
    };
}

struct BgTransition {
    target: Option<Color>, // raw extracted colour we are heading toward
    from: Option<Color>,   // colour we started transitioning from
    display: Option<Color>, // current interpolated display colour
    start: Option<Instant>, // when the last transition began
}

struct MicaCache {
    monitor_x: i32,
    monitor_y: i32,
    monitor_w: u32,
    monitor_h: u32,
    blurred_image: Image,
    timestamp: Instant,
}

pub fn disable_mica(hwnd: HWND) {
    unsafe {
        let value: i32 = 1;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &value as *const _ as *const _,
            std::mem::size_of::<i32>() as u32,
        );
        let value: i32 = 0;
        let attr = DWMWINDOWATTRIBUTE(1029);
        let _ = DwmSetWindowAttribute(
            hwnd,
            attr,
            &value as *const _ as *const _,
            std::mem::size_of::<i32>() as u32,
        );
    }
}

pub fn get_mica_background(
    screen_x: i32,
    screen_y: i32,
    w: u32,
    h: u32,
    monitor_x: i32,
    monitor_y: i32,
    monitor_w: u32,
    monitor_h: u32,
) -> Option<Image> {
    if w == 0 || h == 0 {
        return None;
    }

    let needs_capture = MICA_CACHE.with(|cell| {
        let cache = cell.borrow();
        match cache.as_ref() {
            None => true,
            Some(c) => {
                c.monitor_x != monitor_x
                    || c.monitor_y != monitor_y
                    || c.monitor_w != monitor_w
                    || c.monitor_h != monitor_h
                    || c.timestamp.elapsed().as_millis() >= 2000
            }
        }
    });

    if needs_capture {
        if let Some(blurred) = capture_and_blur_mica(monitor_x, monitor_y, monitor_w, monitor_h) {
            MICA_CACHE.with(|cell| {
                *cell.borrow_mut() = Some(MicaCache {
                    monitor_x,
                    monitor_y,
                    monitor_w,
                    monitor_h,
                    blurred_image: blurred,
                    timestamp: Instant::now(),
                });
            });
        }
    }

    let blurred = MICA_CACHE.with(|cell| {
        let cache = cell.borrow();
        cache.as_ref().map(|c| c.blurred_image.clone())
    })?;

    let crop_x = (screen_x - monitor_x).max(0) as f32;
    let crop_y = (screen_y - monitor_y).max(0) as f32;

    let bm_w = blurred.width() as f32;
    let bm_h = blurred.height() as f32;

    let src_x = (crop_x / monitor_w as f32 * bm_w).max(0.0);
    let src_y = (crop_y / monitor_h as f32 * bm_h).max(0.0);
    let src_w = (w as f32 / monitor_w as f32 * bm_w).max(1.0);
    let src_h = (h as f32 / monitor_h as f32 * bm_h).max(1.0);

    let src_rect = Rect::from_xywh(src_x, src_y, src_w, src_h);
    let dst_rect = Rect::from_xywh(0.0, 0.0, w as f32, h as f32);

    let mut final_surface = surfaces::raster_n32_premul(ISize::new(w as i32, h as i32))?;
    let final_canvas = final_surface.canvas();
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    let sampling = SamplingOptions::new(FilterMode::Linear, MipmapMode::None);
    final_canvas.draw_image_rect_with_sampling_options(
        &blurred,
        Some((&src_rect, SrcRectConstraint::Fast)),
        &dst_rect,
        sampling,
        &paint,
    );

    Some(final_surface.image_snapshot())
}

pub fn clear_mica_cache() {
    MICA_CACHE.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

fn capture_and_blur_mica(
    monitor_x: i32,
    monitor_y: i32,
    monitor_w: u32,
    monitor_h: u32,
) -> Option<Image> {
    let downscale = 8u32;
    let cap_w = (monitor_w / downscale).max(1) as i32;
    let cap_h = (monitor_h / downscale).max(1) as i32;

    unsafe {
        let hdc_screen = GetDC(HWND::default());
        if hdc_screen.is_invalid() {
            return None;
        }

        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbm = CreateCompatibleBitmap(hdc_screen, cap_w, cap_h);
        let old = SelectObject(hdc_mem, hbm);

        let _ = SetStretchBltMode(hdc_mem, STRETCH_BLT_MODE(HALFTONE.0));
        let _ = StretchBlt(
            hdc_mem,
            0,
            0,
            cap_w,
            cap_h,
            hdc_screen,
            monitor_x,
            monitor_y,
            monitor_w as i32,
            monitor_h as i32,
            SRCCOPY,
        );

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = cap_w;
        bmi.bmiHeader.biHeight = -cap_h;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0;

        let pixel_count = (cap_w * cap_h * 4) as usize;
        let mut pixels = vec![0u8; pixel_count];
        GetDIBits(
            hdc_mem,
            hbm,
            0,
            cap_h as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        SelectObject(hdc_mem, old);
        let _ = DeleteObject(hbm);
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(HWND::default(), hdc_screen);

        for pixel in pixels.chunks_exact_mut(4) {
            pixel[3] = 255;
        }

        let info = ImageInfo::new(
            ISize::new(cap_w, cap_h),
            ColorType::BGRA8888,
            AlphaType::Opaque,
            None,
        );
        let data = Data::new_copy(&pixels);
        let src_img = images::raster_from_data(&info, data, (cap_w * 4) as usize)?;

        let blur_sigma = 6.0f32;
        let mut blur_surface = surfaces::raster_n32_premul(ISize::new(cap_w, cap_h))?;
        let blur_canvas = blur_surface.canvas();
        let mut paint = Paint::default();
        if let Some(filter) = image_filters::blur((blur_sigma, blur_sigma), None, None, None) {
            paint.set_image_filter(filter);
        }
        blur_canvas.draw_image(&src_img, (0, 0), Some(&paint));

        Some(blur_surface.image_snapshot())
    }
}

/// Returns the display colour for the dynamic background, with a ~400ms
/// smoothstep transition when the extracted dominant colour changes (Apple HIG
/// recommends avoiding hard cuts for ambient backgrounds).
pub fn get_dynamic_bg_color(img: &Image, cache_key: &str) -> Color {
    // 1. Obtain raw extracted colour (cached per image to avoid re-extraction).
    let raw_color = DYNAMIC_BG_CACHE.with(|cell| {
        let cache = cell.borrow();
        if let Some((key, color)) = cache.as_ref()
            && key == cache_key
        {
            return Some(*color);
        }
        None
    });
    let raw_color = raw_color.unwrap_or_else(|| {
        let c = extract_dominant_color(img);
        DYNAMIC_BG_CACHE.with(|cell| {
            *cell.borrow_mut() = Some((cache_key.to_string(), c));
        });
        LAST_VALID_COLOR.with(|cell| {
            *cell.borrow_mut() = Some(c);
        });
        c
    });

    // 2. Smooth transition to the new target colour.
    BG_TRANSITION.with(|cell| {
        let mut t = cell.borrow_mut();
        let target_changed = t.target != Some(raw_color);

        if target_changed || t.display.is_none() {
            let now = Instant::now();
            // Snapshot current display colour as the transition start point.
            // If this is the very first call, start from the target itself
            // (no visible transition).
            t.from = t.display.or(Some(raw_color));
            t.target = Some(raw_color);
            t.start = Some(now);
            t.display = t.from;
        }

        // Interpolate with smoothstep over 400ms.
        if let (Some(target), Some(from), Some(start)) =
            (t.target, t.from, t.start)
        {
            let elapsed = start.elapsed().as_secs_f32();
            const DURATION: f32 = 0.4;
            let progress = (elapsed / DURATION).min(1.0);
            // smoothstep: 3t² - 2t³
            let eased = progress * progress * (3.0 - 2.0 * progress);
            let cur = lerp_color(from, target, eased);
            t.display = Some(cur);
            cur
        } else {
            raw_color
        }
    })
}

pub fn get_last_valid_color() -> Option<Color> {
    LAST_VALID_COLOR.with(|cell| *cell.borrow())
}

pub fn clear_dynamic_bg_cache() {
    DYNAMIC_BG_CACHE.with(|cell| {
        *cell.borrow_mut() = None;
    });
    BG_TRANSITION.with(|cell| {
        *cell.borrow_mut() = BgTransition {
            target: None,
            from: None,
            display: None,
            start: None,
        };
    });
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color::from_argb(
        255,
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
    )
}

// ─── Apple HIG-aligned dominant colour extraction ──────────────────────
// Apple Music / Dynamic Island background colour strategy (WWDC 22):
//   1. Prefer chromatic colours over frequency-dominant greys.
//   2. Enforce minimum saturation & value thresholds — discard grey/black/white.
//   3. Normalise to a background-safe luminance range so white text meets
//      4.5:1 contrast.
//   4. Fallback chain: dominant hue → last valid colour → cool-dark default.

const H_BUCKETS: usize = 12; // 30° per bucket
const S_BUCKETS: usize = 4;
const V_BUCKETS: usize = 4;
/// Histogram bucket: (sum_r, sum_g, sum_b, pixel_count)
type HsvBucket = (u64, u64, u64, u32);
type HsvHistogram = [[[HsvBucket; V_BUCKETS]; S_BUCKETS]; H_BUCKETS];
const MIN_SATURATION: f32 = 0.25; // below this = grey → skip
const MIN_VALUE: f32 = 0.15; // too dark to be useful
const MAX_WHITISH_VALUE: f32 = 0.95;
const MAX_WHITISH_SAT: f32 = 0.4; // near-white with low sat → skip
const SAMPLE_GRID: usize = 16; // 16×16 = 256 samples
const FALLBACK_R: u8 = 42;
const FALLBACK_G: u8 = 42;
const FALLBACK_B: u8 = 58; // slightly cool dark, avoids pure grey

fn extract_dominant_color(img: &Image) -> Color {
    let w = img.width();
    let h = img.height();
    if w <= 0 || h <= 0 {
        return fallback_color();
    }

    let info = ImageInfo::new(
        ISize::new(w, h),
        ColorType::BGRA8888,
        AlphaType::Premul,
        None,
    );

    let pixel_count = (w * h * 4) as usize;
    let mut pixels = vec![0u8; pixel_count];
    if !img.read_pixels(
        &info,
        &mut pixels,
        (w * 4) as usize,
        (0, 0),
        skia_safe::image::CachingHint::Allow,
    ) {
        return fallback_color();
    }

    // Build 12 (H) × 4 (S) × 4 (V) histogram, storing summed RGB per bucket.
    let mut buckets: HsvHistogram =
        [[[(0, 0, 0, 0); V_BUCKETS]; S_BUCKETS]; H_BUCKETS];

    let step_x = (w as usize / SAMPLE_GRID).max(1);
    let step_y = (h as usize / SAMPLE_GRID).max(1);

    for y in (0..h as usize).step_by(step_y) {
        for x in (0..w as usize).step_by(step_x) {
            let idx = (y * w as usize + x) * 4;
            if idx + 3 >= pixels.len() {
                continue;
            }
            let a = pixels[idx + 3];
            if a <= 128 {
                continue;
            }
            // Un-premultiply.
            let unmult = 255.0 / a as f64;
            let r = (pixels[idx + 2] as f64 * unmult).min(255.0) as u8;
            let g = (pixels[idx + 1] as f64 * unmult).min(255.0) as u8;
            let b = (pixels[idx] as f64 * unmult).min(255.0) as u8;

            let (hue, sat, val) = rgb_to_hsv(r, g, b);

            // Apple HIG: discard grey, black, white.
            if sat < MIN_SATURATION {
                continue;
            }
            if val < MIN_VALUE {
                continue;
            }
            if val > MAX_WHITISH_VALUE && sat < MAX_WHITISH_SAT {
                continue;
            }

            let hi = ((hue / 360.0 * H_BUCKETS as f32) as usize).min(H_BUCKETS - 1);
            let si = ((sat * S_BUCKETS as f32) as usize).min(S_BUCKETS - 1);
            let vi = ((val * V_BUCKETS as f32) as usize).min(V_BUCKETS - 1);

            let bucket = &mut buckets[hi][si][vi];
            bucket.0 += r as u64;
            bucket.1 += g as u64;
            bucket.2 += b as u64;
            bucket.3 += 1;
        }
    }

    // ── 3. Select best bucket ─────────────────────────────────────────
    // Score = pixel_count × (sat_bucket_index + 1) to favour higher saturation.
    let best = find_best_bucket(&buckets);

    if let Some((r_sum, g_sum, b_sum, count)) = best {
        let r = (r_sum / count as u64).min(255) as u8;
        let g = (g_sum / count as u64).min(255) as u8;
        let b = (b_sum / count as u64).min(255) as u8;
        return normalize_for_background(r, g, b);
    }

    // ── 4. Fallback ───────────────────────────────────────────────────
    // Entire image is grey/black/white or no bucket won →
    // use last valid colour or the cool-dark default.
    fallback_color()
}

/// Find the bucket with the highest weighted score.
/// Score favours more saturated buckets; ties are broken by count.
fn find_best_bucket(
    buckets: &HsvHistogram,
) -> Option<HsvBucket> {
    let mut best: Option<(u64, u64, u64, u32, u64)> = None; // (r,g,b,count,score)

    #[allow(clippy::needless_range_loop)]
    for hi in 0..H_BUCKETS {
        for si in 0..S_BUCKETS {
            for vi in 0..V_BUCKETS {
                let (r, g, b, count) = buckets[hi][si][vi];
                if count == 0 {
                    continue;
                }
                // Weight: count × (saturation tier + 1)
                let score = count as u64 * (si as u64 + 1);
                if best.is_none_or(|(_, _, _, _, s)| score > s) {
                    best = Some((r, g, b, count, score));
                }
            }
        }
    }

    best.map(|(r, g, b, count, _)| (r, g, b, count))
}

/// Normalise a raw dominant colour into a background-safe range.
/// Apple HIG: saturation ∈ [0.35, 0.70], lightness ∈ [0.18, 0.28].
/// This guarantees 4.5:1 contrast with white text while keeping the hue.
fn normalize_for_background(r: u8, g: u8, b: u8) -> Color {
    let (h, _s, l) = rgb_to_hsl(r, g, b);

    // Saturate enough to feel "colourful" but not garish.
    let s_out = {
        let s = saturation_01(r, g, b);
        s.clamp(0.35, 0.70)
    };

    // Lock lightness to a band where white text meets 4.5:1 contrast.
    let l_out = l.clamp(0.18, 0.28);

    let (nr, ng, nb) = hsl_to_rgb(h, s_out, l_out);
    // Fully opaque — Apple uses solid dark backgrounds with shadow for depth,
    // not alpha blending.
    Color::from_argb(255, nr, ng, nb)
}

/// Fallback colour chain: last valid → cool-dark default.
fn fallback_color() -> Color {
    get_last_valid_color()
        .unwrap_or(Color::from_argb(255, FALLBACK_R, FALLBACK_G, FALLBACK_B))
}

// ─── Colour-space helpers ─────────────────────────────────────────────

fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;

    let h = if delta < 0.0001 {
        0.0
    } else if (max - rf).abs() < 0.0001 {
        60.0 * (((gf - bf) / delta) % 6.0)
    } else if (max - gf).abs() < 0.0001 {
        60.0 * (((bf - rf) / delta) + 2.0)
    } else {
        60.0 * (((rf - gf) / delta) + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };

    let s = if max < 0.0001 { 0.0 } else { delta / max };
    let v = max;
    (h, s, v)
}

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let l = (max + min) / 2.0;
    let delta = max - min;

    if delta < 0.0001 {
        return (0.0, 0.0, l);
    }

    let s = if l > 0.5 {
        delta / (2.0 - max - min)
    } else {
        delta / (max + min)
    };

    let h = if (max - rf).abs() < 0.0001 {
        60.0 * (((gf - bf) / delta) % 6.0)
    } else if (max - gf).abs() < 0.0001 {
        60.0 * (((bf - rf) / delta) + 2.0)
    } else {
        60.0 * (((rf - gf) / delta) + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };

    (h, s, l)
}

fn saturation_01(r: u8, g: u8, b: u8) -> f32 {
    let max = r.max(g).max(b) as f32 / 255.0;
    let min = r.min(g).min(b) as f32 / 255.0;
    if max < 0.0001 {
        0.0
    } else {
        (max - min) / max
    }
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (rf, gf, bf) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    (
        ((rf + m) * 255.0).min(255.0) as u8,
        ((gf + m) * 255.0).min(255.0) as u8,
        ((bf + m) * 255.0).min(255.0) as u8,
    )
}
