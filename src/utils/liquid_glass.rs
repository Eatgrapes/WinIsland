use skia_safe::{
    AlphaType, ColorType, Data, FilterMode, ISize, Image, ImageInfo, MipmapMode, Paint,
    RuntimeEffect, SamplingOptions, TileMode, image_filters, images, runtime_effect::ChildPtr,
    surfaces,
};
use std::cell::RefCell;
use std::time::Instant;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::*;

type CacheEntry = (Image, Instant, i32, i32, u32, u32);

thread_local! {
    static LIQUID_GLASS_CACHE: RefCell<Option<CacheEntry>> = const { RefCell::new(None) };
    static LIQUID_EFFECT: RefCell<Option<RuntimeEffect>> = const { RefCell::new(None) };
}

const SKSL_SOURCE: &str = r#"
uniform shader uBackground;
uniform float uTime;
uniform float4 uRect;
uniform float4 uShape;

float roundedRectSDF(float x, float y, float w, float h, float r) {
    float qx = abs(x) - w + r;
    float qy = abs(y) - h + r;
    float len = length(float2(max(qx, 0.0), max(qy, 0.0)));
    return min(max(qx, qy), 0.0) + len - r;
}

half4 main(float2 coord) {
    float2 relCoord = coord - uShape.xy;
    float2 uv = relCoord / uShape.zw;

    float ix = uv.x - 0.5;
    float iy = uv.y - 0.5;

    float distanceToEdge = roundedRectSDF(ix, iy, 0.3, 0.2, 0.6);
    float displacement = smoothstep(0.8, 0.0, distanceToEdge - 0.15);
    float scaled = smoothstep(0.0, 1.0, displacement);

    float2 sourceUV = float2(ix * scaled + 0.5, iy * scaled + 0.5);
    float2 center = sourceUV * uShape.zw + uShape.xy;

    float blurAmt = 2.0;
    half4 color = uBackground.eval(center) * 0.4;
    color += uBackground.eval(center + float2(blurAmt, 0)) * 0.15;
    color += uBackground.eval(center - float2(blurAmt, 0)) * 0.15;
    color += uBackground.eval(center + float2(0, blurAmt)) * 0.15;
    color += uBackground.eval(center - float2(0, blurAmt)) * 0.15;

    float gray = dot(color.rgb, half3(0.299, 0.587, 0.114));
    color.rgb = mix(float3(gray), color.rgb, 1.1);
    color.rgb *= 1.05;

    return color;
}
"#;

fn get_or_compile_effect() -> Option<RuntimeEffect> {
    if let Some(eff) = LIQUID_EFFECT.with(|cell| cell.borrow().clone()) {
        return Some(eff);
    }
    let eff = RuntimeEffect::make_for_shader(SKSL_SOURCE, None).ok()?;
    LIQUID_EFFECT.with(|cell| {
        *cell.borrow_mut() = Some(eff.clone());
    });
    Some(eff)
}

struct CaptureResult {
    pixels: Vec<u8>,
    cap_x: i32,
    cap_y: i32,
    cap_w: i32,
    cap_h: i32,
}

unsafe fn capture_screen(sx: i32, sy: i32, w: u32, h: u32, margin: i32) -> Option<CaptureResult> {
    let cap_x = (sx - margin).max(0);
    let cap_y = (sy - margin).max(0);
    let cap_w = w as i32 + 2 * margin;
    let cap_h = h as i32 + 2 * margin;

    let hdc_screen = unsafe { GetDC(HWND::default()) };
    if hdc_screen.is_invalid() {
        return None;
    }

    let hdc_mem = unsafe { CreateCompatibleDC(hdc_screen) };
    let hbm = unsafe { CreateCompatibleBitmap(hdc_screen, cap_w, cap_h) };
    let old = unsafe { SelectObject(hdc_mem, hbm) };

    unsafe {
        let _ = BitBlt(
            hdc_mem, 0, 0, cap_w, cap_h, hdc_screen, cap_x, cap_y, SRCCOPY,
        );
    }

    let mut bmi: BITMAPINFO = unsafe { std::mem::zeroed() };
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = cap_w;
    bmi.bmiHeader.biHeight = -cap_h;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB.0;

    let pixel_count = (cap_w * cap_h * 4) as usize;
    let mut pixels = vec![0u8; pixel_count];
    unsafe {
        GetDIBits(
            hdc_mem,
            hbm,
            0,
            cap_h as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );
    }

    unsafe { SelectObject(hdc_mem, old) };
    unsafe {
        let _ = DeleteObject(hbm);
    };
    unsafe {
        let _ = DeleteDC(hdc_mem);
    };
    unsafe {
        ReleaseDC(HWND::default(), hdc_screen);
    };

    Some(CaptureResult {
        pixels,
        cap_x,
        cap_y,
        cap_w,
        cap_h,
    })
}

pub fn get_liquid_glass_background(
    screen_x: i32,
    screen_y: i32,
    w: u32,
    h: u32,
    blur_sigma: f32,
) -> Option<Image> {
    if w == 0 || h == 0 {
        return None;
    }

    let cached = LIQUID_GLASS_CACHE.with(|cell| {
        let cache = cell.borrow();
        if let Some((img, time, cx, cy, cw, ch)) = cache.as_ref()
            && time.elapsed().as_millis() < 100
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

    let margin = (w.max(h) as f32 * blur_sigma / 10.0).max(20.0) as i32;

    let cap = unsafe { capture_screen(screen_x, screen_y, w, h, margin)? };

    let info = ImageInfo::new(
        ISize::new(cap.cap_w, cap.cap_h),
        ColorType::BGRA8888,
        AlphaType::Premul,
        None,
    );
    let data = Data::new_copy(&cap.pixels);
    let src_img = images::raster_from_data(&info, data, (cap.cap_w * 4) as usize)?;

    let mut blur_surface = surfaces::raster_n32_premul(ISize::new(cap.cap_w, cap.cap_h))?;
    let blur_canvas = blur_surface.canvas();
    let mut blur_paint = Paint::default();
    if let Some(filter) = image_filters::blur((blur_sigma, blur_sigma), None, None, None) {
        blur_paint.set_image_filter(filter);
    }
    blur_canvas.draw_image(&src_img, (0, 0), Some(&blur_paint));
    let blurred_img = blur_surface.image_snapshot();

    let effect = get_or_compile_effect()?;

    let shape_x = (screen_x - cap.cap_x) as f32;
    let shape_y = (screen_y - cap.cap_y) as f32;
    let shape_w = w as f32;
    let shape_h = h as f32;

    let sampling = SamplingOptions::new(FilterMode::Linear, MipmapMode::None);
    let bg_shader = blurred_img.to_shader((TileMode::Clamp, TileMode::Clamp), sampling, None)?;

    let time = Instant::now().elapsed().as_secs_f32();
    let rect = [0.0f32, 0.0f32, cap.cap_w as f32, cap.cap_h as f32];
    let shape = [shape_x, shape_y, shape_w, shape_h];

    let liquid_shader = build_shader(&effect, time, rect, shape, bg_shader)?;

    let mut final_surface = surfaces::raster_n32_premul(ISize::new(w as i32, h as i32))?;
    let final_canvas = final_surface.canvas();

    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_shader(liquid_shader);

    let crop_x = (screen_x - cap.cap_x) as f32;
    let crop_y = (screen_y - cap.cap_y) as f32;

    final_canvas.translate((-crop_x, -crop_y));
    final_canvas.draw_rect(
        skia_safe::Rect::from_xywh(0.0, 0.0, cap.cap_w as f32, cap.cap_h as f32),
        &paint,
    );

    let result = final_surface.image_snapshot();

    LIQUID_GLASS_CACHE.with(|cell| {
        *cell.borrow_mut() = Some((result.clone(), Instant::now(), screen_x, screen_y, w, h));
    });

    Some(result)
}

fn build_shader(
    effect: &RuntimeEffect,
    time: f32,
    rect: [f32; 4],
    shape: [f32; 4],
    bg_shader: skia_safe::Shader,
) -> Option<skia_safe::Shader> {
    let mut uniform_data = Vec::with_capacity(36);
    uniform_data.extend_from_slice(&time.to_le_bytes());
    uniform_data.extend_from_slice(&rect[0].to_le_bytes());
    uniform_data.extend_from_slice(&rect[1].to_le_bytes());
    uniform_data.extend_from_slice(&rect[2].to_le_bytes());
    uniform_data.extend_from_slice(&rect[3].to_le_bytes());
    uniform_data.extend_from_slice(&shape[0].to_le_bytes());
    uniform_data.extend_from_slice(&shape[1].to_le_bytes());
    uniform_data.extend_from_slice(&shape[2].to_le_bytes());
    uniform_data.extend_from_slice(&shape[3].to_le_bytes());

    let data = Data::new_copy(&uniform_data);
    let children = [ChildPtr::from(bg_shader)];

    effect.make_shader(data, &children, None)
}
