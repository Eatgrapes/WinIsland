use skia_safe::{Color, Paint, Rect, RRect, surfaces, image_filters, Surface as SkSurface, SamplingOptions, FilterMode, MipmapMode, ISize, ClipOp};
use softbuffer::Surface;
use std::sync::Arc;
use std::cell::RefCell;
use winit::window::Window;
use crate::core::config::{PADDING, TOP_OFFSET};
use crate::ui::expanded::main_view::{draw_main_page, get_media_palette, draw_visualizer, get_cached_media_image, draw_text_cached};
use crate::ui::expanded::tools_view::draw_tools_page;
use crate::core::smtc::MediaInfo;

thread_local! {
    static SK_SURFACE: RefCell<Option<SkSurface>> = RefCell::new(None);
}

pub fn draw_island(
    surface: &mut Surface<Arc<Window>, Arc<Window>>,
    current_w: f32,
    current_h: f32,
    current_r: f32,
    os_w: u32,
    os_h: u32,
    _weights: [f32; 4],
    sigmas: (f32, f32),
    expansion_progress: f32,
    view_offset: f32,
    media: &MediaInfo,
    music_active: bool,
    global_scale: f32,
    tool_hovers: &[f32; 15],
    tool_presses: &[f32; 15],
    current_lyric: &str,
    old_lyric: &str,
    lyric_transition: f32,
    use_blur: bool,
    hide_progress: f32,
) {
    let mut buffer = surface.buffer_mut().unwrap();
    let mut sk_surface = SK_SURFACE.with(|cell| {
        let mut opt = cell.borrow_mut();
        if let Some(ref s) = *opt {
            if s.width() == os_w as i32 && s.height() == os_h as i32 { return s.clone(); }
        }
        let new_surface = surfaces::raster_n32_premul(ISize::new(os_w as i32, os_h as i32)).unwrap();
        *opt = Some(new_surface.clone());
        new_surface
    });
    let canvas = sk_surface.canvas();
    canvas.clear(Color::TRANSPARENT);
    
    let offset_x = (os_w as f32 - current_w) / 2.0;
    let base_y = PADDING / 2.0;
    let hidden_peek_h = (5.0 * global_scale).max(3.0);
    let hide_distance = (current_h - hidden_peek_h + TOP_OFFSET as f32).max(0.0);
    let hide_y_offset = hide_progress * hide_distance;
    let offset_y = base_y - hide_y_offset;

    let rect = Rect::from_xywh(offset_x, offset_y, current_w, current_h);
    let rrect = RRect::new_rect_xy(rect, current_r, current_r);
    let has_blur = sigmas.0 > 0.1 || sigmas.1 > 0.1;
    let blur_filter = if has_blur { image_filters::blur(sigmas, None, None, None) } else { None };
    canvas.save();
    canvas.clip_rrect(rrect, ClipOp::Intersect, true);
    let mut bg_paint = Paint::default();
    bg_paint.set_color(Color::BLACK);
    bg_paint.set_anti_alias(true);
    canvas.draw_rrect(rrect, &bg_paint);

    let expanded_alpha_f = (expansion_progress.powf(2.0)).clamp(0.0, 1.0) * (1.0 - hide_progress);
    let mini_alpha_f = (1.0 - expansion_progress * 1.5).clamp(0.0, 1.0) * (1.0 - hide_progress);
    
    let viz_h_scale = 0.45 + (1.0 - 0.45) * expansion_progress;

    if expanded_alpha_f > 0.01 {
        let alpha = (expanded_alpha_f * 255.0) as u8;
        canvas.save();
        if let Some(ref filter) = blur_filter {
            let mut layer_paint = Paint::default();
            layer_paint.set_image_filter(filter.clone());
            canvas.save_layer(&skia_safe::canvas::SaveLayerRec::default().paint(&layer_paint));
        }
        canvas.save();
        canvas.translate((-view_offset * current_w, 0.0));
        draw_main_page(canvas, offset_x, offset_y, current_w, current_h, alpha, media, music_active, view_offset, global_scale, expansion_progress, viz_h_scale * global_scale);
        canvas.restore();
        canvas.save();
        canvas.translate(((1.0 - view_offset) * current_w, 0.0));
        draw_tools_page(canvas, offset_x, offset_y, current_w, current_h, alpha, view_offset, global_scale, tool_hovers, tool_presses);
        canvas.restore();
        if view_offset > 0.01 && view_offset < 0.99 {
            let transition_alpha = (view_offset * (1.0 - view_offset) * 4.0 * 0.4 * alpha as f32 / 255.0 * 255.0) as u8;
            if transition_alpha > 0 {
                let mut trans_paint = Paint::default();
                trans_paint.set_color(Color::from_argb(transition_alpha, 0, 0, 0));
                canvas.draw_rrect(rrect, &trans_paint);
            }
        }
        if blur_filter.is_some() { canvas.restore(); }
        canvas.restore();
    }
    if mini_alpha_f > 0.01 && current_w > 45.0 * global_scale && music_active {
        let alpha = (mini_alpha_f * 255.0) as u8;
        if let Some(image) = get_cached_media_image(media) {
            let size = 18.0 * global_scale; 
            let ix = offset_x + 8.0 * global_scale;
            let iy = offset_y + (current_h - size) / 2.0;
            let mut paint = Paint::default();
            paint.set_anti_alias(true);
            paint.set_alpha_f(alpha as f32 / 255.0);
            canvas.save();
            canvas.clip_rrect(RRect::new_rect_xy(Rect::from_xywh(ix, iy, size, size), 5.0 * global_scale, 5.0 * global_scale), ClipOp::Intersect, true);
            let sampling = SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear);
            canvas.draw_image_rect_with_sampling_options(&image, None, Rect::from_xywh(ix, iy, size, size), sampling, &paint);
            canvas.restore();
        }
        let palette = get_media_palette(media);
        let viz_x = offset_x + current_w - 17.0 * global_scale;
        let viz_y = offset_y + current_h / 2.0;
        draw_visualizer(
            canvas,
            viz_x,
            viz_y,
            alpha,
            media.is_playing,
            &palette,
            &media.spectrum,
            0.55 * global_scale,
            viz_h_scale * global_scale,
            (0.6, 0.08)
        );

        if !current_lyric.is_empty() || !old_lyric.is_empty() {
            let lyric_fade_f = (1.0 - expansion_progress * 2.5).clamp(0.0, 1.0);
            let alpha = (alpha as f32 * lyric_fade_f) as u8;

            if alpha > 0 {
                let space_left = offset_x + 30.0 * global_scale;
                let space_right = offset_x + current_w - 29.0 * global_scale;
                let available_w = space_right - space_left;
                let text_x = space_left + available_w / 2.0;

                canvas.save();
                let clip_rect = Rect::from_xywh(space_left, offset_y, available_w, current_h);
                canvas.clip_rect(clip_rect, ClipOp::Intersect, true);

                if use_blur {
                    if lyric_transition < 1.0 && !old_lyric.is_empty() {
                        let mut text_paint = Paint::default();
                        text_paint.set_anti_alias(true);
                        let fade_alpha = (alpha as f32 * (1.0 - lyric_transition)) as u8;
                        text_paint.set_color(Color::from_argb(fade_alpha, 255, 255, 255));
                        
                        let blur_sigma = lyric_transition * 12.0 * global_scale;
                        if blur_sigma > 0.1 {
                            text_paint.set_image_filter(image_filters::blur((blur_sigma, 0.0), None, None, None));
                        }
                        
                        let text_y = offset_y + current_h / 2.0 + 4.0 * global_scale - (10.0 * global_scale * lyric_transition);
                        draw_text_cached(canvas, old_lyric, (text_x, text_y), 12.0 * global_scale, skia_safe::FontStyle::normal(), &text_paint, true, available_w);
                    }

                    if !current_lyric.is_empty() {
                        let mut text_paint = Paint::default();
                        text_paint.set_anti_alias(true);
                        let fade_alpha = (alpha as f32 * lyric_transition) as u8;
                        text_paint.set_color(Color::from_argb(fade_alpha, 255, 255, 255));

                        let blur_sigma = (1.0 - lyric_transition) * 12.0 * global_scale;
                        if blur_sigma > 0.1 {
                            text_paint.set_image_filter(image_filters::blur((blur_sigma, 0.0), None, None, None));
                        }

                        let text_y = offset_y + current_h / 2.0 + 4.0 * global_scale + (10.0 * global_scale * (1.0 - lyric_transition));
                        draw_text_cached(canvas, current_lyric, (text_x, text_y), 12.0 * global_scale, skia_safe::FontStyle::normal(), &text_paint, true, available_w);
                    }
                } else {
                    let text_y = offset_y + current_h / 2.0 + 4.0 * global_scale;
                    if lyric_transition < 0.5 && !old_lyric.is_empty() {
                        let mut text_paint = Paint::default();
                        text_paint.set_anti_alias(true);
                        let progress = lyric_transition * 2.0;
                        let fade_alpha = (alpha as f32 * (1.0 - progress)) as u8;
                        text_paint.set_color(Color::from_argb(fade_alpha, 255, 255, 255));
                        draw_text_cached(canvas, old_lyric, (text_x, text_y), 12.0 * global_scale, skia_safe::FontStyle::normal(), &text_paint, true, available_w);
                    } else if lyric_transition >= 0.5 && !current_lyric.is_empty() {
                        let mut text_paint = Paint::default();
                        text_paint.set_anti_alias(true);
                        let progress = (lyric_transition - 0.5) * 2.0;
                        let fade_alpha = (alpha as f32 * progress) as u8;
                        text_paint.set_color(Color::from_argb(fade_alpha, 255, 255, 255));
                        draw_text_cached(canvas, current_lyric, (text_x, text_y), 12.0 * global_scale, skia_safe::FontStyle::normal(), &text_paint, true, available_w);
                    }
                }
                canvas.restore();
            }
        }
    }
    canvas.restore(); 
    let info = skia_safe::ImageInfo::new(skia_safe::ISize::new(os_w as i32, os_h as i32), skia_safe::ColorType::BGRA8888, skia_safe::AlphaType::Premul, None);
    let dst_row_bytes = (os_w * 4) as usize;
    let u8_buffer: &mut [u8] = bytemuck::cast_slice_mut(&mut *buffer);
    let _ = sk_surface.read_pixels(&info, u8_buffer, dst_row_bytes, (0, 0));
    buffer.present().unwrap();
}
