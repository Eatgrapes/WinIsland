use skia_safe::{
    AlphaType, Color, ColorType, Data, FilterMode, ISize, Image, ImageInfo, MipmapMode, Paint,
    RRect, Rect, SamplingOptions, TileMode, image_filters, images, surfaces,
};
use std::cell::RefCell;
use std::time::Instant;
use windows::Win32::Graphics::Gdi::*;

const SKSL_SOURCE: &str = r#"
uniform shader uBackground;
uniform float4 uShape;
uniform float uRadius;

float roundedRectSDF(float2 p, float2 b, float r) {
    float2 q = abs(p) - b + r;
    return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - r;
}

half4 main(float2 coord) {
    float2 center = uShape.xy + uShape.zw * 0.5;
    float2 halfSize = uShape.zw * 0.5;
    float2 relPos = coord - center;

    float dist = roundedRectSDF(relPos, halfSize, uRadius);

    float2 uv = (coord - uShape.xy) / uShape.zw;
    float ix = uv.x - 0.5;
    float iy = uv.y - 0.5;

    float minDim = min(uShape.z, uShape.w);
    float normDist = dist / minDim;

    float displacement = smoothstep(0.8, 0.0, normDist - 0.15);
    float scaled = smoothstep(0.0, 1.0, displacement);

    float2 sourceUV = float2(ix * scaled + 0.5, iy * scaled + 0.5);
    float2 sourceCoord = sourceUV * uShape.zw + uShape.xy;

    float blurAmt = 6.0;
    half4 color = uBackground.eval(sourceCoord) * 0.4;
    color += uBackground.eval(sourceCoord + float2(blurAmt, 0)) * 0.15;
    color += uBackground.eval(sourceCoord - float2(blurAmt, 0)) * 0.15;
    color += uBackground.eval(sourceCoord + float2(0, blurAmt)) * 0.15;
    color += uBackground.eval(sourceCoord - float2(0, blurAmt)) * 0.15;

    float gray = dot(color.rgb, half3(0.299, 0.587, 0.114));
    color.rgb = mix(float3(gray), color.rgb, 1.1);
    color.rgb *= 1.05;

    float edgeBright = smoothstep(0.0, -0.3, normDist) * 0.08;
    color.rgb += edgeBright;

    float specY = smoothstep(0.15, 0.0, uv.y) * smoothstep(-0.02, 0.08, uv.y);
    float specX = smoothstep(0.1, 0.3, uv.x) * smoothstep(0.9, 0.7, uv.x);
    float specular = specY * specX * smoothstep(0.0, -0.2, normDist);
    color.rgb += specular * 0.15;

    color.rgb += smoothstep(0.3, 0.0, uv.y) * 0.03;

    color.rgb = clamp(color.rgb, 0.0, 1.0);

    return color;
}
"#;

type CacheEntry = (Image, Instant, i32, i32, u32, u32);

thread_local! {
    static GLASS_CACHE: RefCell<Option<CacheEntry>> = const { RefCell::new(None) };
    static EFFECT_CACHE: RefCell<Option<skia_safe::RuntimeEffect>> = const { RefCell::new(None) };
}

fn get_or_init_effect() -> Option<skia_safe::RuntimeEffect> {
    EFFECT_CACHE.with(|cell| {
        if let Some(eff) = cell.borrow().as_ref() {
            return Some(eff.clone());
        }
        let eff = skia_safe::RuntimeEffect::make_for_shader(SKSL_SOURCE, None).ok()?;
        *cell.borrow_mut() = Some(eff.clone());
        Some(eff)
    })
}

#[allow(clippy::too_many_arguments)]
pub fn get_liquid_glass_background(
    screen_x: i32,
    screen_y: i32,
    w: u32,
    h: u32,
    corner_radius: f32,
    _monitor_x: i32,
    _monitor_y: i32,
    _monitor_w: u32,
    _monitor_h: u32,
) -> Option<Image> {
    if w == 0 || h == 0 {
        return None;
    }

    let cached = GLASS_CACHE.with(|cell| {
        let cache = cell.borrow();
        if let Some((img, time, cx, cy, cw, ch)) = cache.as_ref()
            && time.elapsed().as_millis() < 200
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

    let result = render_liquid_glass(screen_x, screen_y, w, h, corner_radius);

    if let Some(ref img) = result {
        GLASS_CACHE.with(|cell| {
            *cell.borrow_mut() = Some((img.clone(), Instant::now(), screen_x, screen_y, w, h));
        });
    }

    result
}

pub fn clear_liquid_glass_cache() {
    GLASS_CACHE.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

fn render_liquid_glass(
    screen_x: i32,
    screen_y: i32,
    w: u32,
    h: u32,
    corner_radius: f32,
) -> Option<Image> {
    // PERFORMANCE: downscale 4x like glass.rs to reduce pixel count 16x.
    // The SKSL shader does 5 texture lookups per pixel, so this dramatically
    // reduces GPU work. The result is scaled back up at the end.
    let downscale = 4u32;
    let margin = ((w.max(h) / downscale) as i32).max(20);
    let cap_full_w = (w as i32 + 2 * margin).max(1);
    let cap_full_h = (h as i32 + 2 * margin).max(1);
    let cap_w = (cap_full_w / downscale as i32).max(1);
    let cap_h = (cap_full_h / downscale as i32).max(1);

    // SAFETY: GDI screen capture for liquid glass backdrop. All Win32 API
    // calls operate on valid handles obtained within this block. Resources
    // are released in reverse order: SelectObject restore, DeleteObject,
    // DeleteDC, ReleaseDC. GetDC with default HWND retrieves the desktop DC.
    unsafe {
        let hdc_screen = GetDC(windows::Win32::Foundation::HWND::default());
        if hdc_screen.is_invalid() {
            return None;
        }

        let hdc_mem = CreateCompatibleDC(hdc_screen);
        if hdc_mem.is_invalid() {
            ReleaseDC(windows::Win32::Foundation::HWND::default(), hdc_screen);
            return None;
        }
        let hbm = CreateCompatibleBitmap(hdc_screen, cap_w, cap_h);
        if hbm.is_invalid() {
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(windows::Win32::Foundation::HWND::default(), hdc_screen);
            return None;
        }
        let old = SelectObject(hdc_mem, hbm);

        let _ = SetStretchBltMode(hdc_mem, STRETCH_BLT_MODE(HALFTONE.0));
        let _ = StretchBlt(
            hdc_mem,
            0,
            0,
            cap_w,
            cap_h,
            hdc_screen,
            screen_x - margin,
            screen_y - margin,
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
        ReleaseDC(windows::Win32::Foundation::HWND::default(), hdc_screen);

        for pixel in pixels.chunks_exact_mut(4) {
            pixel[3] = 255;
        }

        let info = ImageInfo::new(
            ISize::new(cap_w, cap_h),
            ColorType::BGRA8888,
            AlphaType::Premul,
            None,
        );
        let data = Data::new_copy(&pixels);
        let src_img = images::raster_from_data(&info, data, (cap_w * 4) as usize)?;

        // Blur at the downscaled resolution (cheap)
        let blur_sigma = 20.0 / downscale as f32;
        let mut blur_surface = surfaces::raster_n32_premul(ISize::new(cap_w, cap_h))?;
        let blur_canvas = blur_surface.canvas();
        let mut blur_paint = Paint::default();
        if let Some(filter) = image_filters::blur((blur_sigma, blur_sigma), None, None, None) {
            blur_paint.set_image_filter(filter);
        }
        blur_canvas.draw_image(&src_img, (0, 0), Some(&blur_paint));
        let blurred = blur_surface.image_snapshot();

        let effect = get_or_init_effect()?;

        // The effect is applied at downscaled coords but the shader expects
        // full-res shape dimensions. Scale uniforms accordingly.
        let ds = downscale as f32;
        let scaled_margin = margin as f32;
        let shape_x = scaled_margin;
        let shape_y = scaled_margin;
        let shape_w = w as f32 / ds;
        let shape_h = h as f32 / ds;

        let sampling = SamplingOptions::new(FilterMode::Linear, MipmapMode::None);
        let bg_shader = blurred.to_shader((TileMode::Clamp, TileMode::Clamp), sampling, None)?;

        let uniform_size = effect.uniform_size();
        let mut uniform_data = vec![0u8; uniform_size];
        let write_f32 = |data: &mut [u8], offset: usize, val: f32| {
            data[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
        };
        for u in effect.uniforms() {
            match u.name() {
                "uShape" => {
                    let off = u.offset();
                    write_f32(&mut uniform_data, off, shape_x);
                    write_f32(&mut uniform_data, off + 4, shape_y);
                    write_f32(&mut uniform_data, off + 8, shape_w);
                    write_f32(&mut uniform_data, off + 12, shape_h);
                }
                "uRadius" => {
                    let scaled = if corner_radius > 0.0 {
                        (corner_radius / ds).max(1.0)
                    } else {
                        0.0
                    };
                    write_f32(&mut uniform_data, u.offset(), scaled);
                }
                _ => {}
            }
        }

        let uniform_data_obj = skia_safe::Data::new_copy(&uniform_data);
        let children = [skia_safe::runtime_effect::ChildPtr::from(bg_shader)];
        let liquid_shader = effect.make_shader(uniform_data_obj, &children, None)?;

        // Render effect at downscaled size
        let final_w = (w / downscale).max(1) as i32;
        let final_h = (h / downscale).max(1) as i32;

        let mut final_surface = surfaces::raster_n32_premul(ISize::new(final_w, final_h))?;
        let final_canvas = final_surface.canvas();

        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_shader(liquid_shader);

        final_canvas.draw_rect(
            Rect::from_xywh(0.0, 0.0, cap_w as f32, cap_h as f32),
            &paint,
        );

        let rendered = final_surface.image_snapshot();

        // Scale back up to full resolution
        let mut upscale_surface = surfaces::raster_n32_premul(ISize::new(w as i32, h as i32))?;
        let up_canvas = upscale_surface.canvas();
        let up_rect = Rect::from_xywh(0.0, 0.0, w as f32, h as f32);
        let up_sampling = SamplingOptions::new(FilterMode::Linear, MipmapMode::None);
        up_canvas.draw_image_rect_with_sampling_options(
            &rendered,
            None,
            up_rect,
            up_sampling,
            &Paint::default(),
        );

        // Border rendering at full res
        let mut outer_border = Paint::default();
        outer_border.set_anti_alias(true);
        outer_border.set_color(Color::from_argb(110, 255, 255, 255));
        outer_border.set_style(skia_safe::PaintStyle::Stroke);
        outer_border.set_stroke_width(1.0);
        let outer_rrect = RRect::new_rect_xy(
            Rect::from_xywh(0.5, 0.5, w as f32 - 1.0, h as f32 - 1.0),
            corner_radius,
            corner_radius,
        );
        up_canvas.draw_rrect(outer_rrect, &outer_border);

        let inset = 1.0f32;
        let inner_rrect = RRect::new_rect_xy(
            Rect::from_xywh(inset, inset, w as f32 - inset * 2.0, h as f32 - inset * 2.0),
            (corner_radius - inset).max(0.0),
            (corner_radius - inset).max(0.0),
        );
        let mut inner_border = Paint::default();
        inner_border.set_anti_alias(true);
        inner_border.set_color(Color::from_argb(55, 255, 255, 255));
        inner_border.set_style(skia_safe::PaintStyle::Stroke);
        inner_border.set_stroke_width(0.5);
        up_canvas.draw_rrect(inner_rrect, &inner_border);

        Some(upscale_surface.image_snapshot())
    }
}
