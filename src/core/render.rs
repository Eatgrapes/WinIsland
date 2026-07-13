mod background;
mod expanded;
mod mini;

use self::background::{BackgroundParams, draw_background};
use self::expanded::{ExpandedContentParams, draw_expanded_content};
use self::mini::{MiniContentParams, draw_mini_content};

use crate::core::config::{DockPosition, PADDING, TOP_OFFSET, WidgetSlot};
use crate::core::smtc::MediaInfo;
use crate::ui::expanded::music_view::get_media_palette;
use skia_safe::{
    ClipOp, Color, ISize, Paint, RRect, Rect, Surface as SkSurface, image_filters, surfaces,
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
    pub weights: [f32; 4],
    pub lyrics_delay: f64,
    pub dt: f32,
    pub widget_layout: &'a [WidgetSlot],
}

use crate::core::context::MiniContent;

pub struct DrawIslandParams<'a> {
    pub layout: LayoutParams,
    pub media: MediaParams<'a>,
    pub lyrics: LyricsParams<'a>,
    pub mini_content: Option<MiniContent<'a>>,
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

    let text_color = Color::WHITE;
    let text_color_sec = Color::WHITE;

    draw_background(BackgroundParams {
        canvas,
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

    let widget_animating = draw_expanded_content(ExpandedContentParams {
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
