use skia_safe::{images, Data, ISize, ImageInfo, Image, ColorType, AlphaType, image_filters, Paint, surfaces};
use std::cell::RefCell;
use std::time::Instant;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Foundation::HWND;

thread_local! {
    static GLASS_CACHE: RefCell<Option<(Image, Instant, i32, i32, u32, u32)>> = RefCell::new(None);
}

pub fn set_glass_hwnd(_hwnd_raw: isize) {}

pub fn get_glass_background(screen_x: i32, screen_y: i32, w: u32, h: u32, blur_sigma: f32) -> Option<Image> {
    if w == 0 || h == 0 { return None; }

    let cached = GLASS_CACHE.with(|cell| {
        let cache = cell.borrow();
        if let Some((img, time, cx, cy, cw, ch)) = cache.as_ref() {
            if time.elapsed().as_millis() < 100
                && *cx == screen_x && *cy == screen_y
                && *cw == w && *ch == h
            {
                return Some(img.clone());
            }
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

unsafe fn capture_and_blur(sx: i32, sy: i32, w: u32, h: u32, blur_sigma: f32) -> Option<Image> {
    unsafe {
        let margin = w.max(h) as i32;
        let cap_x = (sx - margin).max(0);
        let cap_y = (sy - margin).max(0);
        let cap_w = w as i32 + 2 * margin;
        let cap_h = h as i32 + 2 * margin;

        let hdc_screen = GetDC(HWND::default());
        if hdc_screen.is_invalid() { return None; }

        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbm = CreateCompatibleBitmap(hdc_screen, cap_w, cap_h);
        let old = SelectObject(hdc_mem, hbm);

        let _ = BitBlt(hdc_mem, 0, 0, cap_w, cap_h, hdc_screen, cap_x, cap_y, SRCCOPY);

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = cap_w;
        bmi.bmiHeader.biHeight = -cap_h;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0 as u32;

        let pixel_count = (cap_w * cap_h * 4) as usize;
        let mut pixels = vec![0u8; pixel_count];
        GetDIBits(
            hdc_mem, hbm, 0, cap_h as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi, DIB_RGB_COLORS,
        );

        SelectObject(hdc_mem, old);
        let _ = DeleteObject(hbm);
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(HWND::default(), hdc_screen);

        let info = ImageInfo::new(
            ISize::new(cap_w, cap_h),
            ColorType::BGRA8888,
            AlphaType::Premul,
            None,
        );
        let data = Data::new_copy(&pixels);
        let src_img = images::raster_from_data(&info, data, (cap_w * 4) as usize)?;

        let mut blur_surface = surfaces::raster_n32_premul(ISize::new(cap_w, cap_h))?;
        let blur_canvas = blur_surface.canvas();
        let mut paint = Paint::default();
        if let Some(filter) = image_filters::blur((blur_sigma, blur_sigma), None, None, None) {
            paint.set_image_filter(filter);
        }
        blur_canvas.draw_image(&src_img, (0, 0), Some(&paint));
        let blurred = blur_surface.image_snapshot();

        let crop_x = (sx - cap_x) as f32;
        let crop_y = (sy - cap_y) as f32;
        let mut final_surface = surfaces::raster_n32_premul(ISize::new(w as i32, h as i32))?;
        let final_canvas = final_surface.canvas();
        final_canvas.draw_image(&blurred, (-crop_x, -crop_y), None);

        Some(final_surface.image_snapshot())
    }
}
