use skia_safe::{AlphaType, Color, ColorType, ISize, Image, ImageInfo};
use std::cell::RefCell;
use std::time::Instant;

thread_local! {
    static DYNAMIC_BG_CACHE: RefCell<Option<(String, Color)>> = const { RefCell::new(None) };
    static LAST_VALID_COLOR: RefCell<Option<Color>> = const { RefCell::new(None) };
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
    target: Option<Color>,  // raw extracted colour we are heading toward
    from: Option<Color>,    // colour we started transitioning from
    display: Option<Color>, // current interpolated display colour
    start: Option<Instant>, // when the last transition began
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
        if let (Some(target), Some(from), Some(start)) = (t.target, t.from, t.start) {
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
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
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
    let mut buckets: HsvHistogram = [[[(0, 0, 0, 0); V_BUCKETS]; S_BUCKETS]; H_BUCKETS];
    // Also track a simple all-pixel average (including grey/black/white) so
    // that genuinely monochrome covers don't fall through to the blue default.
    let mut gray_r: u64 = 0;
    let mut gray_g: u64 = 0;
    let mut gray_b: u64 = 0;
    let mut gray_n: u64 = 0;

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

            // All-pixel average (for monochrome-fallback).
            gray_r += r as u64;
            gray_g += g as u64;
            gray_b += b as u64;
            gray_n += 1;

            let (hue, sat, val) = rgb_to_hsv(r, g, b);

            // Apple HIG: discard grey, black, white from chromatic histogram.
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

    // ── 4. Monochrome fallback ─────────────────────────────────────────
    // No chromatic bucket won → the image is genuinely grey/black/white
    // (or near-monochrome like sepia photos). Use the all-pixel average,
    // darkened to the safe luma band, keeping whatever tiny saturation
    // exists so warm/cool tints aren't lost.
    if gray_n > 0 {
        let r = gray_r.checked_div(gray_n).unwrap_or(0).min(255) as u8;
        let g = gray_g.checked_div(gray_n).unwrap_or(0).min(255) as u8;
        let b = gray_b.checked_div(gray_n).unwrap_or(0).min(255) as u8;
        let (h, s_hsl, _l) = rgb_to_hsl(r, g, b);
        // Clamp HSL saturation to at most 0.12 — a sepia photo might have
        // ~0.08 which is worth preserving; pure B&W will be ≈0.0.
        let s_out = s_hsl.clamp(0.0, 0.12);
        let l_out = 0.20;
        let (nr, ng, nb) = hsl_to_rgb(h, s_out, l_out);
        return Color::from_argb(200, nr, ng, nb);
    }

    // ── 5. Ultimate fallback ───────────────────────────────────────────
    // No valid pixels at all → last valid colour or cool-dark default.
    fallback_color()
}

/// Find the bucket with the highest weighted score.
/// Score favours more saturated buckets; ties are broken by count.
fn find_best_bucket(buckets: &HsvHistogram) -> Option<HsvBucket> {
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
/// Apple HIG: saturation ∈ [0.25, 0.42], lightness ∈ [0.18, 0.28].
/// This guarantees 4.5:1 contrast with white text while keeping the hue.
fn normalize_for_background(r: u8, g: u8, b: u8) -> Color {
    let (h, s_hsl, l) = rgb_to_hsl(r, g, b);

    // Clamp HSL saturation (not HSV) to keep colour perceptible but subtle.
    let s_out = s_hsl.clamp(0.25, 0.42);

    // Lock lightness to a band where white text meets 4.5:1 contrast.
    let l_out = l.clamp(0.18, 0.28);

    let (nr, ng, nb) = hsl_to_rgb(h, s_out, l_out);
    // Semi-transparent dark base — lets the island's shadow and glass
    // underlay bleed through for depth.
    Color::from_argb(200, nr, ng, nb)
}

/// Fallback colour chain: last valid → cool-dark default.
fn fallback_color() -> Color {
    get_last_valid_color().unwrap_or(Color::from_argb(200, 32, 32, 36))
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

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let h = h % 360.0;
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
