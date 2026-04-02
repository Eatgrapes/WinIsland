use skia_safe::{Color, Paint, Rect, RRect, surfaces, image_filters, Surface as SkSurface, SamplingOptions, FilterMode, MipmapMode, ISize, ClipOp};
use softbuffer::Surface;
use std::sync::Arc;
use std::cell::RefCell;
use winit::window::Window;
use crate::core::config::{PADDING, TOP_OFFSET};
use crate::ui::expanded::main_view::{draw_main_page, get_media_palette, draw_visualizer, get_cached_media_image, draw_text_cached};
use crate::ui::expanded::widget_view::draw_widget_page;
use crate::core::smtc::MediaInfo;
use crate::core::notification::NotificationInfo;

thread_local! {
    static SK_SURFACE: RefCell<Option<SkSurface>> = RefCell::new(None);
}

fn spring_ease(t: f32) -> f32 {
    let c4 = (2.0 * std::f32::consts::PI) / 3.0;
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
    }
}

fn ease_out_expo(t: f32) -> f32 {
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        1.0 - 2.0_f32.powf(-10.0 * t)
    }
}

fn ease_out_back(t: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

fn draw_notification_expanded(
    canvas: &skia_safe::Canvas,
    ox: f32,
    oy: f32,
    w: f32,
    alpha: u8,
    notif: &NotificationInfo,
    scale: f32,
    show_app_name: bool,
) {
    let img_size = 72.0 * scale;
    let img_x = ox + 24.0 * scale;
    let img_y = oy + 24.0 * scale;
    
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(Color::from_argb((alpha as f32 * 0.3) as u8, 128, 128, 128));
    let rrect = RRect::new_rect_xy(Rect::from_xywh(img_x, img_y, img_size, img_size), 14.0 * scale, 14.0 * scale);
    canvas.draw_rrect(rrect, &paint);
    
    let initial = notif.app_name.chars().next().unwrap_or('?').to_uppercase().next().unwrap_or('?');
    let initial_str = initial.to_string();
    let mut text_paint = Paint::default();
    text_paint.set_anti_alias(true);
    text_paint.set_color(Color::from_argb(alpha, 255, 255, 255));
    draw_text_cached(canvas, &initial_str, (img_x + img_size / 2.0, img_y + img_size / 2.0), img_size * 0.4, skia_safe::FontStyle::bold(), &text_paint, true, img_size);
    
    let text_x = img_x + img_size + 16.0 * scale;
    let max_text_w = w - (text_x - ox) - 40.0 * scale;
    let base_title_y = img_y + 26.0 * scale;
    
    let (title_y, content_y, _body_y) = if show_app_name {
        let mut app_paint = Paint::default();
        app_paint.set_anti_alias(true);
        app_paint.set_color(Color::from_argb(alpha, 255, 255, 255));
        draw_text_cached(canvas, &notif.app_name, (text_x, base_title_y), 15.0 * scale, skia_safe::FontStyle::normal(), &app_paint, false, max_text_w);
        (base_title_y + 22.0 * scale, base_title_y + 44.0 * scale, base_title_y + 66.0 * scale)
    } else {
        (base_title_y, base_title_y + 22.0 * scale, base_title_y + 44.0 * scale)
    };
    
    if !notif.title.is_empty() {
        let mut title_paint = Paint::default();
        title_paint.set_anti_alias(true);
        title_paint.set_color(Color::from_argb(alpha, 255, 255, 255));
        draw_text_cached(canvas, &notif.title, (text_x, title_y), 15.0 * scale, skia_safe::FontStyle::bold(), &title_paint, false, max_text_w);
    }
    
    if !notif.body.is_empty() {
        let mut body_paint = Paint::default();
        body_paint.set_anti_alias(true);
        body_paint.set_color(Color::from_argb((alpha as f32 * 0.6) as u8, 255, 255, 255));
        draw_text_cached(canvas, &notif.body, (text_x, content_y), 15.0 * scale, skia_safe::FontStyle::normal(), &body_paint, false, max_text_w);
    }
}

fn draw_music_section(
    canvas: &skia_safe::Canvas,
    ox: f32,
    oy: f32,
    w: f32,
    _h: f32,
    alpha: u8,
    media: &MediaInfo,
    music_active: bool,
    scale: f32,
    expansion_progress: f32,
    viz_h_scale: f32,
) {
    let img_size = 72.0 * scale;
    let img_x = ox + 24.0 * scale;
    let img_y = oy + 24.0 * scale;
    
    let (palette, has_image) = if music_active {
        if let Some(image) = get_cached_media_image(media) {
            let mut img_paint = Paint::default();
            img_paint.set_anti_alias(true);
            img_paint.set_alpha_f(alpha as f32 / 255.0);
            canvas.save();
            canvas.clip_rrect(RRect::new_rect_xy(Rect::from_xywh(img_x, img_y, img_size, img_size), 14.0 * scale, 14.0 * scale), ClipOp::Intersect, true);
            canvas.draw_image_rect_with_sampling_options(
                &image, None, Rect::from_xywh(img_x, img_y, img_size, img_size),
                SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear), &img_paint
            );
            canvas.restore();
            (get_media_palette(media), true)
        } else {
            let mut paint = Paint::default();
            paint.set_anti_alias(true);
            paint.set_color(Color::from_argb((alpha as f32 * 0.1) as u8, 255, 255, 255));
            let rrect = RRect::new_rect_xy(Rect::from_xywh(img_x, img_y, img_size, img_size), 14.0 * scale, 14.0 * scale);
            canvas.draw_rrect(rrect, &paint);
            let cx = img_x + img_size / 2.0;
            let cy = img_y + img_size / 2.0;
            crate::icons::music::draw_music_icon(canvas, cx, cy, alpha, scale * 1.8);
            (vec![Color::from_rgb(180, 180, 180), Color::from_rgb(100, 100, 100)], false)
        }
    } else {
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_color(Color::from_argb((alpha as f32 * 0.1) as u8, 255, 255, 255));
        let rrect = RRect::new_rect_xy(Rect::from_xywh(img_x, img_y, img_size, img_size), 14.0 * scale, 14.0 * scale);
        canvas.draw_rrect(rrect, &paint);
        let cx = img_x + img_size / 2.0;
        let cy = img_y + img_size / 2.0;
        crate::icons::music::draw_music_icon(canvas, cx, cy, alpha, scale * 1.8);
        (vec![Color::from_rgb(180, 180, 180), Color::from_rgb(100, 100, 100)], false)
    };
    
    let text_x = img_x + img_size + 16.0 * scale;
    let max_text_w = w - (text_x - ox) - 100.0 * scale;
    let title_y = img_y + 26.0 * scale;
    
    let mut text_paint = Paint::default();
    text_paint.set_anti_alias(true);
    text_paint.set_color(Color::from_argb(alpha, 255, 255, 255));
    
    let title = if music_active && !media.title.is_empty() { &media.title } else { "No Music playing" };
    draw_text_cached(canvas, title, (text_x, title_y), 15.0 * scale, skia_safe::FontStyle::bold(), &text_paint, false, max_text_w);
    
    text_paint.set_color(Color::from_argb((alpha as f32 * 0.6) as u8, 255, 255, 255));
    let artist = if music_active && !media.artist.is_empty() { &media.artist } else { "Unknown Artist" };
    draw_text_cached(canvas, artist, (text_x, title_y + 22.0 * scale), 15.0 * scale, skia_safe::FontStyle::normal(), &text_paint, false, max_text_w);
    
    let viz_x_offset = 17.0 + (45.0 - 17.0) * expansion_progress;
    let _ = has_image;
    draw_visualizer(canvas, ox + w - viz_x_offset * scale, title_y - 4.0 * scale, alpha, music_active && media.is_playing, &palette, &media.spectrum, scale, viz_h_scale, (0.6, 0.08));
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
    current_lyric: &str,
    old_lyric: &str,
    lyric_transition: f32,
    use_blur: bool,
    hide_progress: f32,
    notification: Option<&NotificationInfo>,
    notification_active: bool,
    notification_transition: f32,
    show_app_name: bool,
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

        let page_shift = view_offset * current_w;

        if notification_active && notification.is_some() {
            let notif = notification.unwrap();
            let notif_height = 120.0 * global_scale;
            
            let music_phase = (notification_transition * 2.0).min(1.0);
            let notif_phase = ((notification_transition - 0.35) * 1.54).max(0.0).min(1.0);
            
            let music_ease = ease_out_back(music_phase);
            let notif_ease = spring_ease(notif_phase);
            
            let music_slide = notif_height * music_ease;
            let music_y = offset_y + music_slide;
            
            let notif_alpha = (alpha as f32 * notif_ease) as u8;
            draw_notification_expanded(canvas, offset_x, offset_y, current_w, notif_alpha, notif, global_scale, show_app_name);
            
            let divider_y = music_y;
            let divider_alpha = (alpha as f32 * 0.2 * notif_ease) as u8;
            let mut divider_paint = Paint::default();
            divider_paint.set_anti_alias(true);
            divider_paint.set_color(Color::from_argb(divider_alpha, 128, 128, 128));
            canvas.draw_rect(Rect::from_xywh(offset_x + 20.0 * global_scale, divider_y, current_w - 40.0 * global_scale, 1.0 * global_scale), &divider_paint);
            
            draw_music_section(
                canvas, offset_x, music_y, current_w, current_h - music_slide,
                alpha, media, music_active, global_scale, expansion_progress, viz_h_scale * global_scale
            );
        } else {
            canvas.save();
            canvas.translate((-page_shift, 0.0));
            draw_main_page(canvas, offset_x, offset_y, current_w, current_h, alpha, media, music_active, view_offset, global_scale, expansion_progress, viz_h_scale * global_scale);
            canvas.restore();

            canvas.save();
            canvas.translate((current_w - page_shift, 0.0));
            draw_widget_page(canvas, offset_x, offset_y, current_w, current_h, alpha, global_scale);
            canvas.restore();
        }

        if blur_filter.is_some() { canvas.restore(); }
        canvas.restore();
    }
    
    let mini_alpha = (mini_alpha_f * 255.0) as u8;
    
    if mini_alpha_f > 0.01 && current_w > 45.0 * global_scale {
        if notification_active && notification.is_some() {
            let notif = notification.unwrap();
            let alpha = mini_alpha;
            
            let text_x = offset_x + 10.0 * global_scale;
            let max_text_w = current_w - 20.0 * global_scale;
            
            let mut text_paint = Paint::default();
            text_paint.set_anti_alias(true);
            text_paint.set_color(Color::from_argb(alpha, 255, 255, 255));
            
            let text_y = offset_y + current_h / 2.0 + 4.0 * global_scale;
            draw_text_cached(canvas, &notif.app_name, (text_x, text_y), 12.0 * global_scale, skia_safe::FontStyle::bold(), &text_paint, false, max_text_w);
        } else if music_active {
            let alpha = mini_alpha;
            let img_size = 18.0 * global_scale;
            let img_x = offset_x + 8.0 * global_scale;
            let img_y = offset_y + (current_h - img_size) / 2.0;
            
            if let Some(image) = get_cached_media_image(media) {
                let mut paint = Paint::default();
                paint.set_anti_alias(true);
                paint.set_alpha_f(alpha as f32 / 255.0);
                canvas.save();
                canvas.clip_rrect(RRect::new_rect_xy(Rect::from_xywh(img_x, img_y, img_size, img_size), 5.0 * global_scale, 5.0 * global_scale), ClipOp::Intersect, true);
                let sampling = SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear);
                canvas.draw_image_rect_with_sampling_options(&image, None, Rect::from_xywh(img_x, img_y, img_size, img_size), sampling, &paint);
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
                let lyric_alpha = (alpha as f32 * lyric_fade_f) as u8;

                if lyric_alpha > 0 {
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
                            let fade_alpha = (lyric_alpha as f32 * (1.0 - lyric_transition)) as u8;
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
                            let fade_alpha = (lyric_alpha as f32 * lyric_transition) as u8;
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
                            let fade_alpha = (lyric_alpha as f32 * (1.0 - progress)) as u8;
                            text_paint.set_color(Color::from_argb(fade_alpha, 255, 255, 255));
                            draw_text_cached(canvas, old_lyric, (text_x, text_y), 12.0 * global_scale, skia_safe::FontStyle::normal(), &text_paint, true, available_w);
                        } else if lyric_transition >= 0.5 && !current_lyric.is_empty() {
                            let mut text_paint = Paint::default();
                            text_paint.set_anti_alias(true);
                            let progress = (lyric_transition - 0.5) * 2.0;
                            let fade_alpha = (lyric_alpha as f32 * progress) as u8;
                            text_paint.set_color(Color::from_argb(fade_alpha, 255, 255, 255));
                            draw_text_cached(canvas, current_lyric, (text_x, text_y), 12.0 * global_scale, skia_safe::FontStyle::normal(), &text_paint, true, available_w);
                        }
                    }
                    canvas.restore();
                }
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
