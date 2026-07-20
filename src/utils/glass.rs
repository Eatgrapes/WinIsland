use skia_safe::{
    AlphaType, ColorType, Data, ISize, Image, ImageInfo, Paint, TileMode, image_filters, images,
    surfaces,
};
use std::cell::RefCell;
use std::time::{Duration, Instant};
use windows::Win32::Graphics::Gdi::*;

const GLASS_REFRESH_INTERVAL: Duration = Duration::from_millis(33);

struct GlassCache {
    image: Image,
    timestamp: Instant,
    monitor_x: i32,
    monitor_y: i32,
    monitor_w: u32,
    monitor_h: u32,
    blur_sigma_bits: u32,
}

thread_local! {
    static GLASS_CACHE: RefCell<Option<GlassCache>> = const { RefCell::new(None) };
}

#[allow(clippy::too_many_arguments)]
pub fn get_glass_background(
    screen_x: i32,
    screen_y: i32,
    width: u32,
    height: u32,
    blur_sigma: f32,
    monitor_x: i32,
    monitor_y: i32,
    monitor_w: u32,
    monitor_h: u32,
) -> Option<Image> {
    if width == 0 || height == 0 || monitor_w == 0 || monitor_h == 0 {
        return None;
    }

    let blur_sigma_bits = blur_sigma.to_bits();
    let cached = GLASS_CACHE.with(|cell| {
        let cache = cell.borrow();
        let cache = cache.as_ref()?;
        (cache.timestamp.elapsed() < GLASS_REFRESH_INTERVAL
            && cache.monitor_x == monitor_x
            && cache.monitor_y == monitor_y
            && cache.monitor_w == monitor_w
            && cache.monitor_h == monitor_h
            && cache.blur_sigma_bits == blur_sigma_bits)
            .then(|| cache.image.clone())
    });
    if cached.is_some() {
        return cached;
    }

    // SAFETY: dimensions are non-zero and capture_and_blur validates every GDI handle.
    let result = unsafe {
        capture_and_blur(
            screen_x, screen_y, width, height, blur_sigma, monitor_x, monitor_y, monitor_w,
            monitor_h,
        )
    };

    if let Some(ref image) = result {
        GLASS_CACHE.with(|cell| {
            *cell.borrow_mut() = Some(GlassCache {
                image: image.clone(),
                timestamp: Instant::now(),
                monitor_x,
                monitor_y,
                monitor_w,
                monitor_h,
                blur_sigma_bits,
            });
        });
    }

    result
}

pub fn clear_glass_cache() {
    GLASS_CACHE.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

#[allow(clippy::too_many_arguments)]
unsafe fn capture_and_blur(
    screen_x: i32,
    screen_y: i32,
    width: u32,
    height: u32,
    blur_sigma: f32,
    monitor_x: i32,
    monitor_y: i32,
    monitor_w: u32,
    monitor_h: u32,
) -> Option<Image> {
    let downscale = 2u32;
    let cap_w = width.div_ceil(downscale) as i32;
    let cap_h = height.div_ceil(downscale) as i32;
    let width_i32 = width as i32;
    let height_i32 = height as i32;
    let monitor_right = monitor_x.saturating_add(monitor_w as i32);
    let monitor_bottom = monitor_y.saturating_add(monitor_h as i32);
    let left_space = screen_x.saturating_sub(monitor_x);
    let right_space = monitor_right.saturating_sub(screen_x.saturating_add(width_i32));
    let capture_x = if right_space >= width_i32 + 10 {
        screen_x + width_i32 + 10
    } else if left_space >= width_i32 + 10 {
        screen_x - width_i32 - 10
    } else if right_space >= left_space {
        monitor_right.saturating_sub(width_i32).max(monitor_x)
    } else {
        monitor_x
    };
    let max_capture_y = monitor_bottom.saturating_sub(height_i32).max(monitor_y);
    let capture_y = screen_y.clamp(monitor_y, max_capture_y);

    // SAFETY: all GDI resources are checked before use and released in reverse order.
    unsafe {
        let hdc_screen = GetDC(None);
        if hdc_screen.is_invalid() {
            return None;
        }

        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        if hdc_mem.is_invalid() {
            ReleaseDC(None, hdc_screen);
            return None;
        }
        let hbm = CreateCompatibleBitmap(hdc_screen, cap_w, cap_h);
        if hbm.is_invalid() {
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(None, hdc_screen);
            return None;
        }
        let old = SelectObject(hdc_mem, hbm.into());

        let _ = SetStretchBltMode(hdc_mem, STRETCH_BLT_MODE(HALFTONE.0));
        let _ = StretchBlt(
            hdc_mem,
            0,
            0,
            cap_w,
            cap_h,
            Some(hdc_screen),
            capture_x,
            capture_y,
            width_i32,
            height_i32,
            SRCCOPY,
        );

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = cap_w;
        bmi.bmiHeader.biHeight = -cap_h;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0;

        let mut pixels = vec![0u8; (cap_w * cap_h * 4) as usize];
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
        let _ = DeleteObject(hbm.into());
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(None, hdc_screen);

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

        let mut blur_surface = surfaces::raster_n32_premul(ISize::new(cap_w, cap_h))?;
        let blur_canvas = blur_surface.canvas();
        let mut paint = Paint::default();
        let scaled_sigma = blur_sigma / downscale as f32;
        if let Some(filter) = image_filters::blur(
            (scaled_sigma, scaled_sigma),
            Some(TileMode::Clamp),
            None,
            None,
        ) {
            paint.set_image_filter(filter);
        }
        blur_canvas.draw_image(&src_img, (0, 0), Some(&paint));

        Some(blur_surface.image_snapshot())
    }
}
