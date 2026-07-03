use crate::core::i18n::tr;
use crate::utils::color::SettingsTheme;
use crate::utils::font::{DrawTextCachedParams, FontManager};
use crate::utils::settings_ui::items::*;
use skia_safe::{Canvas, Color, Paint, Rect};

use super::{SIDEBAR_KEY_BASE, SIDEBAR_ROW_H, SIDEBAR_W, SettingsApp};

impl SettingsApp {
    pub(crate) fn draw_sidebar(&self, canvas: &Canvas, theme: &SettingsTheme) {
        let fm = FontManager::global();
        let mut paint = Paint::default();
        paint.set_anti_alias(true);

        paint.set_color(theme.sidebar_bg);
        canvas.draw_rect(Rect::from_xywh(0.0, 0.0, SIDEBAR_W, self.win_h), &paint);

        // Draw Apple-style Window Control Dots
        let (red_color, yellow_color, green_color) = if self.focused {
            (
                Color::from_rgb(0xFF, 0x5F, 0x56),
                Color::from_rgb(0xFF, 0xBD, 0x2E),
                Color::from_rgb(0x27, 0xC9, 0x3F),
            )
        } else if self.is_light {
            (
                Color::from_rgb(0xE6, 0xE6, 0xE6),
                Color::from_rgb(0xE6, 0xE6, 0xE6),
                Color::from_rgb(0xE6, 0xE6, 0xE6),
            )
        } else {
            (
                Color::from_rgb(0x4D, 0x4D, 0x4D),
                Color::from_rgb(0x4D, 0x4D, 0x4D),
                Color::from_rgb(0x4D, 0x4D, 0x4D),
            )
        };

        let radius = 6.0;
        let red_center = (20.0, 24.0);
        let yellow_center = (40.0, 24.0);
        let green_center = (60.0, 24.0);

        paint.set_color(red_color);
        canvas.draw_circle(red_center, radius, &paint);

        paint.set_color(yellow_color);
        canvas.draw_circle(yellow_center, radius, &paint);

        paint.set_color(green_color);
        canvas.draw_circle(green_center, radius, &paint);

        // Draw symbols if hovered and focused
        if self.dots_hovered && self.focused {
            let mut sym_paint = Paint::default();
            sym_paint.set_anti_alias(true);
            sym_paint.set_style(skia_safe::paint::Style::Stroke);
            sym_paint.set_stroke_width(1.0);

            // Red Close cross: x
            sym_paint.set_color(Color::from_rgb(0x4C, 0x00, 0x02));
            canvas.draw_line((17.5, 21.5), (22.5, 26.5), &sym_paint);
            canvas.draw_line((22.5, 21.5), (17.5, 26.5), &sym_paint);

            // Yellow Minimize line: -
            sym_paint.set_color(Color::from_rgb(0x5C, 0x3E, 0x00));
            canvas.draw_line((36.5, 24.0), (43.5, 24.0), &sym_paint);

            // Green Maximize plus: +
            sym_paint.set_color(Color::from_rgb(0x00, 0x4D, 0x02));
            canvas.draw_line((57.0, 24.0), (63.0, 24.0), &sym_paint);
            canvas.draw_line((60.0, 21.0), (60.0, 27.0), &sym_paint);
        }

        let mut sep = Paint::default();
        sep.set_anti_alias(true);
        sep.set_color(theme.separator);
        sep.set_stroke_width(0.5);
        sep.set_style(skia_safe::paint::Style::Stroke);
        canvas.draw_line((SIDEBAR_W, 0.0), (SIDEBAR_W, self.win_h), &sep);

        let pages = [
            tr("tab_general"),
            tr("tab_music"),
            tr("tab_widgets"),
            tr("tab_about"),
        ];
        let start_y = 60.0;

        for (i, label) in pages.iter().enumerate() {
            let row_y = start_y + i as f32 * (SIDEBAR_ROW_H + 2.0);
            let row_x = SIDEBAR_PAD;
            let row_w = SIDEBAR_W - SIDEBAR_PAD * 2.0;

            if self.active_page == i {
                paint.set_color(theme.accent);
                canvas.draw_round_rect(
                    Rect::from_xywh(row_x, row_y, row_w, SIDEBAR_ROW_H),
                    SIDEBAR_SEL_RADIUS,
                    SIDEBAR_SEL_RADIUS,
                    &paint,
                );
                paint.set_color(Color::WHITE);
            } else {
                let hover_val = self.anim.get(SIDEBAR_KEY_BASE + i as u64);
                if hover_val > 0.005 {
                    let base = theme.sidebar_hover;
                    let alpha = (base.a() as f32 * hover_val) as u8;
                    paint.set_color(Color::from_argb(alpha, base.r(), base.g(), base.b()));
                    canvas.draw_round_rect(
                        Rect::from_xywh(row_x, row_y, row_w, SIDEBAR_ROW_H),
                        SIDEBAR_SEL_RADIUS,
                        SIDEBAR_SEL_RADIUS,
                        &paint,
                    );
                }
                paint.set_color(theme.text_sec);
            }

            // Draw macOS-style icon next to text
            let icon_bg_rect = Rect::from_xywh(row_x + 8.0, row_y + 6.0, 20.0, 20.0);
            let mut icon_bg_paint = Paint::default();
            icon_bg_paint.set_anti_alias(true);

            match i {
                0 => {
                    icon_bg_paint.set_color(Color::from_rgb(142, 142, 147)); // Gray
                    canvas.draw_round_rect(icon_bg_rect, 5.0, 5.0, &icon_bg_paint);
                    crate::icons::settings::draw_settings_icon(
                        canvas,
                        row_x + 18.0,
                        row_y + 16.0,
                        255,
                        0.5,
                        Color::WHITE,
                    );
                }
                1 => {
                    icon_bg_paint.set_color(Color::from_rgb(252, 60, 68)); // Pink/Red
                    canvas.draw_round_rect(icon_bg_rect, 5.0, 5.0, &icon_bg_paint);
                    crate::icons::music::draw_music_icon(
                        canvas,
                        row_x + 18.0,
                        row_y + 16.0,
                        255,
                        0.5,
                        Color::WHITE,
                    );
                }
                2 => {
                    icon_bg_paint.set_color(Color::from_rgb(52, 199, 89)); // Green
                    canvas.draw_round_rect(icon_bg_rect, 5.0, 5.0, &icon_bg_paint);

                    let mut widget_paint = Paint::default();
                    widget_paint.set_anti_alias(true);
                    widget_paint.set_color(Color::WHITE);
                    // Draw a 2x2 grid representing widgets
                    canvas.draw_round_rect(
                        Rect::from_xywh(row_x + 12.0, row_y + 10.0, 5.0, 5.0),
                        1.0,
                        1.0,
                        &widget_paint,
                    );
                    canvas.draw_round_rect(
                        Rect::from_xywh(row_x + 19.0, row_y + 10.0, 5.0, 5.0),
                        1.0,
                        1.0,
                        &widget_paint,
                    );
                    canvas.draw_round_rect(
                        Rect::from_xywh(row_x + 12.0, row_y + 17.0, 5.0, 5.0),
                        1.0,
                        1.0,
                        &widget_paint,
                    );
                    canvas.draw_round_rect(
                        Rect::from_xywh(row_x + 19.0, row_y + 17.0, 5.0, 5.0),
                        1.0,
                        1.0,
                        &widget_paint,
                    );
                }
                3 => {
                    icon_bg_paint.set_color(Color::from_rgb(0, 122, 255)); // Royal Blue
                    canvas.draw_round_rect(icon_bg_rect, 5.0, 5.0, &icon_bg_paint);

                    let mut info_paint = Paint::default();
                    info_paint.set_anti_alias(true);
                    info_paint.set_color(Color::WHITE);
                    info_paint.set_stroke_width(1.2);

                    info_paint.set_style(skia_safe::paint::Style::Stroke);
                    canvas.draw_circle((row_x + 18.0, row_y + 16.0), 6.5, &info_paint);

                    info_paint.set_style(skia_safe::paint::Style::Fill);
                    canvas.draw_circle((row_x + 18.0, row_y + 13.0), 0.8, &info_paint);

                    info_paint.set_style(skia_safe::paint::Style::Stroke);
                    canvas.draw_line(
                        (row_x + 18.0, row_y + 15.0),
                        (row_x + 18.0, row_y + 18.5),
                        &info_paint,
                    );
                }
                _ => {}
            }

            fm.draw_text_cached(DrawTextCachedParams {
                canvas,
                text: label,
                x: row_x + 36.0,
                y: row_y + 21.0,
                size: 13.0,
                bold: false,
                paint: &paint,
            });
        }
    }
}
