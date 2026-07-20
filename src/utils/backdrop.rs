use skia_safe::canvas::SrcRectConstraint;
use skia_safe::{
    AlphaType, ColorType, Data, FilterMode, ISize, Image, ImageInfo, MipmapMode, Paint, Rect,
    SamplingOptions, image_filters, images, surfaces,
};
use std::cell::RefCell;
use std::time::Instant;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{
    DWMWA_SYSTEMBACKDROP_TYPE, DWMWINDOWATTRIBUTE, DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::*;

use crate::core::smtc::MediaInfo;
use crate::ui::expanded::music_view::get_cached_media_image_with_key;

thread_local! {
    static MICA_CACHE: RefCell<Option<MicaCache>> = const { RefCell::new(None) };
    static BLURRED_COVER_CACHE: RefCell<Option<BlurredCoverCache>> = const { RefCell::new(None) };
}

struct BlurredCoverCache {
    cache_key: u64,
    blurred_image: Image,
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

#[allow(clippy::too_many_arguments)]
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
                    || c.timestamp.elapsed().as_millis() >= 1000
            }
        }
    });

    if needs_capture
        && let Some(blurred) = capture_and_blur_mica(monitor_x, monitor_y, monitor_w, monitor_h)
    {
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
        dst_rect,
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
    if monitor_w == 0 || monitor_h == 0 {
        return None;
    }
    let downscale = 8u32;
    let cap_w = (monitor_w / downscale).max(1) as i32;
    let cap_h = (monitor_h / downscale).max(1) as i32;

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

pub fn get_blurred_cover_background(media: &MediaInfo) -> Option<Image> {
    if media.title.is_empty() {
        return None;
    }
    let (img, cache_key) = get_cached_media_image_with_key(media)?;

    let cached = BLURRED_COVER_CACHE.with(|cell| {
        let cache = cell.borrow();
        if let Some(c) = cache.as_ref()
            && c.cache_key == cache_key
        {
            return Some(c.blurred_image.clone());
        }
        None
    });
    if let Some(cached_img) = cached {
        return Some(cached_img);
    }

    // Downscale to 64x64 to make blur extremely fast and smooth
    let mut temp_surface = surfaces::raster_n32_premul(ISize::new(64, 64))?;
    let temp_canvas = temp_surface.canvas();
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    let src_rect = Rect::from_xywh(0.0, 0.0, img.width() as f32, img.height() as f32);
    let dst_rect = Rect::from_xywh(0.0, 0.0, 64.0, 64.0);
    let sampling = SamplingOptions::new(FilterMode::Linear, MipmapMode::None);
    temp_canvas.draw_image_rect_with_sampling_options(
        &img,
        Some((&src_rect, SrcRectConstraint::Fast)),
        dst_rect,
        sampling,
        &paint,
    );
    let downscaled = temp_surface.image_snapshot();

    let mut blur_surface = surfaces::raster_n32_premul(ISize::new(64, 64))?;
    let blur_canvas = blur_surface.canvas();
    let mut blur_paint = Paint::default();
    blur_paint.set_anti_alias(true);
    let sigma = 8.0f32; // Equivalent to heavy blur on full size
    if let Some(filter) = image_filters::blur((sigma, sigma), None, None, None) {
        blur_paint.set_image_filter(filter);
    }
    blur_canvas.draw_image(&downscaled, (0, 0), Some(&blur_paint));
    let blurred = blur_surface.image_snapshot();

    BLURRED_COVER_CACHE.with(|cell| {
        *cell.borrow_mut() = Some(BlurredCoverCache {
            cache_key,
            blurred_image: blurred.clone(),
        });
    });

    Some(blurred)
}

pub fn clear_blurred_cover_cache() {
    BLURRED_COVER_CACHE.with(|cell| {
        *cell.borrow_mut() = None;
    });
}
