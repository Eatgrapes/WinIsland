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

pub fn get_dynamic_bg_color(img: &Image, cache_key: &str) -> Color {
    let cached = DYNAMIC_BG_CACHE.with(|cell| {
        let cache = cell.borrow();
        if let Some((key, color)) = cache.as_ref()
            && key == cache_key
        {
            return Some(*color);
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

    if !img.read_pixels(
        &info,
        &mut pixels,
        (w * 4) as usize,
        (0, 0),
        skia_safe::image::CachingHint::Allow,
    ) {
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
