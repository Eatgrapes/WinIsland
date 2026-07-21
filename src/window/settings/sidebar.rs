use std::cell::RefCell;

use crate::core::i18n::tr;
use crate::utils::color::SettingsTheme;
use crate::utils::font::{DrawTextCachedParams, FontManager};
use crate::utils::settings_ui::items::*;
use skia_safe::{Canvas, Color, CubicResampler, Data, Image, Paint, Rect, SamplingOptions};

use super::{SIDEBAR_KEY_BASE, SIDEBAR_ROW_H, SIDEBAR_W, SettingsApp};

const SIDEBAR_ICON_BYTES: [&[u8]; 4] = [
    include_bytes!("../../../resources/in_app/settings/settings.png"),
    include_bytes!("../../../resources/in_app/settings/music.png"),
    include_bytes!("../../../resources/in_app/settings/widget.png"),
    include_bytes!("../../../resources/in_app/settings/about.png"),
];

thread_local! {
    static SIDEBAR_ICONS: RefCell<Option<[Image; 4]>> = const { RefCell::new(None) };
}

fn load_sidebar_icon(bytes: &[u8]) -> Image {
    Image::from_encoded(Data::new_copy(bytes)).expect("Failed to load sidebar icon")
}

fn draw_sidebar_icon(canvas: &Canvas, index: usize, rect: Rect) {
    SIDEBAR_ICONS.with(|cache| {
        let mut cache = cache.borrow_mut();
        let icons = cache.get_or_insert_with(|| SIDEBAR_ICON_BYTES.map(load_sidebar_icon));
        let paint = Paint::default();
        canvas.draw_image_rect_with_sampling_options(
            &icons[index],
            None,
            rect,
            SamplingOptions::from(CubicResampler::mitchell()),
            &paint,
        );
    });
}

pub(super) fn clear_sidebar_icon_cache() {
    SIDEBAR_ICONS.with(|cache| {
        *cache.borrow_mut() = None;
    });
}

impl SettingsApp {
    pub(crate) fn draw_sidebar(&self, canvas: &Canvas, theme: &SettingsTheme) {
        let fm = FontManager::global();
        let mut paint = Paint::default();
        paint.set_anti_alias(true);

        paint.set_color(theme.sidebar_bg);
        canvas.draw_rect(Rect::from_xywh(0.0, 0.0, SIDEBAR_W, self.win_h), &paint);

        // Draw Apple-style Window Control Dots
        let (red_color, yellow_color) = if self.focused {
            (
                Color::from_rgb(0xFF, 0x5F, 0x56),
                Color::from_rgb(0xFF, 0xBD, 0x2E),
            )
        } else if self.is_light {
            (
                Color::from_rgb(0xD4, 0x8A, 0x84),
                Color::from_rgb(0xD1, 0xB0, 0x78),
            )
        } else {
            (
                Color::from_rgb(0x4D, 0x4D, 0x4D),
                Color::from_rgb(0x4D, 0x4D, 0x4D),
            )
        };
        let disabled_control = if self.focused {
            Color::from_rgb(142, 142, 147)
        } else if self.is_light {
            Color::from_rgb(190, 190, 195)
        } else {
            Color::from_rgb(77, 77, 77)
        };

        let radius = 6.0;
        let red_center = (20.0, 24.0);
        let yellow_center = (40.0, 24.0);
        let green_center = (60.0, 24.0);

        paint.set_color(red_color);
        canvas.draw_circle(red_center, radius, &paint);

        paint.set_color(yellow_color);
        canvas.draw_circle(yellow_center, radius, &paint);

        paint.set_color(disabled_control);
        canvas.draw_circle(green_center, radius, &paint);

        if self.dots_hovered {
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

            draw_sidebar_icon(
                canvas,
                i,
                Rect::from_xywh(row_x + 8.0, row_y + 6.0, 20.0, 20.0),
            );

            fm.draw_text_cached(DrawTextCachedParams {
                canvas,
                text: label,
                x: row_x + 36.0,
                y: row_y + 21.0,
                size: 13.0,
                bold: true,
                paint: &paint,
            });
        }
    }
}
