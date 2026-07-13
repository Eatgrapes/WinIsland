use crate::core::config::{DockPosition, PADDING, TOP_OFFSET, WidgetSlot};
use crate::core::smtc::MediaInfo;
use crate::ui::expanded::music_view::{
    DrawMusicPageParams, DrawVisualizerParams, draw_music_page, draw_text_cached, draw_visualizer,
    get_cached_media_image, get_media_palette,
};
use crate::ui::expanded::widget_view::draw_widget_page;
use crate::utils::backdrop::get_mica_background;
use crate::utils::font::{DrawTextCachedParams, FontManager};
use crate::utils::glass::get_glass_background;
use skia_safe::canvas::SrcRectConstraint;
use skia_safe::{
    ClipOp, Color, FilterMode, ISize, MipmapMode, Paint, RRect, Rect, SamplingOptions,
    Surface as SkSurface, image_filters, surfaces,
};
use softbuffer::Surface;
use std::cell::RefCell;
use std::sync::Arc;
use winit::window::Window;

thread_local! {
    static SK_SURFACE: RefCell<Option<SkSurface>> = const { RefCell::new(None) };
    static MINI_COVER_ROTATION: RefCell<f32> = const { RefCell::new(0.0) };

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
    pub base_h: f32,
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
    pub monitor_x: i32,
    pub monitor_y: i32,
    pub monitor_w: u32,
    pub monitor_h: u32,
}

#[allow(dead_code)]
pub struct StyleParams<'a> {
    pub island_style: &'a str,
    pub use_blur: bool,
    pub font_size: f32,
    pub weights: [f32; 4],
    pub mini_cover_shape: &'a str,
    pub expanded_cover_shape: &'a str,
    pub cover_rotate: bool,

    pub lyrics_delay: f64,
    pub dt: f32,
    pub widget_layout: &'a [WidgetSlot],
}

use crate::core::context::MiniContent;

pub struct DrawIslandParams<'a> {
    pub layout: LayoutParams,
    pub media: MediaParams<'a>,
    pub lyrics: LyricsParams<'a>,
    pub mini_content: Option<MiniContent>,
    pub window: WindowParams,
    pub style: StyleParams<'a>,
}

pub fn draw_island(
    surface: &mut Surface<Arc<Window>, Arc<Window>>,
    params: DrawIslandParams<'_>,
) -> bool {
    let DrawIslandParams {
        layout,
        media,
        lyrics,
        mini_content,
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
        base_h,
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
    let WindowParams {
        win_x,
        win_y,
        monitor_x,
        monitor_y,
        monitor_w,
        monitor_h,
    } = window;
    let StyleParams {
        island_style,
        use_blur,
        font_size,
        weights: _weights,
        mini_cover_shape: _,
        expanded_cover_shape: _,
        cover_rotate: _,

        lyrics_delay,
        dt,
        widget_layout,
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

    let stable_base_y = if dock_bottom {
        os_h as f32 - PADDING / 2.0 - base_h
    } else {
        PADDING / 2.0
    };
    let stable_offset_y = if dock_bottom {
        stable_base_y + hide_y_offset
    } else {
        stable_base_y - hide_y_offset
    };

    let rect = Rect::from_xywh(offset_x, offset_y, current_w, current_h);
    let rrect = RRect::new_rect_xy(rect, current_r, current_r);
    let has_blur = sigmas.0 > 0.1 || sigmas.1 > 0.1;
    let blur_filter = if has_blur {
        image_filters::blur(sigmas, None, None, None)
    } else {
        None
    };

    let bg_color = Color::BLACK;

    let text_color = Color::WHITE;
    let text_color_sec = Color::WHITE;

    if island_style == "glass" {
        let screen_x = win_x + offset_x as i32;
        let screen_y = win_y + offset_y as i32;
        canvas.save();
        canvas.clip_rrect(rrect, ClipOp::Intersect, true);
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

            let mut darken = Paint::default();
            darken.set_color(Color::from_argb(130, 10, 10, 14));
            darken.set_anti_alias(true);
            darken.set_blend_mode(skia_safe::BlendMode::Multiply);
            canvas.draw_rect(rect, &darken);
        } else {
            let mut bg_paint = Paint::default();
            bg_paint.set_color(Color::from_argb(205, 32, 32, 36));
            bg_paint.set_anti_alias(true);
            canvas.draw_rrect(rrect, &bg_paint);
        }
    } else if island_style == "mica" {
        let screen_x = win_x + offset_x as i32;
        let screen_y = win_y + offset_y as i32;
        canvas.save();
        canvas.clip_rrect(rrect, ClipOp::Intersect, true);
        if let Some(bg_img) = get_mica_background(
            screen_x,
            screen_y,
            current_w as u32,
            current_h as u32,
            monitor_x,
            monitor_y,
            monitor_w,
            monitor_h,
        ) {
            let mut paint = Paint::default();
            paint.set_anti_alias(true);
            let sampling = SamplingOptions::new(FilterMode::Linear, MipmapMode::None);
            canvas.draw_image_rect_with_sampling_options(&bg_img, None, rect, sampling, &paint);

            let mut overlay = Paint::default();
            overlay.set_color(Color::from_argb(110, 32, 32, 32));
            overlay.set_anti_alias(true);
            canvas.draw_rrect(rrect, &overlay);
        } else {
            let mut bg_paint = Paint::default();
            bg_paint.set_color(Color::from_argb(205, 32, 32, 36));
            bg_paint.set_anti_alias(true);
            canvas.draw_rrect(rrect, &bg_paint);
        }
    } else if island_style == "dynamic" {
        canvas.save();
        canvas.clip_rrect(rrect, ClipOp::Intersect, true);
        if let Some(blurred_cover) = crate::utils::backdrop::get_blurred_cover_background(media) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();

            // Slow rotation: 1 full rotation every 60 seconds
            let angle_rad = (now * 0.03) % (2.0 * std::f64::consts::PI);
            let angle_deg = angle_rad * 180.0 / std::f64::consts::PI;

            // Slow drift offsets
            let dx = (now * 0.15).sin() * 20.0;
            let dy = (now * 0.12).cos() * 15.0;

            // Center of the island
            let cx = rect.left() + rect.width() / 2.0;
            let cy = rect.top() + rect.height() / 2.0;

            // Calculate diagonal of the island
            let diagonal = (rect.width() * rect.width() + rect.height() * rect.height()).sqrt();
            // Scale the side length of the square to accommodate rotation and drift
            let side_len = diagonal * 1.3f32;

            canvas.save();
            canvas.translate((cx + dx as f32, cy + dy as f32));
            canvas.rotate(angle_deg as f32, None);

            let draw_rect = Rect::from_xywh(-side_len / 2.0, -side_len / 2.0, side_len, side_len);

            let mut paint = Paint::default();
            paint.set_anti_alias(true);
            let sampling = SamplingOptions::new(FilterMode::Linear, MipmapMode::None);
            canvas.draw_image_rect_with_sampling_options(
                &blurred_cover,
                None,
                draw_rect,
                sampling,
                &paint,
            );
            canvas.restore();

            let mut overlay = Paint::default();
            overlay.set_color(Color::from_argb(120, 20, 20, 24));
            overlay.set_anti_alias(true);
            canvas.draw_rect(rect, &overlay);
        } else {
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

                let mut darken = Paint::default();
                darken.set_color(Color::from_argb(130, 10, 10, 14));
                darken.set_anti_alias(true);
                darken.set_blend_mode(skia_safe::BlendMode::Multiply);
                canvas.draw_rect(rect, &darken);
            } else {
                let mut bg_paint = Paint::default();
                bg_paint.set_color(Color::from_argb(205, 32, 32, 36));
                bg_paint.set_anti_alias(true);
                canvas.draw_rrect(rrect, &bg_paint);
            }
        }
    } else {
        canvas.save();
        canvas.clip_rrect(rrect, ClipOp::Intersect, true);
        let mut bg_paint = Paint::default();
        bg_paint.set_color(bg_color);
        bg_paint.set_anti_alias(true);
        canvas.draw_rrect(rrect, &bg_paint);
    }

    let expanded_alpha_f = (expansion_progress.powf(2.0)).clamp(0.0, 1.0) * (1.0 - hide_progress);
    let mini_alpha_f = (1.0 - expansion_progress * 1.5).clamp(0.0, 1.0) * (1.0 - hide_progress);

    let palette = if expanded_alpha_f > 0.01 || mini_alpha_f > 0.01 {
        get_media_palette(media)
    } else {
        vec![
            Color::from_rgb(180, 180, 180),
            Color::from_rgb(100, 100, 100),
        ]
    };

    let viz_h_scale = 0.45 + (1.0 - 0.45) * expansion_progress;

    let mut widget_animating = false;
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
        let _ = draw_music_page(DrawMusicPageParams {
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
            cover_shape: "square",
            cover_rotate: false,
            dt,
            text_color,
            text_color_sec,
            palette: palette.clone(),
        });
        canvas.restore();

        canvas.save();
        canvas.translate((current_w - page_shift, 0.0));
        let widget_anim = draw_widget_page(
            canvas,
            offset_x,
            offset_y,
            current_w,
            current_h,
            alpha,
            global_scale,
            media,
            font_size,
            lyrics_delay,
            dt,
            widget_layout,
            text_color,
        );
        canvas.restore();

        widget_animating = widget_anim;

        if blur_filter.is_some() {
            canvas.restore();
        }
        canvas.restore();
    }
    if mini_alpha_f > 0.01 && current_w > 45.0 * global_scale {
        match &mini_content {
            Some(MiniContent::Music) => {
                let alpha = (mini_alpha_f * 255.0) as u8;
                if let Some(image) = get_cached_media_image(media) {
                    let base_size = 18.0 * global_scale;
                    let (size, ix, iy) = (
                        base_size,
                        offset_x + 10.0 * global_scale,
                        stable_offset_y + (base_h - base_size) / 2.0,
                    );
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
                    let img_w = image.width() as f32;
                    let img_h = image.height() as f32;
                    let src_rect = if img_w > 0.0 && img_h > 0.0 {
                        let aspect = img_w / img_h;
                        let src = if aspect > 1.0 {
                            let crop_w = img_h;
                            let offset_x = (img_w - crop_w) / 2.0;
                            Rect::from_xywh(offset_x, 0.0, crop_w, img_h)
                        } else {
                            let crop_h = img_w;
                            let offset_y = (img_h - crop_h) / 2.0;
                            Rect::from_xywh(0.0, offset_y, img_w, crop_h)
                        };
                        Some(src)
                    } else {
                        None
                    };
                    canvas.draw_image_rect_with_sampling_options(
                        &image,
                        src_rect.as_ref().map(|r| (r, SrcRectConstraint::Fast)),
                        Rect::from_xywh(ix, iy, size, size),
                        sampling,
                        &paint,
                    );
                    canvas.restore();
                }
                let palette = &palette;
                let viz_x = offset_x + current_w - 17.0 * global_scale;
                let viz_y = stable_offset_y + base_h / 2.0;
                draw_visualizer(DrawVisualizerParams {
                    canvas,
                    x: viz_x,
                    y: viz_y,
                    alpha,
                    is_playing: media.is_playing,
                    palette,
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

                        canvas.save();
                        let clip_rect =
                            Rect::from_xywh(space_left, stable_offset_y, available_w, base_h);
                        canvas.clip_rect(clip_rect, ClipOp::Intersect, true);

                        if use_blur {
                            if lyric_transition < 1.0 && !old_lyric.is_empty() {
                                let mut text_paint = Paint::default();
                                text_paint.set_anti_alias(true);
                                let fade_alpha = (alpha as f32 * (1.0 - lyric_transition)) as u8;
                                text_paint.set_color(Color::from_argb(
                                    fade_alpha,
                                    text_color.r(),
                                    text_color.g(),
                                    text_color.b(),
                                ));

                                let blur_sigma = lyric_transition * 12.0 * global_scale;
                                if blur_sigma > 0.1 {
                                    text_paint.set_image_filter(image_filters::blur(
                                        (blur_sigma, 0.0),
                                        None,
                                        None,
                                        None,
                                    ));
                                }

                                let text_y = stable_offset_y + base_h / 2.0 + 4.0 * global_scale
                                    - (10.0 * global_scale * lyric_transition);
                                let old_lx = if text_centered {
                                    let w = FontManager::global().measure_text_cached(
                                        old_lyric,
                                        lyric_font_sz,
                                        skia_safe::FontStyle::normal(),
                                    );
                                    text_x - w / 2.0
                                } else {
                                    text_x
                                };
                                draw_text_cached(DrawTextCachedParams {
                                    canvas,
                                    text: old_lyric,
                                    x: old_lx,
                                    y: text_y,
                                    size: lyric_font_sz,
                                    bold: false,
                                    paint: &text_paint,
                                });
                            }

                            if !current_lyric.is_empty() {
                                let mut text_paint = Paint::default();
                                text_paint.set_anti_alias(true);
                                let fade_alpha = (alpha as f32 * lyric_transition) as u8;
                                text_paint.set_color(Color::from_argb(
                                    fade_alpha,
                                    text_color.r(),
                                    text_color.g(),
                                    text_color.b(),
                                ));

                                let blur_sigma = (1.0 - lyric_transition) * 12.0 * global_scale;
                                if blur_sigma > 0.1 {
                                    text_paint.set_image_filter(image_filters::blur(
                                        (blur_sigma, 0.0),
                                        None,
                                        None,
                                        None,
                                    ));
                                }

                                let text_y = stable_offset_y
                                    + base_h / 2.0
                                    + 4.0 * global_scale
                                    + (10.0 * global_scale * (1.0 - lyric_transition));
                                let cur_lx = if text_centered {
                                    let w = FontManager::global().measure_text_cached(
                                        current_lyric,
                                        lyric_font_sz,
                                        skia_safe::FontStyle::normal(),
                                    );
                                    text_x - w / 2.0
                                } else {
                                    text_x
                                };
                                draw_text_cached(DrawTextCachedParams {
                                    canvas,
                                    text: current_lyric,
                                    x: cur_lx,
                                    y: text_y,
                                    size: lyric_font_sz,
                                    bold: false,
                                    paint: &text_paint,
                                });
                            }
                        } else {
                            let text_y = stable_offset_y + base_h / 2.0 + 4.0 * global_scale;
                            if lyric_transition < 0.5 && !old_lyric.is_empty() {
                                let mut text_paint = Paint::default();
                                text_paint.set_anti_alias(true);
                                let progress = lyric_transition * 2.0;
                                let fade_alpha = (alpha as f32 * (1.0 - progress)) as u8;
                                text_paint.set_color(Color::from_argb(
                                    fade_alpha,
                                    text_color.r(),
                                    text_color.g(),
                                    text_color.b(),
                                ));
                                let old_lx2 = if text_centered {
                                    let w = FontManager::global().measure_text_cached(
                                        old_lyric,
                                        lyric_font_sz,
                                        skia_safe::FontStyle::normal(),
                                    );
                                    text_x - w / 2.0
                                } else {
                                    text_x
                                };
                                draw_text_cached(DrawTextCachedParams {
                                    canvas,
                                    text: old_lyric,
                                    x: old_lx2,
                                    y: text_y,
                                    size: lyric_font_sz,
                                    bold: false,
                                    paint: &text_paint,
                                });
                            } else if lyric_transition >= 0.5 && !current_lyric.is_empty() {
                                let mut text_paint = Paint::default();
                                text_paint.set_anti_alias(true);
                                let progress = (lyric_transition - 0.5) * 2.0;
                                let fade_alpha = (alpha as f32 * progress) as u8;
                                text_paint.set_color(Color::from_argb(
                                    fade_alpha,
                                    text_color.r(),
                                    text_color.g(),
                                    text_color.b(),
                                ));
                                let cur_lx2 = if text_centered {
                                    let w = FontManager::global().measure_text_cached(
                                        current_lyric,
                                        lyric_font_sz,
                                        skia_safe::FontStyle::normal(),
                                    );
                                    text_x - w / 2.0
                                } else {
                                    text_x
                                };
                                draw_text_cached(DrawTextCachedParams {
                                    canvas,
                                    text: current_lyric,
                                    x: cur_lx2,
                                    y: text_y,
                                    size: lyric_font_sz,
                                    bold: false,
                                    paint: &text_paint,
                                });
                            }
                        }
                        canvas.restore();
                    }
                }
            }
            Some(MiniContent::Plugin(ctx)) => {
                let font_sz = if font_size > 0.0 {
                    font_size * 0.7 * global_scale
                } else {
                    11.0 * global_scale
                };
                let alpha = (mini_alpha_f * 255.0) as u8;
                let mut text_paint = Paint::default();
                text_paint.set_anti_alias(true);
                text_paint.set_color(Color::from_argb(
                    alpha,
                    text_color.r(),
                    text_color.g(),
                    text_color.b(),
                ));
                let text_x = offset_x + 20.0 * global_scale;
                let text_w = current_w - 40.0 * global_scale;
                let text_y = stable_offset_y + base_h / 2.0 - font_sz * 0.3;
                canvas.save();
                let clip = Rect::from_xywh(text_x, stable_offset_y, text_w, base_h);
                canvas.clip_rect(clip, ClipOp::Intersect, true);
                draw_text_cached(DrawTextCachedParams {
                    canvas,
                    text: &ctx.title,
                    x: text_x,
                    y: text_y,
                    size: font_sz,
                    bold: true,
                    paint: &text_paint,
                });
                if !ctx.body.is_empty() {
                    let sec_font_sz = font_sz * 0.8;
                    let mut sec_paint = Paint::default();
                    sec_paint.set_anti_alias(true);
                    sec_paint.set_color(Color::from_argb(
                        (alpha as f32 * 0.7) as u8,
                        text_color.r(),
                        text_color.g(),
                        text_color.b(),
                    ));
                    let sec_y = text_y + font_sz * 1.3;
                    draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: &ctx.body,
                        x: text_x,
                        y: sec_y,
                        size: sec_font_sz,
                        bold: false,
                        paint: &sec_paint,
                    });
                }
                canvas.restore();
            }
            None => {}
        }
    }
    canvas.restore();

    {
        let mut border_paint = Paint::default();
        border_paint.set_anti_alias(true);
        border_paint.set_style(skia_safe::PaintStyle::Stroke);
        border_paint.set_stroke_width(1.0);
        if island_style == "default" {
            border_paint.set_color(Color::from_argb(30, 255, 255, 255));
        } else {
            border_paint.set_color(Color::from_argb(40, 255, 255, 255));
        }
        let border_rrect = RRect::new_rect_xy(
            Rect::from_xywh(
                offset_x + 0.5,
                offset_y + 0.5,
                current_w - 1.0,
                current_h - 1.0,
            ),
            current_r,
            current_r,
        );
        canvas.draw_rrect(border_rrect, &border_paint);
    }

    let info = skia_safe::ImageInfo::new(
        skia_safe::ISize::new(os_w as i32, os_h as i32),
        skia_safe::ColorType::BGRA8888,
        skia_safe::AlphaType::Premul,
        None,
    );
    let dst_row_bytes = (os_w * 4) as usize;
    let u8_buffer: &mut [u8] = bytemuck::cast_slice_mut(&mut buffer);
    let _ = sk_surface.read_pixels(&info, u8_buffer, dst_row_bytes, (0, 0));
    if let Err(e) = buffer.present() {
        log::error!("Present failed: {:?}", e);
    }

    widget_animating
}
