use std::cell::RefCell;
use std::collections::HashMap;

use crate::core::i18n::tr;
use crate::plugin::manager::InstalledPlugin;
use crate::utils::color::SettingsTheme;
use crate::utils::font::{DrawTextCachedParams, FontManager};
use crate::utils::settings_ui::items::{CONTENT_PADDING, SettingsItem};
use skia_safe::{
    Canvas, ClipOp, Color, Contains, Data, FontStyle, Image, Paint, Point, RRect, Rect,
};

use super::super::{
    PLUGIN_DETAIL_KEY, PluginSettingsRequest, SETTINGS_HEADER_H, SIDEBAR_W, SettingsApp,
};

mod detail;
mod markdown;

const PLUGIN_CARD_H: f32 = 76.0;
const PLUGIN_CARD_GAP: f32 = 10.0;
const DETAIL_W: f32 = 350.0;
const DETAIL_ICON_SIZE: f32 = 64.0;

thread_local! {
    static PLUGIN_ICONS: RefCell<HashMap<String, Image>> = RefCell::new(HashMap::new());
}

pub(crate) fn clear_plugin_icon_cache() {
    PLUGIN_ICONS.with(|cache| cache.borrow_mut().clear());
    markdown::clear_cache();
}

impl SettingsApp {
    pub(crate) fn build_plugin_items(&self) -> Vec<SettingsItem> {
        let height = 52.0
            + self.plugins.len() as f32 * (PLUGIN_CARD_H + PLUGIN_CARD_GAP)
            + if self.plugin_status.is_some() {
                62.0
            } else {
                0.0
            }
            + 24.0;
        vec![SettingsItem::Custom { height }]
    }

    pub(crate) fn draw_plugins_page(
        &mut self,
        direct_context: &mut skia_safe::gpu::DirectContext,
        canvas: &Canvas,
        theme: &SettingsTheme,
        width: f32,
        height: f32,
    ) {
        let content_width = width - SIDEBAR_W;
        canvas.save();
        canvas.clip_rect(
            Rect::from_xywh(
                SIDEBAR_W,
                SETTINGS_HEADER_H,
                content_width,
                height - SETTINGS_HEADER_H,
            ),
            skia_safe::ClipOp::Intersect,
            true,
        );
        canvas.translate((SIDEBAR_W, -self.scroll_y));
        self.draw_plugin_list(direct_context, canvas, theme, content_width);
        canvas.restore();

        let detail_progress = self.anim.get(PLUGIN_DETAIL_KEY);
        if detail_progress > 0.005 {
            self.draw_plugin_detail(
                direct_context,
                canvas,
                theme,
                width,
                height,
                detail_progress,
            );
        }
    }

    fn draw_plugin_list(
        &self,
        direct_context: &mut skia_safe::gpu::DirectContext,
        canvas: &Canvas,
        theme: &SettingsTheme,
        width: f32,
    ) {
        let fm = FontManager::global();
        let content_w = width - CONTENT_PADDING * 2.0;
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        let mut y = SETTINGS_HEADER_H + 18.0;
        paint.set_color(theme.text_pri);
        fm.draw_text_cached(DrawTextCachedParams {
            canvas,
            text: &tr("plugin_installed"),
            x: CONTENT_PADDING + 4.0,
            y: y + 18.0,
            size: 13.0,
            bold: true,
            paint: &paint,
        });
        y += 34.0;

        if self.plugins.is_empty() {
            paint.set_color(theme.text_sec);
            draw_centered_text(
                canvas,
                fm,
                &tr("plugin_empty"),
                width / 2.0,
                y + 38.0,
                13.0,
                false,
                &paint,
            );
        }

        for plugin in &self.plugins {
            let card = Rect::from_xywh(CONTENT_PADDING, y, content_w, PLUGIN_CARD_H);
            let hovered = card.contains(Point::new(
                self.logical_mouse_pos.0 - SIDEBAR_W,
                self.logical_mouse_pos.1 + self.scroll_y,
            ));
            paint.set_color(if hovered {
                theme.card_highlight
            } else {
                theme.group_bg
            });
            canvas.draw_round_rect(card, 13.0, 13.0, &paint);
            paint.set_style(skia_safe::paint::Style::Stroke);
            paint.set_stroke_width(0.75);
            paint.set_color(theme.group_border);
            canvas.draw_round_rect(
                Rect::from_xywh(
                    card.left + 0.375,
                    card.top + 0.375,
                    card.width() - 0.75,
                    card.height() - 0.75,
                ),
                12.625,
                12.625,
                &paint,
            );
            paint.set_style(skia_safe::paint::Style::Fill);

            draw_plugin_icon(
                direct_context,
                canvas,
                plugin,
                Rect::from_xywh(card.left + 12.0, card.top + 12.0, 52.0, 52.0),
            );
            paint.set_color(theme.text_pri);
            let name = ellipsize_text(
                fm,
                &plugin.name,
                14.0,
                FontStyle::bold(),
                (card.width() - 142.0).max(20.0),
            );
            fm.draw_text_cached(DrawTextCachedParams {
                canvas,
                text: &name,
                x: card.left + 76.0,
                y: card.top + 29.0,
                size: 14.0,
                bold: true,
                paint: &paint,
            });
            paint.set_color(theme.text_sec);
            let subtitle = format!("{} · v{}", plugin.author, plugin.version);
            let subtitle = ellipsize_text(
                fm,
                &subtitle,
                11.5,
                FontStyle::normal(),
                (card.width() - 142.0).max(20.0),
            );
            fm.draw_text_cached(DrawTextCachedParams {
                canvas,
                text: &subtitle,
                x: card.left + 76.0,
                y: card.top + 51.0,
                size: 11.5,
                bold: false,
                paint: &paint,
            });
            draw_toggle(
                canvas,
                theme,
                plugin.enabled,
                card.right - 50.0,
                card.top + 28.0,
            );
            y += PLUGIN_CARD_H + PLUGIN_CARD_GAP;
        }

        if let Some((message, restart)) = &self.plugin_status {
            let status = Rect::from_xywh(CONTENT_PADDING, y + 6.0, content_w, 48.0);
            paint.set_color(Color::from_argb(
                28,
                theme.accent.r(),
                theme.accent.g(),
                theme.accent.b(),
            ));
            canvas.draw_round_rect(status, 12.0, 12.0, &paint);
            let label = restart.then(|| tr("plugin_restart_now"));
            let label_w = label.as_ref().map_or(0.0, |label| {
                fm.measure_text_cached(label, 12.0, FontStyle::bold())
            });
            let message = ellipsize_text(
                fm,
                message,
                12.0,
                FontStyle::normal(),
                (status.width() - 28.0 - label_w - if *restart { 24.0 } else { 0.0 }).max(20.0),
            );
            paint.set_color(theme.text_pri);
            fm.draw_text_cached(DrawTextCachedParams {
                canvas,
                text: &message,
                x: status.left + 14.0,
                y: status.top + 29.0,
                size: 12.0,
                bold: false,
                paint: &paint,
            });
            if let Some(label) = label {
                paint.set_color(theme.accent);
                fm.draw_text_cached(DrawTextCachedParams {
                    canvas,
                    text: &label,
                    x: status.right - label_w - 14.0,
                    y: status.top + 29.0,
                    size: 12.0,
                    bold: true,
                    paint: &paint,
                });
            }
        }
    }

    pub(crate) fn handle_plugin_click(&mut self) {
        let detail_progress = self.anim.get(PLUGIN_DETAIL_KEY);
        if detail_progress > 0.005 && self.handle_plugin_detail_click() {
            return;
        }
        let Some((index, on_toggle)) = self.plugin_hit() else {
            if self.plugin_restart_hit() {
                self.plugin_request = Some(PluginSettingsRequest::Restart);
            }
            return;
        };
        if on_toggle {
            let plugin = &self.plugins[index];
            self.plugin_request = Some(PluginSettingsRequest::SetEnabled {
                id: plugin.id.clone(),
                enabled: !plugin.enabled,
            });
        } else {
            self.selected_plugin_id = Some(self.plugins[index].id.clone());
            self.plugin_detail_closing = false;
            self.plugin_detail_scroll = 0.0;
            self.anim.set_with_speed(PLUGIN_DETAIL_KEY, 1.0, 0.24);
        }
        if let Some(win) = &self.window {
            win.request_redraw();
        }
    }

    pub(crate) fn plugin_hovered(&self) -> bool {
        self.plugin_hit().is_some()
            || self.plugin_restart_hit()
            || (self.anim.get(PLUGIN_DETAIL_KEY) > 0.005 && self.plugin_detail_hovered())
    }

    fn plugin_hit(&self) -> Option<(usize, bool)> {
        let (mx, my) = (
            self.logical_mouse_pos.0 - SIDEBAR_W,
            self.logical_mouse_pos.1 + self.scroll_y,
        );
        let scale = self
            .window
            .as_ref()
            .map(|window| window.scale_factor() as f32)
            .unwrap_or(1.0);
        let width = self.win_w / scale - SIDEBAR_W;
        let content_w = width - CONTENT_PADDING * 2.0;
        let mut y = SETTINGS_HEADER_H + 52.0;
        for (index, _) in self.plugins.iter().enumerate() {
            let card = Rect::from_xywh(CONTENT_PADDING, y, content_w, PLUGIN_CARD_H);
            if card.contains(Point::new(mx, my)) {
                return Some((index, mx >= card.right - 70.0));
            }
            y += PLUGIN_CARD_H + PLUGIN_CARD_GAP;
        }
        None
    }

    fn plugin_restart_hit(&self) -> bool {
        let Some((_, true)) = &self.plugin_status else {
            return false;
        };
        let scale = self
            .window
            .as_ref()
            .map(|window| window.scale_factor() as f32)
            .unwrap_or(1.0);
        let width = self.win_w / scale - SIDEBAR_W;
        let content_w = width - CONTENT_PADDING * 2.0;
        let y = SETTINGS_HEADER_H
            + 18.0
            + 34.0
            + self.plugins.len() as f32 * (PLUGIN_CARD_H + PLUGIN_CARD_GAP)
            + 6.0;
        let (mx, my) = (
            self.logical_mouse_pos.0 - SIDEBAR_W,
            self.logical_mouse_pos.1 + self.scroll_y,
        );
        Rect::from_xywh(CONTENT_PADDING + content_w - 120.0, y, 120.0, 48.0)
            .contains(Point::new(mx, my))
    }
}

fn draw_toggle(canvas: &Canvas, theme: &SettingsTheme, enabled: bool, x: f32, y: f32) {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(if enabled {
        theme.toggle_on
    } else {
        theme.toggle_off
    });
    canvas.draw_round_rect(Rect::from_xywh(x, y, 36.0, 20.0), 10.0, 10.0, &paint);
    paint.set_color(Color::WHITE);
    canvas.draw_circle(
        (x + if enabled { 26.0 } else { 10.0 }, y + 10.0),
        8.0,
        &paint,
    );
}

pub(super) fn draw_plugin_icon(
    direct_context: &mut skia_safe::gpu::DirectContext,
    canvas: &Canvas,
    plugin: &InstalledPlugin,
    rect: Rect,
) {
    if let Some(image) = plugin.icon.as_ref().and_then(|bytes| {
        PLUGIN_ICONS.with(|cache| {
            if let Some(image) = cache.borrow().get(&plugin.id) {
                return Some(image.clone());
            }
            let image = Image::from_encoded(Data::new_copy(bytes))?
                .new_texture_image(direct_context, skia_safe::gpu::Mipmapped::Yes)?;
            cache.borrow_mut().insert(plugin.id.clone(), image.clone());
            Some(image)
        })
    }) {
        let save_count = canvas.save();
        canvas.clip_rrect(
            RRect::new_rect_xy(rect, rect.width() * 0.22, rect.height() * 0.22),
            ClipOp::Intersect,
            true,
        );
        canvas.draw_image_rect(image, None, rect, &Paint::default());
        canvas.restore_to_count(save_count);
        return;
    }
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(Color::from_rgb(175, 82, 222));
    canvas.draw_round_rect(rect, rect.width() * 0.22, rect.height() * 0.22, &paint);
    paint.set_color(Color::WHITE);
    let initial = plugin
        .name
        .chars()
        .next()
        .unwrap_or('P')
        .to_uppercase()
        .to_string();
    let fm = FontManager::global();
    draw_centered_text(
        canvas,
        fm,
        &initial,
        rect.center_x(),
        rect.center_y() + rect.height() * 0.16,
        rect.height() * 0.44,
        true,
        &paint,
    );
}

pub(super) fn ellipsize_text(
    fm: &FontManager,
    text: &str,
    size: f32,
    style: FontStyle,
    max_width: f32,
) -> String {
    if fm.measure_text_cached(text, size, style) <= max_width {
        return text.to_string();
    }
    let ellipsis = "…";
    let ellipsis_width = fm.measure_text_cached(ellipsis, size, style);
    let mut fitted = String::new();
    for character in text.chars() {
        fitted.push(character);
        if fm.measure_text_cached(&fitted, size, style) + ellipsis_width > max_width {
            fitted.pop();
            break;
        }
    }
    fitted.push_str(ellipsis);
    fitted
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_centered_text(
    canvas: &Canvas,
    fm: &FontManager,
    text: &str,
    center_x: f32,
    baseline: f32,
    size: f32,
    bold: bool,
    paint: &Paint,
) {
    let style = if bold {
        FontStyle::bold()
    } else {
        FontStyle::normal()
    };
    let width = fm.measure_text_cached(text, size, style);
    fm.draw_text_cached(DrawTextCachedParams {
        canvas,
        text,
        x: center_x - width / 2.0,
        y: baseline,
        size,
        bold,
        paint,
    });
}
