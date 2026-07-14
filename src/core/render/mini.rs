use skia_safe::canvas::SrcRectConstraint;
use skia_safe::{
    Canvas, ClipOp, Color, FilterMode, MipmapMode, Paint, RRect, Rect, SamplingOptions,
    image_filters,
};

use crate::core::context::MiniContent;
use crate::core::smtc::MediaInfo;
use crate::ui::expanded::music_view::{
    DrawVisualizerParams, draw_text_cached, draw_visualizer, get_cached_media_image,
};
use crate::utils::font::{DrawTextCachedParams, FontManager};

pub(super) struct MiniContentParams<'a> {
    pub(super) canvas: &'a Canvas,
    pub(super) content: Option<MiniContent<'a>>,
    pub(super) mini_alpha: f32,
    pub(super) current_w: f32,
    pub(super) global_scale: f32,
    pub(super) media: &'a MediaInfo,
    pub(super) offset_x: f32,
    pub(super) stable_offset_y: f32,
    pub(super) base_h: f32,
    pub(super) palette: &'a [Color],
    pub(super) viz_h_scale: f32,
    pub(super) current_lyric: &'a str,
    pub(super) old_lyric: &'a str,
    pub(super) expansion_progress: f32,
    pub(super) font_size: f32,
    pub(super) lyric_scroll_offset: f32,
    pub(super) use_blur: bool,
    pub(super) lyric_transition: f32,
    pub(super) text_color: Color,
}

pub(super) fn draw_mini_content(params: MiniContentParams<'_>) {
    let MiniContentParams {
        canvas,
        content: mini_content,
        mini_alpha: mini_alpha_f,
        current_w,
        global_scale,
        media,
        offset_x,
        stable_offset_y,
        base_h,
        palette,
        viz_h_scale,
        current_lyric,
        old_lyric,
        expansion_progress,
        font_size,
        lyric_scroll_offset,
        use_blur,
        lyric_transition,
        text_color,
    } = params;
    if mini_alpha_f > 0.01 && current_w > 45.0 * global_scale {
        match mini_content {
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
}
