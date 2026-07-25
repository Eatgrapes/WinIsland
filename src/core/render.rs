mod background;
mod codex;
mod expanded;
mod mini;

use self::background::{BackgroundParams, draw_background};
use self::codex::{DrawCodexCompactParams, DrawCodexExpandedParams, draw_compact, draw_expanded};
use self::expanded::{ExpandedContentParams, draw_expanded_content};
use self::mini::{MiniContentParams, draw_mini_content};

pub use self::codex::{
    compact_size as codex_compact_size, expanded_header_height as codex_expanded_header_height,
    expanded_metrics as codex_expanded_metrics,
};

use crate::core::codex::{CodexPet, CodexSnapshot};
use crate::core::config::WidgetSlot;
use crate::core::smtc::MediaInfo;
use crate::ui::compact::CompactOverlay;
use crate::ui::expanded::music_view::{default_media_palette, get_media_palette};
use skia_safe::{ClipOp, Color, Paint, RRect, Rect, Surface, gpu::DirectContext, image_filters};

pub struct LayoutParams {
    pub current_w: f32,
    pub current_h: f32,
    pub current_r: f32,
    pub sigmas: (f32, f32),
    pub expansion_progress: f32,
    pub view_offset: f32,
    pub global_scale: f32,
    pub hide_progress: f32,
    pub island_x: f32,
    pub island_y: f32,
    pub stable_island_y: f32,
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

pub struct StyleParams<'a> {
    pub island_style: &'a str,
    pub use_blur: bool,
    pub font_size: f32,
    pub lyrics_delay: f64,
    pub dt: f32,
    pub widget_layout: &'a [WidgetSlot],
}

use crate::core::context::MiniContent;

pub struct CodexIslandParams<'a> {
    pub snapshot: &'a CodexSnapshot,
    pub pet: Option<&'a CodexPet>,
    pub expanded: bool,
    pub scroll_offset: f32,
    pub animation_time_secs: f32,
}

pub struct DrawIslandParams<'a> {
    pub layout: LayoutParams,
    pub media: MediaParams<'a>,
    pub lyrics: LyricsParams<'a>,
    pub mini_content: Option<MiniContent<'a>>,
    pub compact_overlay: &'a CompactOverlay,
    pub codex: Option<CodexIslandParams<'a>>,
    pub window: WindowParams,
    pub style: StyleParams<'a>,
}

pub fn draw_island(
    direct_context: &mut DirectContext,
    surface: &mut Surface,
    params: DrawIslandParams<'_>,
) -> bool {
    let DrawIslandParams {
        layout,
        media,
        lyrics,
        mini_content,
        compact_overlay,
        codex,
        window,
        style,
    } = params;

    let LayoutParams {
        current_w,
        current_h,
        current_r,
        sigmas,
        expansion_progress,
        view_offset,
        global_scale,
        hide_progress,
        island_x,
        island_y,
        stable_island_y,
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
        lyrics_delay,
        dt,
        widget_layout,
    } = style;
    let canvas = surface.canvas();
    canvas.clear(Color::TRANSPARENT);

    let offset_x = island_x;
    let offset_y = island_y;
    let stable_offset_y = stable_island_y;

    let rect = Rect::from_xywh(offset_x, offset_y, current_w, current_h);
    let rrect = RRect::new_rect_xy(rect, current_r, current_r);
    let has_blur = sigmas.0 > 0.1 || sigmas.1 > 0.1;
    let blur_filter = if has_blur {
        image_filters::blur(sigmas, None, None, None)
    } else {
        None
    };

    let text_color = Color::WHITE;
    let text_color_sec = Color::WHITE;

    draw_background(BackgroundParams {
        canvas,
        direct_context,
        rect,
        rrect,
        island_style,
        media,
        win_x,
        win_y,
        offset_x,
        offset_y,
        current_w,
        current_h,
        global_scale,
        monitor_x,
        monitor_y,
        monitor_w,
        monitor_h,
    });
    canvas.save();
    canvas.clip_rrect(rrect, ClipOp::Intersect, true);

    let compact_overlay_visible = compact_overlay.is_visible();
    let expanded_alpha_f = if compact_overlay_visible {
        0.0
    } else {
        (expansion_progress.powf(2.0)).clamp(0.0, 1.0) * (1.0 - hide_progress)
    };
    let mini_alpha_f = if compact_overlay_visible {
        0.0
    } else {
        (1.0 - expansion_progress * 1.5).clamp(0.0, 1.0) * (1.0 - hide_progress)
    };

    let palette = if expanded_alpha_f > 0.01 || mini_alpha_f > 0.01 {
        get_media_palette(direct_context, media)
    } else {
        default_media_palette()
    };

    let viz_h_scale = 0.45 + (1.0 - 0.45) * expansion_progress;

    let mut animating = false;
    if compact_overlay_visible {
        compact_overlay.draw(canvas, rect, global_scale, 1.0 - hide_progress);
    } else if let Some(codex) = codex {
        animating = if codex.expanded {
            draw_expanded(DrawCodexExpandedParams {
                canvas,
                snapshot: codex.snapshot,
                pet: codex.pet,
                x: offset_x,
                y: offset_y,
                width: current_w,
                height: current_h,
                scale: global_scale,
                font_size,
                animation_time_secs: codex.animation_time_secs,
                scroll_offset: codex.scroll_offset,
                alpha: expanded_alpha_f,
            })
        } else {
            draw_compact(DrawCodexCompactParams {
                canvas,
                snapshot: codex.snapshot,
                pet: codex.pet,
                x: offset_x,
                y: offset_y,
                width: current_w,
                height: current_h,
                scale: global_scale,
                font_size,
                animation_time_secs: codex.animation_time_secs,
                alpha: mini_alpha_f,
            })
        };
    } else {
        animating = draw_expanded_content(ExpandedContentParams {
            canvas,
            blur_filter: blur_filter.clone(),
            expanded_alpha: expanded_alpha_f,
            view_offset,
            current_w,
            offset_x,
            offset_y,
            current_h,
            media,
            music_active,
            global_scale,
            expansion_progress,
            viz_h_scale,
            use_blur,
            font_size,
            dt,
            text_color,
            text_color_sec,
            palette: &palette,
            lyrics_delay,
            widget_layout,
        });
        draw_mini_content(MiniContentParams {
            canvas,
            content: mini_content,
            mini_alpha: mini_alpha_f,
            current_w,
            global_scale,
            media,
            offset_x,
            stable_offset_y,
            base_h,
            palette: &palette,
            viz_h_scale,
            current_lyric,
            old_lyric,
            expansion_progress,
            font_size,
            lyric_scroll_offset,
            use_blur,
            lyric_transition,
            text_color,
        });
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
    animating
}
