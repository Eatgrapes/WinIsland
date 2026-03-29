use skia_safe::{Canvas, Color, Paint, Rect};
use crate::utils::color::*;
use crate::utils::font::FontManager;
use super::items::*;
use super::anim::SwitchAnimator;

fn draw_card(canvas: &Canvas, y: f32, width: f32) {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(COLOR_CARD);
    canvas.draw_round_rect(
        Rect::from_xywh(CARD_MARGIN, y - 5.0, width - CARD_MARGIN * 2.0, CARD_HEIGHT),
        CARD_RADIUS, CARD_RADIUS, &paint,
    );
}

fn draw_switch(canvas: &Canvas, x: f32, y: f32, pos: f32) {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    let off = COLOR_CARD_HIGHLIGHT;
    let on = COLOR_ACCENT;
    let r = off.r() as f32 + (on.r() as f32 - off.r() as f32) * pos;
    let g = off.g() as f32 + (on.g() as f32 - off.g() as f32) * pos;
    let b = off.b() as f32 + (on.b() as f32 - off.b() as f32) * pos;
    paint.set_color(Color::from_rgb(r as u8, g as u8, b as u8));
    canvas.draw_round_rect(Rect::from_xywh(x, y, 48.0, 26.0), 13.0, 13.0, &paint);
    paint.set_color(Color::WHITE);
    canvas.draw_round_rect(Rect::from_xywh(x + 2.0 + (pos * 22.0), y + 2.0, 22.0, 22.0), 11.0, 11.0, &paint);
}

fn draw_round_btn(canvas: &Canvas, x: f32, y: f32, label: &str) {
    let fm = FontManager::global();
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(COLOR_CARD_HIGHLIGHT);
    canvas.draw_round_rect(Rect::from_xywh(x, y, 28.0, 28.0), 14.0, 14.0, &paint);
    paint.set_color(COLOR_TEXT_PRI);
    fm.draw_text_in_rect(canvas, label, x, y + 20.0, 28.0, 20.0, false, &paint);
}

fn draw_pill_btn(canvas: &Canvas, x: f32, y: f32, w: f32, h: f32, label: &str, text_color: Color, bg_color: Color) {
    let fm = FontManager::global();
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(bg_color);
    canvas.draw_round_rect(Rect::from_xywh(x, y, w, h), h / 2.0, h / 2.0, &paint);
    paint.set_color(text_color);
    fm.draw_text_in_rect(canvas, label, x, y + 17.0, w, 12.0, true, &paint);
}

pub fn draw_items(canvas: &Canvas, items: &[SettingsItem], start_y: f32, width: f32, anims: &SwitchAnimator) {
    let fm = FontManager::global();
    let mut y = start_y;
    let mut switch_idx = 0;
    let mut paint = Paint::default();
    paint.set_anti_alias(true);

    for item in items {
        match item {
            SettingsItem::Stepper { label, value, enabled } => {
                draw_card(canvas, y, width);
                paint.set_color(if *enabled { COLOR_TEXT_PRI } else { COLOR_TEXT_SEC });
                fm.draw_text(canvas, label, (LABEL_X, y + LABEL_Y_OFFSET), 14.0, false, &paint);
                draw_round_btn(canvas, BTN_DEC_X, y + BTN_Y_OFFSET, "-");
                paint.set_color(if *enabled { COLOR_TEXT_PRI } else { COLOR_TEXT_SEC });
                fm.draw_text_centered(canvas, value, VALUE_CENTER_X, y + LABEL_Y_OFFSET, 14.0, false, &paint);
                draw_round_btn(canvas, BTN_INC_X, y + BTN_Y_OFFSET, "+");
            }
            SettingsItem::Switch { label, on: _ } => {
                draw_card(canvas, y, width);
                paint.set_color(COLOR_TEXT_PRI);
                fm.draw_text(canvas, label, (LABEL_X, y + LABEL_Y_OFFSET), 14.0, false, &paint);
                draw_switch(canvas, SWITCH_X, y + SWITCH_Y_OFFSET, anims.get(switch_idx));
                switch_idx += 1;
            }
            SettingsItem::TextButton { label, btn_label, btn_x, btn_w } => {
                draw_card(canvas, y, width);
                paint.set_color(COLOR_TEXT_PRI);
                fm.draw_text(canvas, label, (LABEL_X, y + LABEL_Y_OFFSET), 14.0, false, &paint);
                draw_pill_btn(canvas, *btn_x, y + 3.0, *btn_w, 26.0, btn_label, COLOR_TEXT_PRI, COLOR_CARD_HIGHLIGHT);
            }
            SettingsItem::FontPicker { label, btn_label, reset_label } => {
                draw_card(canvas, y, width);
                paint.set_color(COLOR_TEXT_PRI);
                fm.draw_text(canvas, label, (LABEL_X, y + LABEL_Y_OFFSET), 14.0, false, &paint);
                draw_pill_btn(canvas, 310.0, y + 3.0, 65.0, 26.0, btn_label, COLOR_TEXT_PRI, COLOR_CARD_HIGHLIGHT);
                if let Some(rl) = reset_label {
                    draw_pill_btn(canvas, 235.0, y + 3.0, 65.0, 26.0, rl, COLOR_DANGER, COLOR_CARD_HIGHLIGHT);
                }
            }
            SettingsItem::CenterLink { label, color } => {
                paint.set_color(*color);
                fm.draw_text_centered(canvas, label, width / 2.0, y + 20.0, 14.0, false, &paint);
            }
            SettingsItem::Title { text, size } => {
                paint.set_color(COLOR_TEXT_PRI);
                fm.draw_text(canvas, text, (25.0, y + 30.0), *size, true, &paint);
            }
            SettingsItem::CenterText { text, size, color } => {
                paint.set_color(*color);
                fm.draw_text_centered(canvas, text, width / 2.0, y + 20.0, *size, false, &paint);
            }
            SettingsItem::SourceSelect { label, options, enabled } => {
                draw_card(canvas, y, width);
                paint.set_color(if *enabled { COLOR_TEXT_PRI } else { COLOR_TEXT_SEC });
                fm.draw_text(canvas, label, (LABEL_X, y + LABEL_Y_OFFSET), 14.0, false, &paint);
                for (opt_label, opt_x, opt_w, active) in options {
                    let bg = if !enabled { COLOR_DISABLED } else if *active { COLOR_ACCENT } else { COLOR_CARD_HIGHLIGHT };
                    let tc = if *enabled { COLOR_TEXT_PRI } else { COLOR_TEXT_SEC };
                    let mut p = Paint::default();
                    p.set_anti_alias(true);
                    p.set_color(bg);
                    canvas.draw_round_rect(Rect::from_xywh(*opt_x, y + 3.0, *opt_w, 22.0), 11.0, 11.0, &p);
                    p.set_color(tc);
                    fm.draw_text_in_rect(canvas, opt_label, *opt_x, y + 18.0, *opt_w, 11.0, true, &p);
                }
            }
            SettingsItem::SectionHeader { label, btn } => {
                paint.set_color(COLOR_TEXT_SEC);
                fm.draw_text(canvas, label, (30.0, y + 15.0), 12.0, true, &paint);
                if let Some((bl, bx, bw, enabled)) = btn {
                    let bg = if *enabled { COLOR_CARD_HIGHLIGHT } else { COLOR_DISABLED };
                    let tc = if *enabled { COLOR_TEXT_PRI } else { COLOR_TEXT_SEC };
                    draw_pill_btn(canvas, *bx, y, *bw, 24.0, bl, tc, bg);
                }
            }
            SettingsItem::Label { label, enabled } => {
                draw_card(canvas, y, width);
                paint.set_color(if *enabled { COLOR_TEXT_PRI } else { COLOR_TEXT_SEC });
                fm.draw_text(canvas, label, (LABEL_X, y + LABEL_Y_OFFSET), 15.0, false, &paint);
            }
        }
        y += item.height();
    }
}

pub fn content_height(items: &[SettingsItem], start_y: f32) -> f32 {
    let mut h = start_y;
    for item in items {
        h += item.height();
    }
    h
}
