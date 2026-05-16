use super::anim::SwitchAnimator;
use super::items::*;
use super::HOVER_ROW_KEY_BASE;
use crate::utils::anim::AnimPool;
use crate::utils::color::*;
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
    pub visible_min_y: f32,
    pub visible_max_y: f32,
}

fn draw_switch(canvas: &Canvas, x: f32, y: f32, pos: f32, enabled: bool) {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    let (off_color, on_color) = if enabled {
        (COLOR_TOGGLE_OFF, COLOR_TOGGLE_ON)
    } else {
        (COLOR_TOGGLE_OFF, COLOR_TOGGLE_OFF)
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

fn draw_stepper_btn(canvas: &Canvas, x: f32, y: f32, label: &str, enabled: bool) {
    let fm = FontManager::global();
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(if enabled {
        COLOR_CARD_HIGHLIGHT
    } else {
        COLOR_DISABLED
    });
    canvas.draw_round_rect(
        Rect::from_xywh(x, y, STEPPER_BTN_SIZE, STEPPER_BTN_SIZE),
        STEPPER_BTN_SIZE / 2.0,
        STEPPER_BTN_SIZE / 2.0,
        &paint,
    );
    paint.set_color(if enabled {
        COLOR_TEXT_PRI
    } else {
        COLOR_TEXT_SEC
    });
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
        params.h / 2.0,
        params.h / 2.0,
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
        bold: true,
        paint: &paint,
    });
}

fn count_group_rows(items: &[SettingsItem], start: usize) -> usize {
    let mut count = 0;
    let mut i = start;
    while i < items.len() {
        if matches!(items[i], SettingsItem::GroupEnd) {
            break;
        }
        if items[i].is_row() {
            count += 1;
        }
        i += 1;
    }
    count
}

fn draw_row_hover(
    canvas: &Canvas,
    y: f32,
    content_w: f32,
    row_idx: usize,
    in_group: bool,
    hover_anims: &AnimPool,
) {
    let val = hover_anims.get(HOVER_ROW_KEY_BASE + row_idx as u64);
    if val > 0.005 {
        let alpha = (val * 15.0) as u8;
        let mut hp = Paint::default();
        hp.set_anti_alias(true);
        hp.set_color(Color::from_argb(alpha, 255, 255, 255));
        if in_group {
            canvas.draw_round_rect(
                Rect::from_xywh(CONTENT_PADDING + 2.0, y, content_w - 4.0, ROW_HEIGHT),
                4.0,
                4.0,
                &hp,
            );
        } else {
            canvas.draw_round_rect(
                Rect::from_xywh(CONTENT_PADDING, y, content_w, ROW_HEIGHT),
                GROUP_RADIUS,
                GROUP_RADIUS,
                &hp,
            );
        }
    }
}

pub fn draw_items(params: DrawItemsParams<'_>) {
    let canvas = params.canvas;
    let items = params.items;
    let start_y = params.start_y;
    let width = params.width;
    let anims = params.anims;
    let hover_anims = params.hover_anims;
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
                    paint.set_color(COLOR_TEXT_PRI);
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
                    paint.set_color(COLOR_TEXT_SEC);
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
                group_row_count = count_group_rows(items, i + 1);
                let total_h = group_row_count as f32 * ROW_HEIGHT;
                if y + total_h >= visible_min_y && y <= visible_max_y {
                    let mut bg = Paint::default();
                    bg.set_anti_alias(true);
                    bg.set_color(COLOR_GROUP_BG);
                    canvas.draw_round_rect(
                        Rect::from_xywh(CONTENT_PADDING, y, content_w, total_h),
                        GROUP_RADIUS,
                        GROUP_RADIUS,
                        &bg,
                    );
                }
            }
            SettingsItem::GroupEnd => {
                in_group = false;
            }
            SettingsItem::RowStepper {
                label,
                value,
                enabled,
            } => {
                if y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y {
                    draw_row_hover(canvas, y, content_w, row_idx, in_group, hover_anims);
                }
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + ROW_HEIGHT / 2.0;

                if y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y {
                    paint.set_color(if *enabled {
                        COLOR_TEXT_PRI
                    } else {
                        COLOR_TEXT_SEC
                    });
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
                    draw_stepper_btn(canvas, btn_dec_x, btn_y, "-", *enabled);
                    draw_stepper_btn(canvas, btn_inc_x, btn_y, "+", *enabled);
                }

                let val_center = (btn_dec_x + STEPPER_BTN_SIZE + btn_inc_x) / 2.0;
                if y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y {
                    paint.set_color(if *enabled {
                        COLOR_TEXT_PRI
                    } else {
                        COLOR_TEXT_SEC
                    });
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
                    if group_current_row < group_row_count
                        && y + ROW_HEIGHT >= visible_min_y
                        && y <= visible_max_y
                    {
                        let mut sep = Paint::default();
                        sep.set_anti_alias(true);
                        sep.set_color(color_separator());
                        sep.set_stroke_width(0.5);
                        sep.set_style(skia_safe::paint::Style::Stroke);
                        canvas.draw_line(
                            (row_x, y + ROW_HEIGHT),
                            (
                                CONTENT_PADDING + content_w - GROUP_INNER_PAD,
                                y + ROW_HEIGHT,
                            ),
                            &sep,
                        );
                    }
                }
                row_idx += 1;
            }
            SettingsItem::RowSwitch {
                label,
                on: _,
                enabled,
            } => {
                if y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y {
                    draw_row_hover(canvas, y, content_w, row_idx, in_group, hover_anims);
                }
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + ROW_HEIGHT / 2.0;

                if y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y {
                    paint.set_color(if *enabled {
                        COLOR_TEXT_PRI
                    } else {
                        COLOR_TEXT_SEC
                    });
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
                    draw_switch(canvas, toggle_x, toggle_y, anims.get(switch_idx), *enabled);
                }
                switch_idx += 1;

                if in_group {
                    group_current_row += 1;
                    if group_current_row < group_row_count
                        && y + ROW_HEIGHT >= visible_min_y
                        && y <= visible_max_y
                    {
                        let mut sep = Paint::default();
                        sep.set_anti_alias(true);
                        sep.set_color(color_separator());
                        sep.set_stroke_width(0.5);
                        sep.set_style(skia_safe::paint::Style::Stroke);
                        canvas.draw_line(
                            (row_x, y + ROW_HEIGHT),
                            (
                                CONTENT_PADDING + content_w - GROUP_INNER_PAD,
                                y + ROW_HEIGHT,
                            ),
                            &sep,
                        );
                    }
                }
                row_idx += 1;
            }
            SettingsItem::RowFontPicker {
                label,
                btn_label,
                reset_label,
            } => {
                let visible = y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y;
                if visible {
                    draw_row_hover(canvas, y, content_w, row_idx, in_group, hover_anims);
                }
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + ROW_HEIGHT / 2.0;

                if visible {
                    paint.set_color(COLOR_TEXT_PRI);
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
                        text_color: COLOR_TEXT_PRI,
                        bg_color: COLOR_CARD_HIGHLIGHT,
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
                            text_color: COLOR_DANGER,
                            bg_color: COLOR_CARD_HIGHLIGHT,
                        });
                    }
                }

                if in_group {
                    group_current_row += 1;
                    if group_current_row < group_row_count && visible {
                        let mut sep = Paint::default();
                        sep.set_anti_alias(true);
                        sep.set_color(color_separator());
                        sep.set_stroke_width(0.5);
                        sep.set_style(skia_safe::paint::Style::Stroke);
                        canvas.draw_line(
                            (row_x, y + ROW_HEIGHT),
                            (
                                CONTENT_PADDING + content_w - GROUP_INNER_PAD,
                                y + ROW_HEIGHT,
                            ),
                            &sep,
                        );
                    }
                }
                row_idx += 1;
            }
            SettingsItem::RowSourceSelect {
                label,
                options,
                enabled,
            } => {
                let visible = y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y;
                if visible {
                    draw_row_hover(canvas, y, content_w, row_idx, in_group, hover_anims);
                }
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + ROW_HEIGHT / 2.0;

                if visible {
                    paint.set_color(if *enabled {
                        COLOR_TEXT_PRI
                    } else {
                        COLOR_TEXT_SEC
                    });
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

                let selected_label = options
                    .iter()
                    .find(|(_, active)| *active)
                    .map(|(l, _)| l.as_str())
                    .unwrap_or("");

                let btn_x = CONTENT_PADDING + content_w - GROUP_INNER_PAD - POPUP_BTN_W;
                let btn_y = cy - POPUP_BTN_H / 2.0;

                if visible {
                    let mut p = Paint::default();
                    p.set_anti_alias(true);
                    p.set_color(if *enabled {
                        COLOR_CARD_HIGHLIGHT
                    } else {
                        COLOR_DISABLED
                    });
                    canvas.draw_round_rect(
                        Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                        POPUP_BTN_R,
                        POPUP_BTN_R,
                        &p,
                    );

                    p.set_color(if *enabled {
                        COLOR_TEXT_PRI
                    } else {
                        COLOR_TEXT_SEC
                    });
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
                        chev_cx - 3.0,
                        chev_cy - 1.5,
                        chev_cx,
                        chev_cy + 1.5,
                        chev_cx + 3.0,
                        chev_cy - 1.5,
                    );
                    p.set_color(if *enabled {
                        COLOR_TEXT_SEC
                    } else {
                        COLOR_DISABLED
                    });
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
                        sep.set_color(color_separator());
                        sep.set_stroke_width(0.5);
                        sep.set_style(skia_safe::paint::Style::Stroke);
                        canvas.draw_line(
                            (row_x, y + ROW_HEIGHT),
                            (
                                CONTENT_PADDING + content_w - GROUP_INNER_PAD,
                                y + ROW_HEIGHT,
                            ),
                            &sep,
                        );
                    }
                }
                row_idx += 1;
            }
            SettingsItem::RowAppItem {
                label,
                active,
                enabled,
            } => {
                let visible = y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y;
                if visible {
                    draw_row_hover(canvas, y, content_w, row_idx, in_group, hover_anims);
                }
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + ROW_HEIGHT / 2.0;

                let check_size = 20.0;
                let check_x = CONTENT_PADDING + content_w - GROUP_INNER_PAD - check_size;
                let check_y = cy - check_size / 2.0;

                let mut p = Paint::default();
                p.set_anti_alias(true);
                if visible && *active && *enabled {
                    p.set_color(COLOR_ACCENT);
                    canvas.draw_round_rect(
                        Rect::from_xywh(check_x, check_y, check_size, check_size),
                        5.0,
                        5.0,
                        &p,
                    );
                    p.set_color(Color::WHITE);
                    p.set_stroke_width(2.0);
                    p.set_style(skia_safe::paint::Style::Stroke);
                    let svg = format!(
                        "M {} {} L {} {} L {} {}",
                        check_x + 5.0,
                        check_y + 10.0,
                        check_x + 9.0,
                        check_y + 14.0,
                        check_x + 15.0,
                        check_y + 6.0,
                    );
                    if let Some(path) = skia_safe::Path::from_svg(&svg) {
                        canvas.draw_path(&path, &p);
                    }
                } else if visible {
                    p.set_color(if *enabled {
                        COLOR_CARD_HIGHLIGHT
                    } else {
                        COLOR_DISABLED
                    });
                    p.set_style(skia_safe::paint::Style::Stroke);
                    p.set_stroke_width(1.5);
                    canvas.draw_round_rect(
                        Rect::from_xywh(check_x, check_y, check_size, check_size),
                        5.0,
                        5.0,
                        &p,
                    );
                }

                if visible {
                    paint.set_color(if *enabled {
                        COLOR_TEXT_PRI
                    } else {
                        COLOR_TEXT_SEC
                    });
                    let max_label_w = check_x - row_x - 8.0;
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: label,
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
                        sep.set_color(color_separator());
                        sep.set_stroke_width(0.5);
                        sep.set_style(skia_safe::paint::Style::Stroke);
                        canvas.draw_line(
                            (row_x, y + ROW_HEIGHT),
                            (
                                CONTENT_PADDING + content_w - GROUP_INNER_PAD,
                                y + ROW_HEIGHT,
                            ),
                            &sep,
                        );
                    }
                }
                row_idx += 1;
            }
            SettingsItem::RowLabel { label } => {
                let visible = y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y;
                if visible {
                    draw_row_hover(canvas, y, content_w, row_idx, in_group, hover_anims);
                }
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + ROW_HEIGHT / 2.0;
                if visible {
                    paint.set_color(COLOR_TEXT_SEC);
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
                        sep.set_color(color_separator());
                        sep.set_stroke_width(0.5);
                        sep.set_style(skia_safe::paint::Style::Stroke);
                        canvas.draw_line(
                            (row_x, y + ROW_HEIGHT),
                            (
                                CONTENT_PADDING + content_w - GROUP_INNER_PAD,
                                y + ROW_HEIGHT,
                            ),
                            &sep,
                        );
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
        }
        y += item.height();
        i += 1;
    }
}

pub fn content_height(items: &[SettingsItem], start_y: f32) -> f32 {
    let mut h = start_y;
    for item in items {
        h += item.height();
    }
    h
}
