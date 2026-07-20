use skia_safe::{
    Canvas, ClipOp, Color, FilterMode, MipmapMode, Paint, RRect, Rect, SamplingOptions,
};

use crate::core::smtc::MediaInfo;
use crate::utils::backdrop::get_mica_background;
use crate::utils::glass::get_glass_background;

pub(super) struct BackgroundParams<'a> {
    pub(super) canvas: &'a Canvas,
    pub(super) rect: Rect,
    pub(super) rrect: RRect,
    pub(super) island_style: &'a str,
    pub(super) media: &'a MediaInfo,
    pub(super) win_x: i32,
    pub(super) win_y: i32,
    pub(super) offset_x: f32,
    pub(super) offset_y: f32,
    pub(super) current_w: f32,
    pub(super) current_h: f32,
    pub(super) global_scale: f32,
    pub(super) monitor_x: i32,
    pub(super) monitor_y: i32,
    pub(super) monitor_w: u32,
    pub(super) monitor_h: u32,
}

pub(super) fn draw_background(params: BackgroundParams<'_>) {
    let BackgroundParams {
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
    } = params;
    let bg_color = Color::BLACK;
    let screen_x = win_x + offset_x as i32;
    let screen_y = win_y + offset_y as i32;
    if island_style == "glass" {
        canvas.save();
        canvas.clip_rrect(rrect, ClipOp::Intersect, true);
        if let Some(bg_img) = get_glass_background(
            screen_x,
            screen_y,
            current_w as u32,
            current_h as u32,
            40.0 * global_scale,
            monitor_x,
            monitor_y,
            monitor_w,
            monitor_h,
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

            let angle_rad = (now * 0.03) % (2.0 * std::f64::consts::PI);
            let angle_deg = angle_rad * 180.0 / std::f64::consts::PI;

            let dx = (now * 0.15).sin() * 20.0;
            let dy = (now * 0.12).cos() * 15.0;

            let cx = rect.left() + rect.width() / 2.0;
            let cy = rect.top() + rect.height() / 2.0;

            let diagonal = (rect.width() * rect.width() + rect.height() * rect.height()).sqrt();
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
            if let Some(bg_img) = get_glass_background(
                screen_x,
                screen_y,
                current_w as u32,
                current_h as u32,
                40.0 * global_scale,
                monitor_x,
                monitor_y,
                monitor_w,
                monitor_h,
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
    canvas.restore();
}
