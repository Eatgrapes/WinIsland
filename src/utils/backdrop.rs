use skia_safe::{Color, Image, ISize, ImageInfo, ColorType, AlphaType};
use std::cell::RefCell;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};

thread_local! {
    static DYNAMIC_BG_CACHE: RefCell<Option<(String, Color)>> = RefCell::new(None);
    static LAST_VALID_COLOR: RefCell<Option<Color>> = RefCell::new(None);
}

pub fn try_enable_mica(hwnd: HWND) -> bool {
    unsafe {
        let value: i32 = 2;
        let attr = DWMWINDOWATTRIBUTE(38);
        let result = DwmSetWindowAttribute(
            hwnd,
            attr,
            &value as *const _ as *const _,
            std::mem::size_of::<i32>() as u32,
        );
        if result.is_ok() {
            return true;
        }
        let value: i32 = 1;
        let attr = DWMWINDOWATTRIBUTE(1029);
        DwmSetWindowAttribute(
            hwnd,
            attr,
            &value as *const _ as *const _,
            std::mem::size_of::<i32>() as u32,
        ).is_ok()
    }
}

pub fn get_dynamic_bg_color(img: &Image, cache_key: &str) -> Color {
    let cached = DYNAMIC_BG_CACHE.with(|cell| {
        let cache = cell.borrow();
        if let Some((key, color)) = cache.as_ref() {
            if key == cache_key {
                return Some(*color);
            }
        }
        None
    });
    if let Some(color) = cached {
        return color;
    }

    let color = extract_dominant_color(img);
    DYNAMIC_BG_CACHE.with(|cell| {
        *cell.borrow_mut() = Some((cache_key.to_string(), color));
    });
    LAST_VALID_COLOR.with(|cell| {
        *cell.borrow_mut() = Some(color);
    });
    color
}

pub fn get_last_valid_color() -> Option<Color> {
    LAST_VALID_COLOR.with(|cell| *cell.borrow())
}

pub fn clear_dynamic_bg_cache() {
    DYNAMIC_BG_CACHE.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

fn extract_dominant_color(img: &Image) -> Color {
    let w = img.width();
    let h = img.height();
    if w <= 0 || h <= 0 {
        return Color::from_argb(200, 40, 40, 40);
    }

    let info = ImageInfo::new(
        ISize::new(w, h),
        ColorType::BGRA8888,
        AlphaType::Premul,
        None,
    );
    
    let pixel_count = (w * h * 4) as usize;
    let mut pixels = vec![0u8; pixel_count];
    
    if !img.read_pixels(&info, &mut pixels, (w * 4) as usize, (0, 0), skia_safe::image::CachingHint::Allow) {
        return Color::from_argb(200, 40, 40, 40);
    }

    let mut r_sum: u64 = 0;
    let mut g_sum: u64 = 0;
    let mut b_sum: u64 = 0;
    let mut count: u64 = 0;

    let step_x = (w / 8).max(1) as usize;
    let step_y = (h / 8).max(1) as usize;

    for y in (0..h as usize).step_by(step_y) {
        for x in (0..w as usize).step_by(step_x) {
            let idx = (y * w as usize + x) * 4;
            if idx + 3 < pixels.len() {
                let b = pixels[idx] as u64;
                let g = pixels[idx + 1] as u64;
                let r = pixels[idx + 2] as u64;
                let a = pixels[idx + 3] as u64;
                if a > 128 {
                    r_sum += r;
                    g_sum += g;
                    b_sum += b;
                    count += 1;
                }
            }
        }
    }

    if count == 0 {
        return Color::from_argb(200, 40, 40, 40);
    }

    let r = (r_sum / count) as u8;
    let g = (g_sum / count) as u8;
    let b = (b_sum / count) as u8;

    let luminance = 0.299 * r as f32 / 255.0 + 0.587 * g as f32 / 255.0 + 0.114 * b as f32 / 255.0;

    let (nr, ng, nb) = if luminance > 0.5 {
        let factor = 0.3;
        (
            (r as f32 * factor).min(255.0) as u8,
            (g as f32 * factor).min(255.0) as u8,
            (b as f32 * factor).min(255.0) as u8,
        )
    } else {
        let factor = 0.6;
        (
            (r as f32 * factor).min(255.0) as u8,
            (g as f32 * factor).min(255.0) as u8,
            (b as f32 * factor).min(255.0) as u8,
        )
    };

    Color::from_argb(200, nr, ng, nb)
}
