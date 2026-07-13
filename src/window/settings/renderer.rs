use crate::core::i18n::tr;
use crate::ui::widget::draw_mini_card;
use crate::utils::color::SettingsTheme;
use crate::utils::font::{DrawTextCachedParams, FontManager};
use crate::utils::settings_ui::items::*;
use crate::utils::settings_ui::*;
use skia_safe::{Canvas, Color, Paint, Rect, surfaces};

use super::{
    CONTENT_START_Y, POPUP_MENU_R, POPUP_OPACITY_KEY, SIDEBAR_W, SUB_TAB_H, SUB_TAB_START_Y,
    SettingsApp,
};

impl SettingsApp {
    pub(crate) fn draw(&mut self) {
        let Some(win) = self.window.as_ref() else {
            return;
        };
        let (p_w, p_h, scale) = {
            let size = win.inner_size();
            (
                size.width as i32,
                size.height as i32,
                win.scale_factor() as f32,
            )
        };
        if p_w <= 0 || p_h <= 0 {
            return;
        }

        self.ensure_items_cache();
        let theme = self.theme();
        let win_w = self.win_w / scale;
        let win_h = self.win_h / scale;
        let anim = self.switch_anim.clone();

        let mut surface = match self.surface.take() {
            Some(s) => s,
            None => return,
        };

        {
            let mut buffer = match surface.buffer_mut() {
                Ok(b) => b,
                Err(_) => {
                    self.surface = Some(surface);
                    return;
                }
            };
            let info = skia_safe::ImageInfo::new(
                skia_safe::ISize::new(p_w, p_h),
                skia_safe::ColorType::BGRA8888,
                skia_safe::AlphaType::Premul,
                None,
            );
            let dst_row_bytes = (p_w * 4) as usize;
            let u8_buffer: &mut [u8] = bytemuck::cast_slice_mut(&mut buffer);
            let expected_size = (p_w * p_h * 4) as usize;
            let actual_size = u8_buffer.len();
            if actual_size != expected_size {
                return;
            }
            let mut sk_surface = match surfaces::wrap_pixels(&info, u8_buffer, dst_row_bytes, None)
            {
                Some(s) => s,
                None => {
                    return;
                }
            };

            let canvas = sk_surface.canvas();
            canvas.reset_matrix();
            canvas.clear(Color::TRANSPARENT);
            canvas.scale((scale, scale));

            let win_rect = Rect::from_xywh(0.0, 0.0, win_w, win_h);
            let win_rrect = skia_safe::RRect::new_rect_xy(win_rect, 12.0, 12.0);

            canvas.save();
            canvas.clip_rrect(win_rrect, skia_safe::ClipOp::Intersect, true);

            let mut bg_paint = Paint::default();
            bg_paint.set_anti_alias(true);
            bg_paint.set_color(theme.win_bg);
            canvas.draw_rect(win_rect, &bg_paint);

            self.draw_sidebar(canvas, &theme);

            let content_w = win_w - SIDEBAR_W;
            self.draw_sub_tabs(canvas, &theme, content_w);

            let content_start_y = if self.active_page == 0 {
                SUB_TAB_START_Y + SUB_TAB_H + CONTENT_START_Y
            } else {
                50.0
            };

            self.target_scroll_y = self.target_scroll_y.clamp(0.0, self.cached_max_scroll);

            let clip_start_y = if self.active_page == 0 {
                SUB_TAB_START_Y + SUB_TAB_H
            } else {
                50.0
            };

            canvas.save();
            canvas.clip_rect(
                Rect::from_xywh(SIDEBAR_W, clip_start_y, content_w, win_h - clip_start_y),
                skia_safe::ClipOp::Intersect,
                true,
            );
            canvas.translate((SIDEBAR_W, -self.scroll_y));
            draw_items(DrawItemsParams {
                canvas,
                items: &self.cached_items,
                start_y: content_start_y,
                width: content_w,
                anims: &anim,
                theme: &theme,
                visible_min_y: self.scroll_y,
                visible_max_y: self.scroll_y + win_h,
                island_style: &self.config.island_style,
                adaptive_border: self.config.adaptive_border,
                expanded_width: self.config.expanded_width,
                expanded_height: self.config.expanded_height,
                widget_layout: &self.config.widget_layout,
                widget_dragging: self.widget_dragging,
                widget_drag_hover_slot: self.widget_drag_hover_slot,
                widget_preview_hover_slot: self.widget_preview_hover_slot,
            });
            canvas.restore();

            let ch = self.cached_content_height;
            let view_h = win_h;
            if ch > view_h {
                let bar_h = (view_h / ch) * view_h;
                let bar_y = (self.scroll_y / (ch - view_h)) * (view_h - bar_h);
                let mut p = Paint::default();
                p.set_anti_alias(true);
                p.set_color(Color::from_argb(60, 255, 255, 255));
                canvas.draw_round_rect(
                    Rect::from_xywh(win_w - 6.0, bar_y, 4.0, bar_h),
                    2.0,
                    2.0,
                    &p,
                );
            }

            self.draw_popup(canvas, &theme);
            self.draw_widget_drag_overlay(canvas, &theme, win_w, win_h);
            canvas.restore();

            // Draw a subtle rounded border around the window
            let border_rect = Rect::from_xywh(0.5, 0.5, win_w - 1.0, win_h - 1.0);
            let border_rrect = skia_safe::RRect::new_rect_xy(border_rect, 11.5, 11.5);
            let mut border_paint = Paint::default();
            border_paint.set_anti_alias(true);
            border_paint.set_style(skia_safe::paint::Style::Stroke);
            border_paint.set_stroke_width(1.0);
            border_paint.set_color(theme.separator);
            canvas.draw_rrect(border_rrect, &border_paint);

            let _ = buffer.present();
        }

        self.surface = Some(surface);
    }

    fn widget_preview_item_y_cached(&self) -> Option<f32> {
        if self.active_page != 2 {
            return None;
        }
        let mut y = 50.0;
        for item in &self.cached_items {
            if matches!(item, SettingsItem::WidgetPreview) {
                return Some(y);
            }
            y += item.height();
        }
        None
    }

    fn draw_widget_drag_overlay(
        &self,
        canvas: &Canvas,
        _theme: &SettingsTheme,
        win_w: f32,
        win_h: f32,
    ) {
        let Some(widget) = self.widget_dragging else {
            return;
        };

        let (w, h) = self
            .widget_preview_item_y_cached()
            .map(|item_y| {
                let scale = self
                    .window
                    .as_ref()
                    .map(|w| w.scale_factor() as f32)
                    .unwrap_or(1.0);
                let width = self.win_w / scale - SIDEBAR_W;
                let geom = widget_grid_geom(
                    item_y,
                    width,
                    self.config.expanded_width,
                    self.config.expanded_height,
                );
                let (_, _, w, h) = geom.footprint_rect(widget, 0);
                (w.max(60.0), h.max(48.0))
            })
            .unwrap_or((96.0, 96.0));

        let (mx, my) = self.logical_mouse_pos;
        let x = (mx - w / 2.0).clamp(8.0, win_w - w - 8.0);
        let y = (my - h / 2.0).clamp(8.0, win_h - h - 8.0);

        let mut shadow = Paint::default();
        shadow.set_anti_alias(true);
        shadow.set_color(Color::from_argb(90, 0, 0, 0));
        canvas.draw_round_rect(Rect::from_xywh(x, y + 4.0, w, h), 12.0, 12.0, &shadow);

        draw_mini_card(canvas, widget, x, y, w, h);
    }

    pub(crate) fn draw_sub_tabs(&self, canvas: &Canvas, theme: &SettingsTheme, content_w: f32) {
        if self.active_page != 0 {
            return;
        }

        let fm = FontManager::global();
        let tabs = [
            tr("section_appearance"),
            tr("section_effects"),
            tr("section_behavior"),
        ];
        let tab_w = content_w / tabs.len() as f32;
        let start_x = SIDEBAR_W;

        let mut paint = Paint::default();
        paint.set_anti_alias(true);

        paint.set_color(theme.text_pri);
        fm.draw_text_cached(DrawTextCachedParams {
            canvas,
            text: &tr("tab_general"),
            x: SIDEBAR_W + CONTENT_PADDING,
            y: 35.0,
            size: 20.0,
            bold: true,
            paint: &paint,
        });

        let mut sep = Paint::default();
        sep.set_anti_alias(true);
        sep.set_color(theme.separator);
        sep.set_stroke_width(0.5);
        sep.set_style(skia_safe::paint::Style::Stroke);
        canvas.draw_line(
            (SIDEBAR_W, SUB_TAB_START_Y + SUB_TAB_H),
            (SIDEBAR_W + content_w, SUB_TAB_START_Y + SUB_TAB_H),
            &sep,
        );

        for (i, label) in tabs.iter().enumerate() {
            let tab_x = start_x + i as f32 * tab_w;
            let is_active = self.active_sub_page == i;
            let is_hover = self.sub_tab_hover == i as i32;

            paint.set_color(if is_active || is_hover {
                theme.text_pri
            } else {
                theme.text_sec
            });

            let label_w = FontManager::global().measure_text_cached(
                label,
                13.0,
                skia_safe::FontStyle::normal(),
            );
            let text_x = tab_x + (tab_w - label_w) / 2.0;
            let text_y = SUB_TAB_START_Y + SUB_TAB_H / 2.0 + 5.0;
            fm.draw_text_cached(DrawTextCachedParams {
                canvas,
                text: label,
                x: text_x,
                y: text_y,
                size: 13.0,
                bold: false,
                paint: &paint,
            });

            if is_active {
                let underline_pad = 4.0;
                let underline_x = text_x - underline_pad;
                let underline_w = label_w + underline_pad * 2.0;
                let underline_y = SUB_TAB_START_Y + SUB_TAB_H - 2.0;
                paint.set_style(skia_safe::paint::Style::Fill);
                canvas.draw_rect(
                    Rect::from_xywh(underline_x, underline_y, underline_w, 2.0),
                    &paint,
                );
            }
        }
    }

    pub(crate) fn draw_popup(&self, canvas: &Canvas, theme: &SettingsTheme) {
        let popup = match &self.popup {
            Some(p) => p,
            None => return,
        };
        let opacity = self.anim.get(POPUP_OPACITY_KEY);
        if opacity < 0.005 {
            return;
        }
        let fm = FontManager::global();
        let menu = popup.menu_rect();

        let mut shadow = Paint::default();
        shadow.set_anti_alias(true);
        shadow.set_color(Color::from_argb((60.0 * opacity) as u8, 0, 0, 0));
        canvas.draw_round_rect(
            Rect::from_xywh(
                menu.left - 1.0,
                menu.top + 2.0,
                menu.width() + 2.0,
                menu.height() + 2.0,
            ),
            POPUP_MENU_R,
            POPUP_MENU_R,
            &shadow,
        );

        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_color(Color::from_argb(
            (255.0 * opacity) as u8,
            theme.popup_bg.r(),
            theme.popup_bg.g(),
            theme.popup_bg.b(),
        ));
        canvas.draw_round_rect(menu, POPUP_MENU_R, POPUP_MENU_R, &paint);

        let mut border = Paint::default();
        border.set_anti_alias(true);
        border.set_color(Color::from_argb(
            (40.0 * opacity) as u8,
            theme.popup_border.r(),
            theme.popup_border.g(),
            theme.popup_border.b(),
        ));
        border.set_style(skia_safe::paint::Style::Stroke);
        border.set_stroke_width(0.5);
        canvas.draw_round_rect(menu, POPUP_MENU_R, POPUP_MENU_R, &border);

        let text_alpha = (255.0 * opacity) as u8;
        for (i, opt_label) in popup.options.iter().enumerate() {
            let item_rect = popup.item_rect(i);

            if popup.hover_idx == Some(i) {
                let a = theme.accent.a() as f32 * opacity;
                paint.set_color(Color::from_argb(
                    a as u8,
                    theme.accent.r(),
                    theme.accent.g(),
                    theme.accent.b(),
                ));
                paint.set_style(skia_safe::paint::Style::Fill);
                canvas.draw_round_rect(item_rect, 4.0, 4.0, &paint);
            }

            paint.set_color(Color::from_argb(
                text_alpha,
                theme.text_pri.r(),
                theme.text_pri.g(),
                theme.text_pri.b(),
            ));
            paint.set_style(skia_safe::paint::Style::Fill);
            fm.draw_text_cached(DrawTextCachedParams {
                canvas,
                text: opt_label,
                x: item_rect.left + 8.0,
                y: item_rect.top + 19.0,
                size: 12.0,
                bold: false,
                paint: &paint,
            });

            if i == popup.selected_idx {
                let check_base = if popup.hover_idx == Some(i) {
                    theme.text_pri
                } else {
                    theme.accent
                };
                paint.set_color(Color::from_argb(
                    text_alpha,
                    check_base.r(),
                    check_base.g(),
                    check_base.b(),
                ));
                paint.set_style(skia_safe::paint::Style::Stroke);
                paint.set_stroke_width(2.0);
                let cx = item_rect.right - 14.0;
                let cy = item_rect.top + POPUP_ITEM_H / 2.0;
                let svg = format!(
                    "M {} {} L {} {} L {} {}",
                    cx - 4.0,
                    cy,
                    cx - 1.0,
                    cy + 3.0,
                    cx + 4.0,
                    cy - 3.0,
                );
                if let Some(path) = skia_safe::Path::from_svg(&svg) {
                    canvas.draw_path(&path, &paint);
                }
                paint.set_style(skia_safe::paint::Style::Fill);
            }

            if i < popup.options.len() - 1 {
                let mut sep = Paint::default();
                sep.set_anti_alias(true);
                sep.set_color(Color::from_argb(
                    (30.0 * opacity) as u8,
                    theme.separator.r(),
                    theme.separator.g(),
                    theme.separator.b(),
                ));
                sep.set_stroke_width(0.5);
                sep.set_style(skia_safe::paint::Style::Stroke);
                canvas.draw_line(
                    (item_rect.left, item_rect.bottom),
                    (item_rect.right, item_rect.bottom),
                    &sep,
                );
            }
        }
    }
}
