use skia_safe::{
    AlphaType, Color, ColorType, Data, ISize, Image, ImageInfo, Paint, image_filters, images,
    surfaces,
};
use std::cell::RefCell;
use std::time::Instant;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::*;

type GlassCacheEntry = (Image, Instant, i32, i32, u32, u32);

thread_local! {
    static GLASS_CACHE: RefCell<Option<GlassCacheEntry>> = const { RefCell::new(None) };
}

pub fn set_glass_hwnd(_hwnd_raw: isize) {}

/// Frosted dark glass backdrop: captures the island region + margin from the
/// desktop, then applies a heavy blur (sigma ~40). WDA_EXCLUDEFROMCAPTURE
/// naturally blacks out the island window in the capture, producing a dark
/// base. A strong darkening blend is applied as a fallback in case WDA does
/// not fully work on the current system (e.g. certain GPU drivers or Win10
/// configurations that don't honour the display affinity flag).
pub fn get_glass_background(
    screen_x: i32,
    screen_y: i32,
    w: u32,
    h: u32,
    blur_sigma: f32,
) -> Option<Image> {
    if w == 0 || h == 0 {
        return None;
    }

    let cached = GLASS_CACHE.with(|cell| {
        let cache = cell.borrow();
        if let Some((img, time, cx, cy, cw, ch)) = cache.as_ref()
            && time.elapsed().as_millis() < 500
            && *cx == screen_x
            && *cy == screen_y
            && *cw == w
            && *ch == h
        {
            return Some(img.clone());
        }
        None
    });
    if let Some(img) = cached {
        return Some(img);
    }

    let result = unsafe { capture_and_blur(screen_x, screen_y, w, h, blur_sigma) };

    if let Some(ref img) = result {
        GLASS_CACHE.with(|cell| {
            *cell.borrow_mut() = Some((img.clone(), Instant::now(), screen_x, screen_y, w, h));
        });
    }

    result
}

/// Captures the island region + margin from the desktop, heavily blurs,
/// crops to the island area, then blends with a dark base colour to
/// guarantee the signature dark frosted-glass look.
unsafe fn capture_and_blur(sx: i32, sy: i32, w: u32, h: u32, blur_sigma: f32) -> Option<Image> {
    let downscale = 4u32;
    // Margin is wide enough that after heavy blur the blacked-out island
    // centre gets diluted by surrounding desktop content, producing a dark
    // tint instead of solid black.
    let margin = (w.max(h) / downscale) as i32;
    let cap_full_w = (w as i32 + 2 * margin).max(1);
    let cap_full_h = (h as i32 + 2 * margin).max(1);
    let cap_w = (cap_full_w / downscale as i32).max(1);
    let cap_h = (cap_full_h / downscale as i32).max(1);

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
            sx - margin,
            sy - margin,
            cap_full_w,
            cap_full_h,
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

        // Frosted glass: heavy blur (sigma ~40, stronger than Mica's ~6).
        let scaled_sigma = blur_sigma / downscale as f32;
        let mut blur_surface = surfaces::raster_n32_premul(ISize::new(cap_w, cap_h))?;
        let blur_canvas = blur_surface.canvas();
        let mut paint = Paint::default();
        if let Some(filter) = image_filters::blur((scaled_sigma, scaled_sigma), None, None, None) {
            paint.set_image_filter(filter);
        }
        blur_canvas.draw_image(&src_img, (0, 0), Some(&paint));
        let blurred = blur_surface.image_snapshot();

        let crop_x = (margin / downscale as i32) as f32;
        let crop_y = (margin / downscale as i32) as f32;
        let crop_w = (w / downscale).max(1) as i32;
        let crop_h = (h / downscale).max(1) as i32;

        let mut final_surface = surfaces::raster_n32_premul(ISize::new(crop_w, crop_h))?;
        let final_canvas = final_surface.canvas();
        final_canvas.draw_image(&blurred, (-crop_x, -crop_y), None);

        // Blend with a very dark base to guarantee the signature black glass
        // look even when WDA_EXCLUDEFROMCAPTURE doesn't fully black out the
        // island area on the current system.
        let mut darken = Paint::default();
        darken.set_color(Color::from_argb(195, 8, 8, 12));
        darken.set_anti_alias(true);
        darken.set_blend_mode(skia_safe::BlendMode::Multiply);
        final_canvas.draw_paint(&darken);

        Some(final_surface.image_snapshot())
    }
}
