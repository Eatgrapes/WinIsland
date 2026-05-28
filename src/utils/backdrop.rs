use image::GenericImageView;
use skia_safe::{
    AlphaType, Color, ColorType, Data, ISize, Image, ImageInfo, Paint, image_filters, images,
    surfaces,
};
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{
    DWMWA_SYSTEMBACKDROP_TYPE, DWMWINDOWATTRIBUTE, DwmSetWindowAttribute,
};
use windows::Win32::UI::WindowsAndMessaging::SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS;
use windows::Win32::UI::WindowsAndMessaging::SystemParametersInfoW;

thread_local! {
    static DYNAMIC_BG_CACHE: RefCell<Option<(String, Color)>> = const { RefCell::new(None) };
    static LAST_VALID_COLOR: RefCell<Option<Color>> = const { RefCell::new(None) };
}

static MICA_CACHE: Mutex<Option<MicaCache>> = Mutex::new(None);
static MICA_PREWARMING: AtomicBool = AtomicBool::new(false);

struct MicaCache {
    wallpaper_path: String,
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

pub fn prewarm_mica_cache(monitor_x: i32, monitor_y: i32, monitor_w: u32, monitor_h: u32) {
    let wallpaper_path = match get_wallpaper_path() {
        Some(p) => p,
        None => return,
    };

    let cache_key = format!(
        "{}_{}_{}_{}_{}",
        wallpaper_path, monitor_x, monitor_y, monitor_w, monitor_h
    );

    if let Ok(cache) = MICA_CACHE.lock()
        && cache
            .as_ref()
            .is_some_and(|c| c.wallpaper_path == cache_key)
    {
        return;
    }

    if MICA_PREWARMING.swap(true, Ordering::AcqRel) {
        return;
    }

    std::thread::spawn(move || {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let blurred =
                load_and_blur_wallpaper(&wallpaper_path, monitor_x, monitor_y, monitor_w, monitor_h);
            if let Some(img) = blurred {
                if let Ok(mut cache) = MICA_CACHE.lock() {
                    *cache = Some(MicaCache {
                        wallpaper_path: cache_key,
                        blurred_image: img,
                        timestamp: Instant::now(),
                    });
                }
            }
        }));
        MICA_PREWARMING.store(false, Ordering::Release);
    });
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

    let current_wallpaper = get_wallpaper_path()?;
    let cache_key = format!(
        "{}_{}_{}_{}_{}",
        current_wallpaper, monitor_x, monitor_y, monitor_w, monitor_h
    );

    let needs_update = {
        let Ok(cache) = MICA_CACHE.lock() else {
            return None;
        };
        let Some(ref c) = *cache else {
            drop(cache);
            prewarm_mica_cache(monitor_x, monitor_y, monitor_w, monitor_h);
            return None;
        };
        c.wallpaper_path != cache_key || c.timestamp.elapsed().as_secs() >= 30
    };

    if needs_update {
        prewarm_mica_cache(monitor_x, monitor_y, monitor_w, monitor_h);
    }

    let blurred = {
        let Ok(cache) = MICA_CACHE.lock() else {
            return None;
        };
        cache.as_ref()?.blurred_image.clone()
    };

    let crop_x = (screen_x - monitor_x).max(0) as f32;
    let crop_y = (screen_y - monitor_y).max(0) as f32;

    let mut final_surface = surfaces::raster_n32_premul(ISize::new(w as i32, h as i32))?;
    let final_canvas = final_surface.canvas();
    final_canvas.draw_image(&blurred, (-crop_x, -crop_y), None);

    Some(final_surface.image_snapshot())
}

pub fn clear_mica_cache() {
    if let Ok(mut cache) = MICA_CACHE.lock() {
        *cache = None;
    }
}

fn get_wallpaper_path() -> Option<String> {
    unsafe {
        let mut buffer = [0u16; 260];
        let result = SystemParametersInfoW(
            windows::Win32::UI::WindowsAndMessaging::SPI_GETDESKWALLPAPER,
            buffer.len() as u32,
            Some(buffer.as_mut_ptr() as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
        if result.is_ok() {
            let len = buffer.iter().position(|&c| c == 0).unwrap_or(0);
            if len > 0 {
                return Some(String::from_utf16_lossy(&buffer[..len]));
            }
        }
        None
    }
}

fn load_and_blur_wallpaper(
    path: &str,
    monitor_x: i32,
    monitor_y: i32,
    monitor_w: u32,
    monitor_h: u32,
) -> Option<Image> {
    let img_data = std::fs::read(PathBuf::from(path)).ok()?;
    let dyn_img = image::ImageReader::new(std::io::Cursor::new(&img_data))
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;

    let (virtual_x, virtual_y, virtual_w, virtual_h) = get_virtual_screen_rect();

    let thumb_max = 256u32;
    let (orig_w, orig_h) = dyn_img.dimensions();
    let scale = if orig_w > orig_h {
        thumb_max as f32 / orig_w as f32
    } else {
        thumb_max as f32 / orig_h as f32
    };
    let thumb_w = ((orig_w as f32 * scale).round() as u32).max(1);
    let thumb_h = ((orig_h as f32 * scale).round() as u32).max(1);
    let thumb = dyn_img.thumbnail(thumb_w, thumb_h);
    let rgba = thumb.to_rgba8();
    let (tw, th) = rgba.dimensions();

    let info = ImageInfo::new(
        ISize::new(tw as i32, th as i32),
        ColorType::BGRA8888,
        AlphaType::Unpremul,
        None,
    );

    let mut bgra_pixels = vec![0u8; (tw * th * 4) as usize];
    for (i, pixel) in rgba.pixels().enumerate() {
        let dst = i * 4;
        bgra_pixels[dst] = pixel[2];
        bgra_pixels[dst + 1] = pixel[1];
        bgra_pixels[dst + 2] = pixel[0];
        bgra_pixels[dst + 3] = pixel[3];
    }

    let data = Data::new_copy(&bgra_pixels);
    let src_img = images::raster_from_data(&info, data, (tw * 4) as usize)?;

    let scale_x = tw as f32 / virtual_w as f32;
    let scale_y = th as f32 / virtual_h as f32;

    let crop_x = ((monitor_x - virtual_x) as f32 * scale_x).round() as i32;
    let crop_y = ((monitor_y - virtual_y) as f32 * scale_y).round() as i32;
    let crop_w = ((monitor_w as f32 * scale_x).round() as i32).max(1);
    let crop_h = ((monitor_h as f32 * scale_y).round() as i32).max(1);

    let mut crop_surface = surfaces::raster_n32_premul(ISize::new(crop_w, crop_h))?;
    let crop_canvas = crop_surface.canvas();
    crop_canvas.draw_image(&src_img, (-crop_x as f32, -crop_y as f32), None);
    let cropped = crop_surface.image_snapshot();

    let blur_sigma = 30.0f32;
    let mut blur_surface = surfaces::raster_n32_premul(ISize::new(crop_w, crop_h))?;
    let blur_canvas = blur_surface.canvas();
    let mut paint = Paint::default();
    if let Some(filter) = image_filters::blur((blur_sigma, blur_sigma), None, None, None) {
        paint.set_image_filter(filter);
    }
    blur_canvas.draw_image(&cropped, (0, 0), Some(&paint));

    Some(blur_surface.image_snapshot())
}

fn get_virtual_screen_rect() -> (i32, i32, i32, i32) {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
            SM_YVIRTUALSCREEN,
        };
        let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
        (x, y, w, h)
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
