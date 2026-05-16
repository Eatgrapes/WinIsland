use crate::core::config::{DockPosition, PADDING, TOP_OFFSET};
use crate::core::smtc::MediaInfo;
use crate::ui::expanded::main_view::{
    draw_main_page, draw_text_cached, draw_visualizer, get_cached_media_image, get_media_palette,
    DrawMainPageParams, DrawVisualizerParams,
};
use crate::ui::expanded::widget_view::draw_widget_page;
use crate::utils::font::DrawTextCachedParams;
use crate::utils::glass::get_glass_background;
use skia_safe::{
    image_filters, surfaces, ClipOp, Color, FilterMode, ISize, MipmapMode, Paint, RRect,
    Rect, SamplingOptions, Surface as SkSurface,
};
use softbuffer::Surface;
use std::cell::RefCell;
use std::sync::Arc;
use winit::window::Window;

thread_local! {
    static SK_SURFACE: RefCell<Option<SkSurface>> = const { RefCell::new(None) };
}

pub struct LayoutParams {
    pub current_w: f32,
    pub current_h: f32,
    pub current_r: f32,
    pub os_w: u32,
    pub os_h: u32,
    pub sigmas: (f32, f32),
    pub expansion_progress: f32,
    pub view_offset: f32,
    pub global_scale: f32,
    pub hide_progress: f32,
    pub dock_position: DockPosition,
}

pub struct MediaParams<'a> {
    pub media: &'a MediaInfo,
    pub music_active: bool,
}

pub struct LyricsParams<'a> {
    pub current_lyric: &'a str,
    pub old_lyric: &'a str,
    pub lyric_transition: f32,
    pub lyric_scroll_offset: f32,
}

pub struct WindowParams {
    pub win_x: i32,
    pub win_y: i32,
}

pub struct StyleParams<'a> {
    pub island_style: &'a str,
    pub use_blur: bool,
    pub font_size: f32,
    pub weights: [f32; 4],
}

pub struct DrawIslandParams<'a> {
    pub layout: LayoutParams,
    pub media: MediaParams<'a>,
    pub lyrics: LyricsParams<'a>,
    pub window: WindowParams,
    pub style: StyleParams<'a>,
}

pub fn draw_island(surface: &mut Surface<Arc<Window>, Arc<Window>>, params: DrawIslandParams<'_>) {
    let DrawIslandParams {
        layout,
        media,
        lyrics,
        window,
        style,
    } = params;

    let LayoutParams {
        current_w,
        current_h,
        current_r,
        os_w,
        os_h,
        sigmas,
        expansion_progress,
        view_offset,
        global_scale,
        hide_progress,
        dock_position,
    } = layout;
    let MediaParams {
        media,
        music_active,
    } = media;
    let LyricsParams {
        current_lyric,
        old_lyric,
        lyric_transition,
        lyric_scroll_offset,
    } = lyrics;
    let WindowParams { win_x, win_y } = window;
    let StyleParams {
        island_style,
        use_blur,
        font_size,
        weights: _weights,
    } = style;

    let mut buffer = surface.buffer_mut().unwrap();
    let mut sk_surface = SK_SURFACE.with(|cell| {
        let mut opt = cell.borrow_mut();
        if let Some(ref s) = *opt
            && s.width() == os_w as i32
            && s.height() == os_h as i32
        {
            return s.clone();
        }
        let new_surface =
            surfaces::raster_n32_premul(ISize::new(os_w as i32, os_h as i32)).unwrap();
        *opt = Some(new_surface.clone());
        new_surface
    });
    let canvas = sk_surface.canvas();
    canvas.clear(Color::TRANSPARENT);

    let dock_bottom = dock_position.is_bottom();
    let offset_x = if dock_position.is_left() {
        PADDING / 2.0
    } else if dock_position.is_right() {
        (os_w as f32 - PADDING / 2.0 - current_w).max(0.0)
    } else {
        (os_w as f32 - current_w) / 2.0
    };
    let base_y = if dock_bottom {
        os_h as f32 - PADDING / 2.0 - current_h
    } else {
        PADDING / 2.0
    };
    let hidden_peek_h = (5.0 * global_scale).max(3.0);
    let hide_distance = if dock_bottom {
        (current_h - hidden_peek_h).max(0.0)
    } else {
        (current_h - hidden_peek_h + TOP_OFFSET as f32).max(0.0)
    };
    let hide_y_offset = hide_progress * hide_distance;
    let offset_y = if dock_bottom {
        base_y + hide_y_offset
    } else {
        base_y - hide_y_offset
    };

    let rect = Rect::from_xywh(offset_x, offset_y, current_w, current_h);
    let rrect = RRect::new_rect_xy(rect, current_r, current_r);
    let has_blur = sigmas.0 > 0.1 || sigmas.1 > 0.1;
    let blur_filter = if has_blur {
        image_filters::blur(sigmas, None, None, None)
    } else {
        None
    };
    canvas.save();
    canvas.clip_rrect(rrect, ClipOp::Intersect, true);

    if island_style == "glass" {
        let screen_x = win_x + offset_x as i32;
        let screen_y = win_y + offset_y as i32;
        if let Some(bg_img) = get_glass_background(
            screen_x,
            screen_y,
            current_w as u32,
            current_h as u32,
            40.0 * global_scale,
        ) {
            let mut paint = Paint::default();
            paint.set_anti_alias(true);
            let sampling = SamplingOptions::new(FilterMode::Linear, MipmapMode::None);
            canvas.draw_image_rect_with_sampling_options(&bg_img, None, rect, sampling, &paint);
        }
        let mut overlay = Paint::default();
        overlay.set_color(Color::from_argb(120, 0, 0, 0));
        overlay.set_anti_alias(true);
        canvas.draw_rrect(rrect, &overlay);
    } else {
        let mut bg_paint = Paint::default();
        bg_paint.set_color(Color::BLACK);
        bg_paint.set_anti_alias(true);
        canvas.draw_rrect(rrect, &bg_paint);
    }

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

        canvas.save();
        canvas.translate((-page_shift, 0.0));
        draw_main_page(DrawMainPageParams {
            canvas,
            ox: offset_x,
            oy: offset_y,
            w: current_w,
            h: current_h,
            alpha,
            media,
            music_active,
            view_offset,
            scale: global_scale,
            expansion_progress,
            viz_h_scale: viz_h_scale * global_scale,
            use_blur,
            font_size,
        });
        canvas.restore();

        canvas.save();
        canvas.translate((current_w - page_shift, 0.0));
        draw_widget_page(
            canvas,
            offset_x,
            offset_y,
            current_w,
            current_h,
            alpha,
            global_scale,
        );
        canvas.restore();

        if blur_filter.is_some() {
            canvas.restore();
        }
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
            canvas.clip_rrect(
                RRect::new_rect_xy(
                    Rect::from_xywh(ix, iy, size, size),
                    5.0 * global_scale,
                    5.0 * global_scale,
                ),
                ClipOp::Intersect,
                true,
            );
            let sampling = SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear);
            canvas.draw_image_rect_with_sampling_options(
                &image,
                None,
                Rect::from_xywh(ix, iy, size, size),
                sampling,
                &paint,
            );
            canvas.restore();
        }
        let palette = get_media_palette(media);
        let viz_x = offset_x + current_w - 17.0 * global_scale;
        let viz_y = offset_y + current_h / 2.0;
        draw_visualizer(DrawVisualizerParams {
            canvas,
            x: viz_x,
            y: viz_y,
            alpha,
            is_playing: media.is_playing,
            palette: &palette,
            spectrum: &media.spectrum,
            w_scale: 0.55 * global_scale,
            h_scale: viz_h_scale * global_scale,
            smooth_factors: (0.6, 0.08),
        });

        if !current_lyric.is_empty() || !old_lyric.is_empty() {
            let lyric_fade_f = (1.0 - expansion_progress * 2.5).clamp(0.0, 1.0);
            let alpha = (alpha as f32 * lyric_fade_f) as u8;

            if alpha > 0 {
                let lyric_font_sz = if font_size > 0.0 {
                    font_size * 0.8 * global_scale
                } else {
                    12.0 * global_scale
                };
                let space_left = offset_x + 30.0 * global_scale;
                let space_right = offset_x + current_w - 29.0 * global_scale;
                let available_w = space_right - space_left;
                let scrolling = lyric_scroll_offset > 0.0;
                let text_x = if scrolling {
                    space_left - lyric_scroll_offset
                } else {
                    space_left + available_w / 2.0
                };
                let text_centered = !scrolling;
                let text_max_w = if scrolling { 10000.0 } else { available_w };

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
                            text_paint.set_image_filter(image_filters::blur(
                                (blur_sigma, 0.0),
                                None,
                                None,
                                None,
                            ));
                        }

                        let text_y = offset_y + current_h / 2.0 + 4.0 * global_scale
                            - (10.0 * global_scale * lyric_transition);
                        draw_text_cached(DrawTextCachedParams {
                            canvas,
                            text: old_lyric,
                            pos: (text_x, text_y),
                            size: lyric_font_sz,
                            style: skia_safe::FontStyle::normal(),
                            paint: &text_paint,
                            align_center: text_centered,
                            max_w: text_max_w,
                        });
                    }

                    if !current_lyric.is_empty() {
                        let mut text_paint = Paint::default();
                        text_paint.set_anti_alias(true);
                        let fade_alpha = (alpha as f32 * lyric_transition) as u8;
                        text_paint.set_color(Color::from_argb(fade_alpha, 255, 255, 255));

                        let blur_sigma = (1.0 - lyric_transition) * 12.0 * global_scale;
                        if blur_sigma > 0.1 {
                            text_paint.set_image_filter(image_filters::blur(
                                (blur_sigma, 0.0),
                                None,
                                None,
                                None,
                            ));
                        }

                        let text_y = offset_y
                            + current_h / 2.0
                            + 4.0 * global_scale
                            + (10.0 * global_scale * (1.0 - lyric_transition));
                        draw_text_cached(DrawTextCachedParams {
                            canvas,
                            text: current_lyric,
                            pos: (text_x, text_y),
                            size: lyric_font_sz,
                            style: skia_safe::FontStyle::normal(),
                            paint: &text_paint,
                            align_center: text_centered,
                            max_w: text_max_w,
                        });
                    }
                } else {
                    let text_y = offset_y + current_h / 2.0 + 4.0 * global_scale;
                    if lyric_transition < 0.5 && !old_lyric.is_empty() {
                        let mut text_paint = Paint::default();
                        text_paint.set_anti_alias(true);
                        let progress = lyric_transition * 2.0;
                        let fade_alpha = (alpha as f32 * (1.0 - progress)) as u8;
                        text_paint.set_color(Color::from_argb(fade_alpha, 255, 255, 255));
                        draw_text_cached(DrawTextCachedParams {
                            canvas,
                            text: old_lyric,
                            pos: (text_x, text_y),
                            size: lyric_font_sz,
                            style: skia_safe::FontStyle::normal(),
                            paint: &text_paint,
                            align_center: text_centered,
                            max_w: text_max_w,
                        });
                    } else if lyric_transition >= 0.5 && !current_lyric.is_empty() {
                        let mut text_paint = Paint::default();
                        text_paint.set_anti_alias(true);
                        let progress = (lyric_transition - 0.5) * 2.0;
                        let fade_alpha = (alpha as f32 * progress) as u8;
                        text_paint.set_color(Color::from_argb(fade_alpha, 255, 255, 255));
                        draw_text_cached(DrawTextCachedParams {
                            canvas,
                            text: current_lyric,
                            pos: (text_x, text_y),
                            size: lyric_font_sz,
                            style: skia_safe::FontStyle::normal(),
                            paint: &text_paint,
                            align_center: text_centered,
                            max_w: text_max_w,
                        });
                    }
                }
                canvas.restore();
            }
        }
    }
    canvas.restore();
    let info = skia_safe::ImageInfo::new(
        skia_safe::ISize::new(os_w as i32, os_h as i32),
        skia_safe::ColorType::BGRA8888,
        skia_safe::AlphaType::Premul,
        None,
    );
    let dst_row_bytes = (os_w * 4) as usize;
    let u8_buffer: &mut [u8] = bytemuck::cast_slice_mut(&mut buffer);
    let _ = sk_surface.read_pixels(&info, u8_buffer, dst_row_bytes, (0, 0));
    buffer.present().unwrap();
}
