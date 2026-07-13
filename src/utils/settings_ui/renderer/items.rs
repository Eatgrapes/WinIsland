use skia_safe::{Color, FontStyle, Paint, Rect};

use crate::core::i18n::tr;
use crate::utils::font::{DrawTextCachedParams, DrawTextInRectParams, FontManager};

use super::super::items::*;
use super::DrawItemsParams;
use super::controls::*;
use super::widget_preview::{WidgetPreviewParams, draw_widget_preview};

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
    let theme = params.theme;
    let visible_min_y = params.visible_min_y;
    let visible_max_y = params.visible_max_y;
    let island_style = params.island_style;
    let adaptive_border = params.adaptive_border;
    let expanded_width = params.expanded_width;
    let expanded_height = params.expanded_height;
    let widget_layout = params.widget_layout;
    let widget_dragging = params.widget_dragging;
    let widget_drag_hover_slot = params.widget_drag_hover_slot;
    let widget_preview_hover_slot = params.widget_preview_hover_slot;

    let fm = FontManager::global();
    let mut y = start_y;
    let mut switch_idx = 0;
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    let mut in_group = false;
    let mut group_row_count = 0;
    let mut group_current_row = 0;
    let content_w = width - CONTENT_PADDING * 2.0;

    let mut i = 0;
    while i < items.len() {
        let item = &items[i];
        if y > visible_max_y + 120.0 {
            break;
        }
        match item {
            SettingsItem::SectionHeader { label } => {
                let h = item.height();
                if y + h >= visible_min_y && y <= visible_max_y {
                    paint.set_color(theme.text_pri);
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: label,
                        x: CONTENT_PADDING + 4.0,
                        y: y + 22.0,
                        size: 13.0,
                        bold: true,
                        paint: &paint,
                    });
                }
            }
            SettingsItem::GroupStart => {
                in_group = true;
                group_current_row = 0;
                let total_h = group_height_from(items, i + 1);
                group_row_count = items[i + 1..]
                    .iter()
                    .take_while(|item| !matches!(item, SettingsItem::GroupEnd))
                    .filter(|item| item.is_row())
                    .count();
                if y + total_h >= visible_min_y && y <= visible_max_y {
                    let mut bg = Paint::default();
                    bg.set_anti_alias(true);
                    bg.set_color(theme.group_bg);
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
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + ROW_HEIGHT / 2.0;

                if y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y {
                    paint.set_color(if *enabled {
                        theme.text_pri
                    } else {
                        theme.text_sec
                    });
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: label,
                        x: row_x,
                        y: cy + 5.0,
                        size: 13.0,
                        bold: false,
                        paint: &paint,
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
                    paint.set_color(if *enabled {
                        theme.text_pri
                    } else {
                        theme.text_sec
                    });
                    let val_w = fm.measure_text_cached(value, 13.0, FontStyle::normal());
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: value,
                        x: val_center - val_w / 2.0,
                        y: cy + 5.0,
                        size: 13.0,
                        bold: false,
                        paint: &paint,
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
                        sep.set_color(theme.separator);
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
            }
            SettingsItem::RowSwitch {
                label,
                on: _,
                enabled,
            } => {
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + ROW_HEIGHT / 2.0;

                if y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y {
                    paint.set_color(if *enabled {
                        theme.text_pri
                    } else {
                        theme.text_sec
                    });
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: label,
                        x: row_x,
                        y: cy + 5.0,
                        size: 13.0,
                        bold: false,
                        paint: &paint,
                    });
                }

                let toggle_x = CONTENT_PADDING + content_w - GROUP_INNER_PAD - TOGGLE_W;
                let toggle_y = cy - TOGGLE_H / 2.0;
                if y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y {
                    draw_switch(
                        canvas,
                        toggle_x,
                        toggle_y,
                        anims.get(switch_idx),
                        *enabled,
                        theme,
                    );
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
                        sep.set_color(theme.separator);
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
            }
            SettingsItem::RowFontPicker {
                label,
                btn_label,
                reset_label,
            } => {
                let visible = y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y;
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + ROW_HEIGHT / 2.0;

                if visible {
                    paint.set_color(theme.text_pri);
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: label,
                        x: row_x,
                        y: cy + 5.0,
                        size: 13.0,
                        bold: false,
                        paint: &paint,
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
            }
            SettingsItem::RowFolderPicker {
                label,
                btn_label,
                clear_label,
                current_path,
                enabled,
            } => {
                let has_path = current_path.as_ref().is_some_and(|p| !p.is_empty());
                let row_h = if has_path { 64.0 } else { ROW_HEIGHT };
                let visible = y + row_h >= visible_min_y && y <= visible_max_y;
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + row_h / 2.0;

                if visible {
                    paint.set_color(if *enabled {
                        theme.text_pri
                    } else {
                        theme.text_sec
                    });
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: label,
                        x: row_x,
                        y: cy + 5.0,
                        size: 13.0,
                        bold: false,
                        paint: &paint,
                    });

                    // Show current path as secondary text on the left
                    if let Some(path) = current_path
                        && !path.is_empty()
                    {
                        paint.set_color(theme.text_sec);
                        let max_w = content_w - GROUP_INNER_PAD * 2.0 - 140.0;
                        let display = truncate_text(fm, path, 11.0, max_w);
                        fm.draw_text_cached(DrawTextCachedParams {
                            canvas,
                            text: &display,
                            x: row_x,
                            y: cy + 17.0,
                            size: 11.0,
                            bold: false,
                            paint: &paint,
                        });
                    }

                    let label_color = if *enabled {
                        theme.text_pri
                    } else {
                        theme.text_sec
                    };
                    let bg_color = if *enabled {
                        theme.card_highlight
                    } else {
                        theme.disabled
                    };

                    let sel_w: f32 = 60.0;
                    let sel_x = CONTENT_PADDING + content_w - GROUP_INNER_PAD - sel_w;
                    draw_pill_btn(PillBtnParams {
                        canvas,
                        x: sel_x,
                        y: cy - 13.0,
                        w: sel_w,
                        h: 26.0,
                        label: btn_label,
                        text_color: label_color,
                        bg_color,
                    });

                    if let Some(cl) = clear_label {
                        let clr_w: f32 = 60.0;
                        let clr_x = sel_x - clr_w - 6.0;
                        draw_pill_btn(PillBtnParams {
                            canvas,
                            x: clr_x,
                            y: cy - 13.0,
                            w: clr_w,
                            h: 26.0,
                            label: cl,
                            text_color: if *enabled {
                                theme.danger
                            } else {
                                theme.text_sec
                            },
                            bg_color,
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
                        canvas.draw_line(
                            (row_x, y + row_h),
                            (CONTENT_PADDING + content_w - GROUP_INNER_PAD, y + row_h),
                            &sep,
                        );
                    }
                }
            }
            SettingsItem::RowSourceSelect {
                label,
                options,
                enabled,
            } => {
                let visible = y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y;
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + ROW_HEIGHT / 2.0;

                if visible {
                    paint.set_color(if *enabled {
                        theme.text_pri
                    } else {
                        theme.text_sec
                    });
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: label,
                        x: row_x,
                        y: cy + 5.0,
                        size: 13.0,
                        bold: false,
                        paint: &paint,
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
                        theme.card_highlight
                    } else {
                        theme.disabled
                    });
                    canvas.draw_round_rect(
                        Rect::from_xywh(btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H),
                        POPUP_BTN_R,
                        POPUP_BTN_R,
                        &p,
                    );

                    p.set_color(if *enabled {
                        theme.text_pri
                    } else {
                        theme.text_sec
                    });
                    let text_w = POPUP_BTN_W - 22.0;
                    fm.draw_text_in_rect(DrawTextInRectParams {
                        canvas,
                        text: selected_label,
                        x: btn_x + 4.0,
                        y: btn_y + 17.0,
                        w: text_w,
                        size: 13.0,
                        bold: false,
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
                        theme.text_sec
                    } else {
                        theme.disabled
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
                        sep.set_color(theme.separator);
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
            }
            SettingsItem::RowButton {
                label,
                btn_label,
                enabled,
            } => {
                let visible = y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y;
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + ROW_HEIGHT / 2.0;

                if visible {
                    paint.set_color(if *enabled {
                        theme.text_pri
                    } else {
                        theme.text_sec
                    });
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: label,
                        x: row_x,
                        y: cy + 5.0,
                        size: 13.0,
                        bold: false,
                        paint: &paint,
                    });

                    let label_color = if *enabled {
                        theme.text_pri
                    } else {
                        theme.text_sec
                    };
                    let bg_color = if *enabled {
                        theme.card_highlight
                    } else {
                        theme.disabled
                    };

                    let btn_x = CONTENT_PADDING + content_w - GROUP_INNER_PAD - POPUP_BTN_W;
                    draw_pill_btn(PillBtnParams {
                        canvas,
                        x: btn_x,
                        y: cy - POPUP_BTN_H / 2.0,
                        w: POPUP_BTN_W,
                        h: POPUP_BTN_H,
                        label: btn_label,
                        text_color: label_color,
                        bg_color,
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
            }
            SettingsItem::RowAppItem {
                label,
                active,
                enabled,
            } => {
                let visible = y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y;
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + ROW_HEIGHT / 2.0;

                let check_size = 20.0;
                let check_x = CONTENT_PADDING + content_w - GROUP_INNER_PAD - check_size;
                let check_y = cy - check_size / 2.0;

                let mut p = Paint::default();
                p.set_anti_alias(true);
                if visible && *active && *enabled {
                    p.set_color(theme.accent);
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
                        theme.card_highlight
                    } else {
                        theme.disabled
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
                        theme.text_pri
                    } else {
                        theme.text_sec
                    });
                    let max_label_w = check_x - row_x - 8.0;
                    let display = truncate_text(fm, label, 13.0, max_label_w);
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: &display,
                        x: row_x,
                        y: cy + 5.0,
                        size: 13.0,
                        bold: false,
                        paint: &paint,
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
            }
            SettingsItem::RowLabel { label } => {
                let visible = y + ROW_HEIGHT >= visible_min_y && y <= visible_max_y;
                let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                let cy = y + ROW_HEIGHT / 2.0;
                if visible {
                    paint.set_color(theme.text_sec);
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: label,
                        x: row_x,
                        y: cy + 5.0,
                        size: 13.0,
                        bold: false,
                        paint: &paint,
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
            }
            SettingsItem::CenterLink { label, color } => {
                let h = item.height();
                if y + h >= visible_min_y && y <= visible_max_y {
                    paint.set_color(*color);
                    let link_w = fm.measure_text_cached(label, 13.0, FontStyle::normal());
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text: label,
                        x: width / 2.0 - link_w / 2.0,
                        y: y + 24.0,
                        size: 13.0,
                        bold: false,
                        paint: &paint,
                    });
                }
            }
            SettingsItem::CenterText { text, size, color } => {
                let h = item.height();
                if y + h >= visible_min_y && y <= visible_max_y {
                    paint.set_color(*color);
                    let ct_w = fm.measure_text_cached(text, *size, FontStyle::normal());
                    fm.draw_text_cached(DrawTextCachedParams {
                        canvas,
                        text,
                        x: width / 2.0 - ct_w / 2.0,
                        y: y + 22.0,
                        size: *size,
                        bold: false,
                        paint: &paint,
                    });
                }
            }
            SettingsItem::Spacer { .. } => {}
            SettingsItem::FontPreview { has_custom_font } => {
                let preview_h = 50.0;
                let top_pad = (70.0 - preview_h) / 2.0;
                let py = y + top_pad;
                let visible = py + preview_h >= visible_min_y && py <= visible_max_y;
                if visible {
                    let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
                    let preview_w = content_w - GROUP_INNER_PAD * 2.0;

                    let mut bg_p = Paint::default();
                    bg_p.set_anti_alias(true);
                    bg_p.set_color(theme.card_highlight);
                    canvas.draw_round_rect(
                        Rect::from_xywh(row_x, py, preview_w, preview_h),
                        8.0,
                        8.0,
                        &bg_p,
                    );

                    let mut label_p = Paint::default();
                    label_p.set_anti_alias(true);
                    label_p.set_color(theme.text_sec);
                    fm.draw_text_with_default_font(
                        canvas,
                        &tr("font_preview_default"),
                        (row_x + 8.0, py + 14.0),
                        11.0,
                        false,
                        &label_p,
                    );

                    label_p.set_color(theme.text_pri);
                    let default_samples = [tr("font_preview_sample")];
                    for (si, sample) in default_samples.iter().enumerate() {
                        fm.draw_text_with_default_font(
                            canvas,
                            sample,
                            (row_x + 8.0, py + 34.0 + si as f32 * 18.0),
                            14.0,
                            false,
                            &label_p,
                        );
                    }

                    if *has_custom_font {
                        let div_x = row_x + preview_w / 2.0;
                        let mut div_p = Paint::default();
                        div_p.set_anti_alias(true);
                        div_p.set_color(theme.separator);
                        div_p.set_stroke_width(1.0);
                        div_p.set_style(skia_safe::paint::Style::Stroke);
                        canvas.draw_line((div_x, py + 6.0), (div_x, py + preview_h - 6.0), &div_p);

                        label_p.set_color(theme.text_sec);
                        fm.draw_text_cached(DrawTextCachedParams {
                            canvas,
                            text: &tr("font_preview_custom"),
                            x: div_x + 8.0,
                            y: py + 14.0,
                            size: 11.0,
                            bold: false,
                            paint: &label_p,
                        });

                        label_p.set_color(theme.accent);
                        let custom_samples = [tr("font_preview_sample")];
                        for (si, sample) in custom_samples.iter().enumerate() {
                            fm.draw_text_with_custom_font(
                                canvas,
                                sample,
                                (div_x + 8.0, py + 34.0 + si as f32 * 18.0),
                                14.0,
                                false,
                                &label_p,
                            );
                        }
                    }
                }
            }
            SettingsItem::WidgetPreview => {
                draw_widget_preview(WidgetPreviewParams {
                    canvas,
                    item_y: y,
                    width,
                    content_width: content_w,
                    visible_min_y,
                    visible_max_y,
                    island_style,
                    adaptive_border,
                    expanded_width,
                    expanded_height,
                    widget_layout,
                    widget_dragging,
                    widget_drag_hover_slot,
                    widget_preview_hover_slot,
                    theme,
                });
            }
        }
        y += item.height();
        i += 1;
    }
}

fn group_height_from(items: &[SettingsItem], start: usize) -> f32 {
    let mut h = 0.0;
    for item in &items[start..] {
        if matches!(item, SettingsItem::GroupEnd) {
            break;
        }
        h += item.height();
    }
    h
}
