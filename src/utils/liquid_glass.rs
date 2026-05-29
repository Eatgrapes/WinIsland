use skia_safe::{
    AlphaType, Color, ColorType, Data, FilterMode, ISize, Image, ImageInfo, MipmapMode, Paint,
    Rect, RRect, SamplingOptions, TileMode, image_filters, images, surfaces,
};
use skia_safe::canvas::SrcRectConstraint;
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

    float minDim = min(uShape.z, uShape.w);
    float edgeWidth = minDim * 0.15;
    float edgeFactor = smoothstep(0.0, -edgeWidth, dist);

    float2 uv = (coord - uShape.xy) / uShape.zw;

    float2 refractDir = normalize(relPos + 0.001);
    float refractStrength = edgeFactor * minDim * 0.008;
    half4 bg = uBackground.eval(coord + refractDir * refractStrength);
    float3 color = bg.rgb;

    float3 mid = float3(0.45);
    float3 contrasted = (color - mid) * 1.4 + mid;
    color = mix(color, contrasted, edgeFactor * 0.65);
    color *= mix(1.0, 0.94, edgeFactor);

    float gray = dot(color, half3(0.299, 0.587, 0.114));
    color = mix(float3(gray), color, 1.15);

    float diagUV = (uv.x - uv.y + 1.0) * 0.5;
    float cornerHL = smoothstep(0.22, 0.0, diagUV) * 0.7
                   + smoothstep(0.78, 1.0, diagUV) * 0.7;
    cornerHL *= edgeFactor;
    color += cornerHL * 0.12;

    float tlDist = length(float2(uv.x, uv.y));
    float tlHL = smoothstep(0.4, 0.0, tlDist) * edgeFactor;
    color += tlHL * 0.09;

    float brDist = length(float2(1.0 - uv.x, 1.0 - uv.y));
    float brHL = smoothstep(0.4, 0.0, brDist) * edgeFactor;
    color += brHL * 0.04;

    float innerEdge = smoothstep(-minDim * 0.03, 0.0, dist)
                    * smoothstep(minDim * 0.01, 0.0, dist);
    color = mix(color, color * 0.82, innerEdge * 0.4 * edgeFactor);

    float topBright = smoothstep(0.15, 0.0, uv.y) * edgeFactor;
    color += topBright * 0.07;

    float bottomReflect = smoothstep(0.85, 1.0, uv.y) * edgeFactor;
    color += bottomReflect * 0.04;

    float innerBorderDist = abs(dist + minDim * 0.018);
    float innerBorder = smoothstep(minDim * 0.03, 0.0, innerBorderDist) * edgeFactor;
    color = mix(color, color * 0.6, innerBorder * 0.3);

    float outerGlow = smoothstep(2.5, 0.0, abs(dist + 1.5)) * edgeFactor;
    color += outerGlow * 0.06;

    color = mix(color, color * 0.97 + 0.01, edgeFactor * 0.4);

    float chromaOffset = edgeFactor * 1.0;
    half4 cR = uBackground.eval(coord + float2(chromaOffset, 0.0));
    half4 cB = uBackground.eval(coord - float2(chromaOffset, 0.0));
    color.r = mix(color.r, cR.r, edgeFactor * 0.1);
    color.b = mix(color.b, cB.b, edgeFactor * 0.1);

    float insideMask = smoothstep(1.0, -1.0, dist);
    color = mix(bg.rgb, color, insideMask);

    return half4(color, 1.0);
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
    let downscale = 4u32;
    let margin = (w.max(h) / downscale) as i32;
    let cap_full_w = (w as i32 + 2 * margin).max(1);
    let cap_full_h = (h as i32 + 2 * margin).max(1);
    let cap_w = (cap_full_w / downscale as i32).max(1);
    let cap_h = (cap_full_h / downscale as i32).max(1);

    unsafe {
        let hdc_screen = GetDC(windows::Win32::Foundation::HWND::default());
        if hdc_screen.is_invalid() {
            return None;
        }

        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbm = CreateCompatibleBitmap(hdc_screen, cap_w, cap_h);
        let old = SelectObject(hdc_mem, hbm);

        let _ = SetStretchBltMode(hdc_mem, STRETCH_BLT_MODE(HALFTONE.0));
        let _ = StretchBlt(
            hdc_mem, 0, 0, cap_w, cap_h,
            hdc_screen,
            screen_x - margin, screen_y - margin,
            cap_full_w, cap_full_h,
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
            hdc_mem, hbm, 0, cap_h as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi, DIB_RGB_COLORS,
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
            AlphaType::Opaque,
            None,
        );
        let data = Data::new_copy(&pixels);
        let src_img = images::raster_from_data(&info, data, (cap_w * 4) as usize)?;

        let blur_sigma = 8.0f32 / downscale as f32;
        let mut blur_surface = surfaces::raster_n32_premul(ISize::new(cap_w, cap_h))?;
        let blur_canvas = blur_surface.canvas();
        let mut blur_paint = Paint::default();
        if let Some(filter) = image_filters::blur((blur_sigma, blur_sigma), None, None, None) {
            blur_paint.set_image_filter(filter);
        }
        blur_canvas.draw_image(&src_img, (0, 0), Some(&blur_paint));
        let blurred = blur_surface.image_snapshot();

        let effect = get_or_init_effect()?;

        let shape_x = (margin / downscale as i32) as f32;
        let shape_y = (margin / downscale as i32) as f32;
        let shape_w = (w / downscale) as f32;
        let shape_h = (h / downscale) as f32;
        let scaled_radius = corner_radius / downscale as f32;

        let sampling = SamplingOptions::new(FilterMode::Linear, MipmapMode::None);
        let bg_shader = blurred.to_shader(
            (TileMode::Clamp, TileMode::Clamp),
            sampling,
            None,
        )?;

        let mut uniform_data = Vec::with_capacity(20);
        uniform_data.extend_from_slice(&shape_x.to_le_bytes());
        uniform_data.extend_from_slice(&shape_y.to_le_bytes());
        uniform_data.extend_from_slice(&shape_w.to_le_bytes());
        uniform_data.extend_from_slice(&shape_h.to_le_bytes());
        uniform_data.extend_from_slice(&scaled_radius.to_le_bytes());

        let uniform_data_obj = skia_safe::Data::new_copy(&uniform_data);
        let children = [skia_safe::runtime_effect::ChildPtr::from(bg_shader)];
        let liquid_shader = effect.make_shader(uniform_data_obj, &children, None)?;

        let mut shader_surface = surfaces::raster_n32_premul(ISize::new(cap_w, cap_h))?;
        let shader_canvas = shader_surface.canvas();

        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_shader(liquid_shader);
        shader_canvas.draw_rect(
            Rect::from_xywh(0.0, 0.0, cap_w as f32, cap_h as f32),
            &paint,
        );

        let shader_img = shader_surface.image_snapshot();

        let output_w = (w / downscale).max(1) as i32;
        let output_h = (h / downscale).max(1) as i32;
        let crop_x = (margin / downscale as i32) as f32;
        let crop_y = (margin / downscale as i32) as f32;
        let src_rect = Rect::from_xywh(crop_x, crop_y, output_w as f32, output_h as f32);
        let dst_rect = Rect::from_xywh(0.0, 0.0, w as f32, h as f32);

        let mut final_surface = surfaces::raster_n32_premul(ISize::new(w as i32, h as i32))?;
        let final_canvas = final_surface.canvas();

        let mut final_paint = Paint::default();
        final_paint.set_anti_alias(true);
        let final_sampling = SamplingOptions::new(FilterMode::Linear, MipmapMode::None);
        final_canvas.draw_image_rect_with_sampling_options(
            &shader_img,
            Some((&src_rect, SrcRectConstraint::Fast)),
            dst_rect,
            final_sampling,
            &final_paint,
        );

        let final_img = final_surface.image_snapshot();

        let mut border_surface = surfaces::raster_n32_premul(ISize::new(w as i32, h as i32))?;
        let border_canvas = border_surface.canvas();
        border_canvas.draw_image(&final_img, (0, 0), None);

        let mut outer_border = Paint::default();
        outer_border.set_anti_alias(true);
        outer_border.set_color(Color::from_argb(55, 255, 255, 255));
        outer_border.set_style(skia_safe::PaintStyle::Stroke);
        outer_border.set_stroke_width(1.0);
        let outer_rrect = RRect::new_rect_xy(
            Rect::from_xywh(0.0, 0.0, w as f32, h as f32),
            corner_radius,
            corner_radius,
        );
        border_canvas.draw_rrect(outer_rrect, &outer_border);

        let inset = 1.5f32;
        let inner_rrect = RRect::new_rect_xy(
            Rect::from_xywh(inset, inset, w as f32 - inset * 2.0, h as f32 - inset * 2.0),
            (corner_radius - inset).max(0.0),
            (corner_radius - inset).max(0.0),
        );
        let mut inner_border = Paint::default();
        inner_border.set_anti_alias(true);
        inner_border.set_color(Color::from_argb(20, 255, 255, 255));
        inner_border.set_style(skia_safe::PaintStyle::Stroke);
        inner_border.set_stroke_width(0.5);
        border_canvas.draw_rrect(inner_rrect, &inner_border);

        Some(border_surface.image_snapshot())
    }
}
