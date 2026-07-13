use skia_safe::{Canvas, Color, FontStyle, Paint, Rect};

use crate::utils::color::SettingsTheme;
use crate::utils::font::{DrawTextInRectParams, FontManager};

use super::super::items::*;

pub(super) struct PillBtnParams<'a> {
    pub(super) canvas: &'a Canvas,
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) w: f32,
    pub(super) h: f32,
    pub(super) label: &'a str,
    pub(super) text_color: Color,
    pub(super) bg_color: Color,
}

pub(super) fn draw_switch(
    canvas: &Canvas,
    x: f32,
    y: f32,
    pos: f32,
    enabled: bool,
    theme: &SettingsTheme,
) {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    let (off_color, on_color) = if enabled {
        (theme.toggle_off, theme.toggle_on)
    } else {
        (theme.toggle_off, theme.toggle_off)
    };
    let r = off_color.r() as f32 + (on_color.r() as f32 - off_color.r() as f32) * pos;
    let g = off_color.g() as f32 + (on_color.g() as f32 - off_color.g() as f32) * pos;
    let b = off_color.b() as f32 + (on_color.b() as f32 - off_color.b() as f32) * pos;
    paint.set_color(Color::from_rgb(r as u8, g as u8, b as u8));
    canvas.draw_round_rect(
        Rect::from_xywh(x, y, TOGGLE_W, TOGGLE_H),
        TOGGLE_R,
        TOGGLE_R,
        &paint,
    );

    let knob_x = x + TOGGLE_INSET + (pos * (TOGGLE_W - TOGGLE_KNOB - TOGGLE_INSET * 2.0));
    let knob_y = y + TOGGLE_INSET;

    let mut shadow = Paint::default();
    shadow.set_anti_alias(true);
    shadow.set_color(Color::from_argb(40, 0, 0, 0));
    canvas.draw_round_rect(
        Rect::from_xywh(knob_x, knob_y + 1.0, TOGGLE_KNOB, TOGGLE_KNOB),
        TOGGLE_KNOB / 2.0,
        TOGGLE_KNOB / 2.0,
        &shadow,
    );

    paint.set_color(Color::WHITE);
    canvas.draw_round_rect(
        Rect::from_xywh(knob_x, knob_y, TOGGLE_KNOB, TOGGLE_KNOB),
        TOGGLE_KNOB / 2.0,
        TOGGLE_KNOB / 2.0,
        &paint,
    );
}

pub(super) fn draw_stepper_btn(
    canvas: &Canvas,
    x: f32,
    y: f32,
    label: &str,
    enabled: bool,
    theme: &SettingsTheme,
) {
    let fm = FontManager::global();
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(if enabled {
        theme.card_highlight
    } else {
        theme.disabled
    });
    canvas.draw_round_rect(
        Rect::from_xywh(x, y, STEPPER_BTN_SIZE, STEPPER_BTN_SIZE),
        STEPPER_BTN_SIZE / 2.0,
        STEPPER_BTN_SIZE / 2.0,
        &paint,
    );
    paint.set_color(if enabled {
        theme.text_pri
    } else {
        theme.text_sec
    });
    let font = fm.get_font(16.0, false);
    let (_, bounds) = font.measure_str(label, None);
    let text_x = x + (STEPPER_BTN_SIZE - bounds.width()) / 2.0 - bounds.left();
    let text_y = y + (STEPPER_BTN_SIZE - bounds.height()) / 2.0 - bounds.top();
    canvas.draw_str(label, (text_x, text_y), &font, &paint);
}

pub(super) fn draw_pill_btn(params: PillBtnParams<'_>) {
    let fm = FontManager::global();
    let canvas = params.canvas;
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(params.bg_color);
    canvas.draw_round_rect(
        Rect::from_xywh(params.x, params.y, params.w, params.h),
        POPUP_BTN_R,
        POPUP_BTN_R,
        &paint,
    );
    paint.set_color(params.text_color);
    fm.draw_text_in_rect(DrawTextInRectParams {
        canvas,
        text: params.label,
        x: params.x,
        y: params.y + 17.0,
        w: params.w,
        size: 12.0,
        bold: false,
        paint: &paint,
    });
}

pub(super) fn truncate_text(fm: &FontManager, text: &str, size: f32, max_w: f32) -> String {
    let w = fm.measure_text_cached(text, size, FontStyle::normal());
    if w <= max_w {
        return text.to_string();
    }
    let ellipsis = "...";
    let ew = fm.measure_text_cached(ellipsis, size, FontStyle::normal());
    let mut result = String::new();
    let mut current_w = 0.0;
    for c in text.chars() {
        let cw = fm.measure_text_cached(&c.to_string(), size, FontStyle::normal());
        if current_w + cw + ew > max_w {
            result.push_str(ellipsis);
            return result;
        }
        current_w += cw;
        result.push(c);
    }
    result
}
