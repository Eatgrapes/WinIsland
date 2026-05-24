use super::HOVER_ROW_KEY_BASE;
use super::anim::SwitchAnimator;
use super::items::*;
use crate::core::i18n::tr;
use crate::utils::anim::AnimPool;
use crate::utils::color::SettingsTheme;
use crate::utils::font::{DrawTextCachedParams, DrawTextInRectParams, FontManager};
use skia_safe::{Canvas, Color, FontStyle, Paint, Rect};

struct PillBtnParams<'a> {
    canvas: &'a Canvas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: &'a str,
    text_color: Color,
    bg_color: Color,
}

pub struct DrawItemsParams<'a> {
    pub canvas: &'a Canvas,
    pub items: &'a [SettingsItem],
    pub start_y: f32,
    pub width: f32,
    pub anims: &'a SwitchAnimator,
    pub hover_anims: &'a AnimPool,
    pub theme: &'a SettingsTheme,
    pub visible_min_y: f32,
    pub visible_max_y: f32,
}

fn draw_switch(canvas: &Canvas, x: f32, y: f32, pos: f32, enabled: bool, theme: &SettingsTheme) {
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
    canvas.draw_round_rect(Rect::from_xywh(x, y, TOGGLE_W, TOGGLE_H), TOGGLE_R, TOGGLE_R, &paint);

    let knob_x = x + TOGGLE_INSET + (pos * (TOGGLE_W - TOGGLE_KNOB - TOGGLE_INSET * 2.0));
    let knob_y = y + TOGGLE_INSET;

    let mut shadow = Paint::default();
    shadow.set_anti_alias(true);
    shadow.set_color(Color::from_argb(40, 0, 0, 0));
    canvas.draw_round_rect(
        Rect::from_xywh(knob_x, knob_y + 1.0, TOGGLE_KNOB, TOGGLE_KNOB),
        TOGGLE_KNOB / 2.0, TOGGLE_KNOB / 2.0, &shadow,
    );

    paint.set_color(Color::WHITE);
    canvas.draw_round_rect(
        Rect::from_xywh(knob_x, knob_y, TOGGLE_KNOB, TOGGLE_KNOB),
        TOGGLE_KNOB / 2.0, TOGGLE_KNOB / 2.0, &paint,
    );
}

fn draw_stepper_btn(canvas: &Canvas, x: f32, y: f32, label: &str, enabled: bool, theme: &SettingsTheme) {
    let fm = FontManager::global();
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(if enabled { theme.card_highlight } else { theme.disabled });
    canvas.draw_round_rect(
        Rect::from_xywh(x, y, STEPPER_BTN_SIZE, STEPPER_BTN_SIZE),
        STEPPER_BTN_SIZE / 2.0, STEPPER_BTN_SIZE / 2.0, &paint,
    );
    paint.set_color(if enabled { theme.text_pri } else { theme.text_sec });
    fm.draw_text_in_rect(DrawTextInRectParams {
        canvas,
        text: label,
        x,
        y: y + 17.0,
        w: STEPPER_BTN_SIZE,
        size: 16.0,
        bold: false,
        paint: &paint,
    });
}

fn draw_pill_btn(params: PillBtnParams<'_>) {
    let fm = FontManager::global();
    let canvas = params.canvas;
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(params.bg_color);
    canvas.draw_round_rect(
        Rect::from_xywh(params.x, params.y, params.w, params.h),
        params.h / 2.0, params.h / 2.0, &paint,
    );
    paint.set_color(params.text_color);
    fm.draw_text_in_rect(DrawTextInRectParams {
        canvas,
        text: params.label,
        x: params.x,
        y: params.y + 17.0,
        w: params.w,
        size: 12.0,
        bold: true,
        paint: &paint,
    });
}

fn truncate_text(fm: &FontManager, text: &str, size: f32, max_w: f32) -> String {
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

fn draw_row_hover(canvas: &Canvas, y: f32, content_w: f32, row_idx: usize, in_group: bool, hover_anims: &AnimPool, theme: &SettingsTheme) {
    let val = hover_anims.get(HOVER_ROW_KEY_BASE + row_idx as u64);
    if val > 0.005 {
        let alpha = (val * 15.0) as u8;
        let base = theme.hover_row;
        let mut hp = Paint::default();
        hp.set_anti_alias(true);
        hp.set_color(Color::from_argb(alpha, base.r(), base.g(), base.b()));
        if in_group {
            canvas.draw_round_rect(
                Rect::from_xywh(CONTENT_PADDING + 2.0, y, content_w - 4.0, ROW_HEIGHT),
                4.0, 4.0, &hp,
            );
        } else {
            canvas.draw_round_rect(
                Rect::from_xywh(CONTENT_PADDING, y, content_w, ROW_HEIGHT),
                GROUP_RADIUS, GROUP_RADIUS, &hp,
            );
        }
    }
}

pub fn content_height(items: &[SettingsItem], start_y: f32) -> f32 {
    let mut h = start_y;
    for item in items {
        h += item.height();
    }
    h
}

pub fn draw_items(params: DrawItemsParams<'_>) {
    let canvas = params.canvas;
    let items = params.items;
    let start_y = params.start_y;
    let width = params.width;
    let anims = params.anims;
    let hover_anims = params.hover_anims;
    let theme = params.theme;
    let visible_min_y = params.visible_min_y;
    let visible_max_y = params.visible_max_y;

    let fm = FontManager::global();
    let mut y = start_y;
    let mut switch_idx = 0;
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    let mut in_group = false;
    let mut group_row_count = 0;
    let mut group_current_row = 0;
    let content_w = width - CONTENT_PADDING * 2.0;
    let mut row_idx: usize = 0;

    let mut i = 0;
    while i < items.len() {
        let item = &items[i];
        if y > visible_max_y + 120.0 {
            break;
        }
        match item {
            SettingsItem::PageTitle { text } => {
                let h = item.height();
                if y + h >= visible_min_y && y <= visible_max_y {
                    paint.set_color(theme.text_pri);
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text,
                        pos: (CONTENT_PADDING, y + 35.0),
                        size: 20.0,
                        style: FontStyle::bold(),
                        paint: &paint,
                        align_center: false,
                        max_w: f32::MAX,
                    });
                }
            }
            SettingsItem::SectionHeader { label } => {
                let h = item.height();
                if y + h >= visible_min_y && y <= visible_max_y {
                    paint.set_color(theme.text_sec);
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: label,
                        pos: (CONTENT_PADDING + 4.0, y + 22.0),
                        size: 12.0,
                        style: FontStyle::normal(),
                        paint: &paint,
                        align_center: false,
                        max_w: f32::MAX,
                    });
                }
            }
            SettingsItem::GroupStart => {
                in_group = true;
                group_current_row = 0;
                group_row_count = count_group_rows_from(items, i + 1);
                let total_h = group_row_count as f32 * ROW_HEIGHT;
                if y + total_h >= visible_min_y && y <= visible_max_y {
                    let mut bg = Paint::default();
                    bg.set_anti_alias(true);
                    bg.set_color(theme.group_bg);
                    canvas.draw_round_rect(
                        Rect::from_xywh(CONTENT_PADDING, y, content_w, total_h),
                        GROUP_RADIUS, GROUP_RADIUS, &bg,
                    );
                }
            }
            SettingsItem::GroupEnd => {
                in_group = false;
            }
            SettingsItem::RowStepper { label, value, enabled } => {
                if y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y {
                    draw_row_hover(canvas, y, content_w, row_idx, in_group, hover_anims, theme);
                }
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + ROW_HEIGHT / 2.0;

                if y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y {
                    paint.set_color(if *enabled { theme.text_pri } else { theme.text_sec });
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: label,
                        pos: (row_x, cy + 5.0),
                        size: 13.0,
                        style: FontStyle::normal(),
                        paint: &paint,
                        align_center: false,
                        max_w: f32::MAX,
                    });
                }

                let btn_inc_x = CONTENT_PADDING + content_w - GROUP_INNER_PAD - STEPPER_BTN_SIZE;
                let btn_dec_x = btn_inc_x - STEPPER_BTN_SIZE - 60.0;
                let btn_y = cy - STEPPER_BTN_SIZE / 2.0;
                if y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y {
                    draw_stepper_btn(canvas, btn_dec_x, btn_y, "-", *enabled, theme);
                    draw_stepper_btn(canvas, btn_inc_x, btn_y, "+", *enabled, theme);
                }

                let val_center = (btn_dec_x + STEPPER_BTN_SIZE + btn_inc_x) / 2.0;
                if y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y {
                    paint.set_color(if *enabled { theme.text_pri } else { theme.text_sec });
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: value,
                        pos: (val_center, cy + 5.0),
                        size: 13.0,
                        style: FontStyle::normal(),
                        paint: &paint,
                        align_center: true,
                        max_w: f32::MAX,
                    });
                }

                if in_group {
                    group_current_row += 1;
                    if group_current_row < group_row_count && y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y {
                        let mut sep = Paint::default();
                        sep.set_anti_alias(true);
                        sep.set_color(theme.separator);
                        sep.set_stroke_width(0.5);
                        sep.set_style(skia_safe::paint::Style::Stroke);
                        canvas.draw_line((row_x, y + ROW_HEIGHT), (CONTENT_PADDING + content_w - GROUP_INNER_PAD, y + ROW_HEIGHT), &sep);
                    }
                }
                row_idx += 1;
            }
            SettingsItem::RowSwitch { label, on: _, enabled } => {
                if y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y {
                    draw_row_hover(canvas, y, content_w, row_idx, in_group, hover_anims, theme);
                }
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + ROW_HEIGHT / 2.0;

                if y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y {
                    paint.set_color(if *enabled { theme.text_pri } else { theme.text_sec });
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: label,
                        pos: (row_x, cy + 5.0),
                        size: 13.0,
                        style: FontStyle::normal(),
                        paint: &paint,
                        align_center: false,
                        max_w: f32::MAX,
                    });
                }

                let toggle_x = CONTENT_PADDING + content_w - GROUP_INNER_PAD - TOGGLE_W;
                let toggle_y = cy - TOGGLE_H / 2.0;
                if y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y {
                    draw_switch(canvas, toggle_x, toggle_y, anims.get(switch_idx), *enabled, theme);
                }
                switch_idx += 1;

                if in_group {
                    group_current_row += 1;
                    if group_current_row < group_row_count && y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y {
                        let mut sep = Paint::default();
                        sep.set_anti_alias(true);
                        sep.set_color(theme.separator);
                        sep.set_stroke_width(0.5);
                        sep.set_style(skia_safe::paint::Style::Stroke);
                        canvas.draw_line((row_x, y + ROW_HEIGHT), (CONTENT_PADDING + content_w - GROUP_INNER_PAD, y + ROW_HEIGHT), &sep);
                    }
                }
                row_idx += 1;
            }
            SettingsItem::RowFontPicker { label, btn_label, reset_label } => {
                let visible = y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y;
                if visible {
                    draw_row_hover(canvas, y, content_w, row_idx, in_group, hover_anims, theme);
                }
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + ROW_HEIGHT / 2.0;

                if visible {
                    paint.set_color(theme.text_pri);
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: label,
                        pos: (row_x, cy + 5.0),
                        size: 13.0,
                        style: FontStyle::normal(),
                        paint: &paint,
                        align_center: false,
                        max_w: f32::MAX,
                    });

                    let sel_w: f32 = 60.0;
                    let sel_x = CONTENT_PADDING + content_w - GROUP_INNER_PAD - sel_w;
                    draw_pill_btn(PillBtnParams {
                        canvas,
                        x: sel_x,
                        y: cy - 13.0,
                        w: sel_w,
                        h: 26.0,
                        label: btn_label,
                        text_color: theme.text_pri,
                        bg_color: theme.card_highlight,
                    });

                    if let Some(rl) = reset_label {
                        let rst_w: f32 = 60.0;
                        let rst_x = sel_x - rst_w - 6.0;
                        draw_pill_btn(PillBtnParams {
                            canvas,
                            x: rst_x,
                            y: cy - 13.0,
                            w: rst_w,
                            h: 26.0,
                            label: rl,
                            text_color: theme.danger,
                            bg_color: theme.card_highlight,
                        });
                    }
                }

                if in_group {
                    group_current_row += 1;
                    if group_current_row < group_row_count && visible {
                        let mut sep = Paint::default();
                        sep.set_anti_alias(true);
                        sep.set_color(theme.separator);
                        sep.set_stroke_width(0.5);
                        sep.set_style(skia_safe::paint::Style::Stroke);
                        canvas.draw_line((row_x, y + ROW_HEIGHT), (CONTENT_PADDING + content_w - GROUP_INNER_PAD, y + ROW_HEIGHT), &sep);
                    }
                }
                row_idx += 1;
            }
            SettingsItem::RowSourceSelect { label, options, enabled } => {
                let visible = y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y;
                if visible {
                    draw_row_hover(canvas, y, content_w, row_idx, in_group, hover_anims, theme);
                }
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + ROW_HEIGHT / 2.0;

                if visible {
                    paint.set_color(if *enabled { theme.text_pri } else { theme.text_sec });
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: label,
                        pos: (row_x, cy + 5.0),
                        size: 13.0,
                        style: FontStyle::normal(),
                        paint: &paint,
                        align_center: false,
                        max_w: f32::MAX,
                    });
                }

                let selected_label = options.iter()
                    .find(|(_, active)| *active)
                    .map(|(l, _)| l.as_str())
                    .unwrap_or("");

                let btn_x = CONTENT_PADDING + content_w - GROUP_INNER_PAD - POPUP_BTN_W;
                let btn_y = cy - POPUP_BTN_H / 2.0;

                if visible {
                    let mut p = Paint::default();
                    p.set_anti_alias(true);
                    p.set_color(if *enabled { theme.card_highlight } else { theme.disabled });
                    canvas.draw_round_rect(
                        Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                        POPUP_BTN_R, POPUP_BTN_R, &p,
                    );

                    p.set_color(if *enabled { theme.text_pri } else { theme.text_sec });
                    let text_w = POPUP_BTN_W - 22.0;
                    fm.draw_text_in_rect(DrawTextInRectParams {
                        canvas,
                        text: selected_label,
                        x: btn_x + 4.0,
                        y: btn_y + 17.0,
                        w: text_w,
                        size: 12.0,
                        bold: true,
                        paint: &p,
                    });

                    let chev_cx = btn_x + POPUP_BTN_W - 12.0;
                    let chev_cy = cy;
                    let chev_svg = format!(
                        "M {} {} L {} {} L {} {}",
                        chev_cx - 3.0, chev_cy - 1.5,
                        chev_cx, chev_cy + 1.5,
                        chev_cx + 3.0, chev_cy - 1.5,
                    );
                    p.set_color(if *enabled { theme.text_sec } else { theme.disabled });
                    p.set_style(skia_safe::paint::Style::Stroke);
                    p.set_stroke_width(1.5);
                    if let Some(chev_path) = skia_safe::Path::from_svg(&chev_svg) {
                        canvas.draw_path(&chev_path, &p);
                    }
                }

                if in_group {
                    group_current_row += 1;
                    if group_current_row < group_row_count && visible {
                        let mut sep = Paint::default();
                        sep.set_anti_alias(true);
                        sep.set_color(theme.separator);
                        sep.set_stroke_width(0.5);
                        sep.set_style(skia_safe::paint::Style::Stroke);
                        canvas.draw_line((row_x, y + ROW_HEIGHT), (CONTENT_PADDING + content_w - GROUP_INNER_PAD, y + ROW_HEIGHT), &sep);
                    }
                }
                row_idx += 1;
            }
            SettingsItem::RowAppItem { label, active, enabled } => {
                let visible = y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y;
                if visible {
                    draw_row_hover(canvas, y, content_w, row_idx, in_group, hover_anims, theme);
                }
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + ROW_HEIGHT / 2.0;

                let check_size = 20.0;
                let check_x = CONTENT_PADDING + content_w - GROUP_INNER_PAD - check_size;
                let check_y = cy - check_size / 2.0;

                let mut p = Paint::default();
                p.set_anti_alias(true);
                if visible && *active && *enabled {
                    p.set_color(theme.accent);
                    canvas.draw_round_rect(Rect::from_xywh(check_x, check_y, check_size, check_size), 5.0, 5.0, &p);
                    p.set_color(Color::WHITE);
                    p.set_stroke_width(2.0);
                    p.set_style(skia_safe::paint::Style::Stroke);
                    let svg = format!(
                        "M {} {} L {} {} L {} {}",
                        check_x + 5.0, check_y + 10.0,
                        check_x + 9.0, check_y + 14.0,
                        check_x + 15.0, check_y + 6.0,
                    );
                    if let Some(path) = skia_safe::Path::from_svg(&svg) {
                        canvas.draw_path(&path, &p);
                    }
                } else if visible {
                    p.set_color(if *enabled { theme.card_highlight } else { theme.disabled });
                    p.set_style(skia_safe::paint::Style::Stroke);
                    p.set_stroke_width(1.5);
                    canvas.draw_round_rect(Rect::from_xywh(check_x, check_y, check_size, check_size), 5.0, 5.0, &p);
                }

                if visible {
                    paint.set_color(if *enabled { theme.text_pri } else { theme.text_sec });
                    let max_label_w = check_x - row_x - 8.0;
                    let display = truncate_text(fm, label, 13.0, max_label_w);
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: &display,
                        pos: (row_x, cy + 5.0),
                        size: 13.0,
                        style: FontStyle::normal(),
                        paint: &paint,
                        align_center: false,
                        max_w: max_label_w,
                    });
                }

                if in_group {
                    group_current_row += 1;
                    if group_current_row < group_row_count && visible {
                        let mut sep = Paint::default();
                        sep.set_anti_alias(true);
                        sep.set_color(theme.separator);
                        sep.set_stroke_width(0.5);
                        sep.set_style(skia_safe::paint::Style::Stroke);
                        canvas.draw_line((row_x, y + ROW_HEIGHT), (CONTENT_PADDING + content_w - GROUP_INNER_PAD, y + ROW_HEIGHT), &sep);
                    }
                }
                row_idx += 1;
            }
            SettingsItem::RowLabel { label } => {
                let visible = y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y;
                if visible {
                    draw_row_hover(canvas, y, content_w, row_idx, in_group, hover_anims, theme);
                }
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + ROW_HEIGHT / 2.0;
                if visible {
                    paint.set_color(theme.text_sec);
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: label,
                        pos: (row_x, cy + 5.0),
                        size: 13.0,
                        style: FontStyle::normal(),
                        paint: &paint,
                        align_center: false,
                        max_w: f32::MAX,
                    });
                }

                if in_group {
                    group_current_row += 1;
                    if group_current_row < group_row_count && visible {
                        let mut sep = Paint::default();
                        sep.set_anti_alias(true);
                        sep.set_color(theme.separator);
                        sep.set_stroke_width(0.5);
                        sep.set_style(skia_safe::paint::Style::Stroke);
                        canvas.draw_line((row_x, y + ROW_HEIGHT), (CONTENT_PADDING + content_w - GROUP_INNER_PAD, y + ROW_HEIGHT), &sep);
                    }
                }
                row_idx += 1;
            }
            SettingsItem::CenterLink { label, color } => {
                let h = item.height();
                if y + h >= visible_min_y && y <= visible_max_y {
                    paint.set_color(*color);
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: label,
                        pos: (width / 2.0, y + 24.0),
                        size: 13.0,
                        style: FontStyle::normal(),
                        paint: &paint,
                        align_center: true,
                        max_w: f32::MAX,
                    });
                }
            }
            SettingsItem::CenterText { text, size, color } => {
                let h = item.height();
                if y + h >= visible_min_y && y <= visible_max_y {
                    paint.set_color(*color);
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text,
                        pos: (width / 2.0, y + 22.0),
                        size: *size,
                        style: FontStyle::normal(),
                        paint: &paint,
                        align_center: true,
                        max_w: f32::MAX,
                    });
                }
            }
            SettingsItem::Spacer { .. } => {}
            SettingsItem::FontPreview { has_custom_font } => {
                let preview_h = 100.0;
                let visible = y + preview_h >= visible_min_y && y <= visible_max_y;
                if visible {
                    let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                    let preview_w = content_w - GROUP_INNER_PAD * 2.0;

                    let mut bg_p = Paint::default();
                    bg_p.set_anti_alias(true);
                    bg_p.set_color(theme.card_highlight);
                    canvas.draw_round_rect(Rect::from_xywh(row_x, y, preview_w, preview_h), 8.0, 8.0, &bg_p);

                    let mut label_p = Paint::default();
                    label_p.set_anti_alias(true);
                    label_p.set_color(theme.text_sec);
                    fm.draw_text_with_default_font(canvas, &tr("font_preview_default"), (row_x + 8.0, y + 16.0), 11.0, false, &label_p);

                    label_p.set_color(theme.text_pri);
                    let default_samples = ["Hello World", "你好世界", "こんにちは"];
                    for (si, sample) in default_samples.iter().enumerate() {
                        fm.draw_text_with_default_font(canvas, sample, (row_x + 8.0, y + 36.0 + si as f32 * 18.0), 14.0, false, &label_p);
                    }

                    if *has_custom_font {
                        let div_x = row_x + preview_w / 2.0;
                        let mut div_p = Paint::default();
                        div_p.set_anti_alias(true);
                        div_p.set_color(theme.separator);
                        div_p.set_stroke_width(1.0);
                        div_p.set_style(skia_safe::paint::Style::Stroke);
                        canvas.draw_line((div_x, y + 8.0), (div_x, y + preview_h - 8.0), &div_p);

                        label_p.set_color(theme.text_sec);
                        fm.draw_text_cached(DrawTextCachedParams {
                            canvas,
                            text: &tr("font_preview_custom"),
                            pos: (div_x + 8.0, y + 16.0),
                            size: 11.0,
                            style: FontStyle::normal(),
                            paint: &label_p,
                            align_center: false,
                            max_w: f32::MAX,
                        });

                        label_p.set_color(theme.accent);
                        let custom_samples = ["Hello World", "你好世界", "こんにちは"];
                        for (si, sample) in custom_samples.iter().enumerate() {
                            fm.draw_text_with_custom_font(canvas, sample, (div_x + 8.0, y + 36.0 + si as f32 * 18.0), 14.0, false, &label_p);
                        }
                    }
                }
            }
        }
        y += item.height();
        i += 1;
    }
}

fn count_group_rows_from(items: &[SettingsItem], start: usize) -> usize {
    let mut count = 0;
    let mut found_group = false;
    for item in &items[start..] {
        if matches!(item, SettingsItem::GroupStart) {
            found_group = true;
            continue;
        }
        if matches!(item, SettingsItem::GroupEnd) {
            break;
        }
        if found_group && item.is_row() {
            count += 1;
        }
    }
    count
}
