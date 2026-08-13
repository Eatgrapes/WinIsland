use skia_safe::{Canvas, Color, Contains, Paint, Point, Rect};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::PCWSTR;

use crate::core::i18n::tr;
use crate::plugin::manager::InstalledPlugin;
use crate::utils::color::SettingsTheme;
use crate::utils::font::{DrawTextCachedParams, FontManager};

use super::super::super::{
    PLUGIN_DETAIL_KEY, PluginSettingsRequest, SETTINGS_HEADER_H, SIDEBAR_W, SettingsApp,
};
use super::{
    DETAIL_ICON_SIZE, DETAIL_W, draw_centered_text, draw_plugin_icon, draw_toggle, markdown,
};

impl SettingsApp {
    pub(crate) fn plugin_detail_contains(&self, mouse_x: f32) -> bool {
        let scale = self
            .window
            .as_ref()
            .map(|window| window.scale_factor() as f32)
            .unwrap_or(1.0);
        mouse_x >= self.win_w / scale - DETAIL_W * self.anim.get(PLUGIN_DETAIL_KEY)
    }

    pub(super) fn close_plugin_detail(&mut self) {
        self.plugin_detail_closing = true;
        self.anim.set_with_speed(PLUGIN_DETAIL_KEY, 0.0, 0.28);
    }

    fn selected_plugin(&self) -> Option<&InstalledPlugin> {
        let id = self.selected_plugin_id.as_ref()?;
        self.plugins.iter().find(|plugin| &plugin.id == id)
    }

    pub(super) fn handle_plugin_detail_click(&mut self) -> bool {
        let Some(plugin) = self.selected_plugin().cloned() else {
            return false;
        };
        let scale = self
            .window
            .as_ref()
            .map(|window| window.scale_factor() as f32)
            .unwrap_or(1.0);
        let win_w = self.win_w / scale;
        let panel_x = win_w - DETAIL_W * self.anim.get(PLUGIN_DETAIL_KEY);
        let (mouse_x, mouse_y) = self.logical_mouse_pos;
        if mouse_x < panel_x {
            self.close_plugin_detail();
            return true;
        }
        if close_rect(panel_x).contains(Point::new(mouse_x, mouse_y)) {
            self.close_plugin_detail();
            return true;
        }

        let content_y = mouse_y + self.plugin_detail_scroll;
        if toggle_rect(panel_x).contains(Point::new(mouse_x, content_y)) {
            self.plugin_request = Some(PluginSettingsRequest::SetEnabled {
                id: plugin.id,
                enabled: !plugin.enabled,
            });
            return true;
        }
        if safe_github_url(&plugin.github_link)
            && github_rect(panel_x, &plugin).contains(Point::new(mouse_x, content_y))
        {
            open_url(&plugin.github_link);
            return true;
        }

        let fallback = tr("plugin_readme_empty");
        let readme = plugin.readme.as_deref().unwrap_or(&fallback);
        if let Some(link) = markdown::links(
            readme,
            panel_x + 20.0,
            plugin_readme_y(&plugin),
            DETAIL_W - 40.0,
        )
        .iter()
        .find(|link| link.rect.contains(Point::new(mouse_x, content_y)))
        {
            open_url(&link.url);
        }
        true
    }

    pub(super) fn plugin_detail_hovered(&self) -> bool {
        let Some(plugin) = self.selected_plugin() else {
            return false;
        };
        let scale = self
            .window
            .as_ref()
            .map(|window| window.scale_factor() as f32)
            .unwrap_or(1.0);
        let panel_x = self.win_w / scale - DETAIL_W * self.anim.get(PLUGIN_DETAIL_KEY);
        let (mouse_x, mouse_y) = self.logical_mouse_pos;
        if close_rect(panel_x).contains(Point::new(mouse_x, mouse_y)) {
            return true;
        }
        let content_y = mouse_y + self.plugin_detail_scroll;
        if toggle_rect(panel_x).contains(Point::new(mouse_x, content_y))
            || (safe_github_url(&plugin.github_link)
                && github_rect(panel_x, plugin).contains(Point::new(mouse_x, content_y)))
        {
            return true;
        }
        let fallback = tr("plugin_readme_empty");
        let readme = plugin.readme.as_deref().unwrap_or(&fallback);
        markdown::links(
            readme,
            panel_x + 20.0,
            plugin_readme_y(plugin),
            DETAIL_W - 40.0,
        )
        .iter()
        .any(|link| link.rect.contains(Point::new(mouse_x, content_y)))
    }

    pub(super) fn draw_plugin_detail(
        &mut self,
        direct_context: &mut skia_safe::gpu::DirectContext,
        canvas: &Canvas,
        theme: &SettingsTheme,
        win_w: f32,
        win_h: f32,
        progress: f32,
    ) {
        let Some(plugin) = self.selected_plugin().cloned() else {
            return;
        };
        let panel_x = win_w - DETAIL_W * progress;
        draw_panel_background(canvas, theme, panel_x, win_h, progress);
        draw_panel_header(canvas, theme, panel_x);

        canvas.save();
        canvas.clip_rect(
            Rect::from_xywh(
                panel_x,
                SETTINGS_HEADER_H,
                DETAIL_W,
                win_h - SETTINGS_HEADER_H,
            ),
            skia_safe::ClipOp::Intersect,
            true,
        );
        canvas.translate((0.0, -self.plugin_detail_scroll));

        let fm = FontManager::global();
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        let mut y = 82.0;
        draw_plugin_icon(
            direct_context,
            canvas,
            &plugin,
            Rect::from_xywh(panel_x + 20.0, y, DETAIL_ICON_SIZE, DETAIL_ICON_SIZE),
        );
        paint.set_color(theme.text_pri);
        fm.draw_text_cached(DrawTextCachedParams {
            canvas,
            text: &plugin.name,
            x: panel_x + 108.0,
            y: y + 25.0,
            size: 18.0,
            bold: true,
            paint: &paint,
        });
        paint.set_color(theme.text_sec);
        fm.draw_text_cached(DrawTextCachedParams {
            canvas,
            text: &format!("{} · v{}", plugin.author, plugin.version),
            x: panel_x + 108.0,
            y: y + 49.0,
            size: 12.0,
            bold: false,
            paint: &paint,
        });
        draw_toggle(
            canvas,
            theme,
            plugin.enabled,
            panel_x + DETAIL_W - 56.0,
            y + 28.0,
        );

        y += 94.0;
        let description = markdown::render(markdown::MarkdownRenderParams {
            canvas,
            markdown: &plugin.description,
            origin: (panel_x + 20.0, y),
            width: DETAIL_W - 40.0,
            visible_range: (
                self.plugin_detail_scroll + SETTINGS_HEADER_H,
                self.plugin_detail_scroll + win_h,
            ),
            colors: markdown_colors(theme),
        });
        y += description.height + 18.0;

        if safe_github_url(&plugin.github_link) {
            paint.set_color(theme.control_bg);
            canvas.draw_round_rect(
                Rect::from_xywh(panel_x + 20.0, y, DETAIL_W - 40.0, 38.0),
                9.0,
                9.0,
                &paint,
            );
            paint.set_color(theme.accent);
            draw_centered_text(
                canvas,
                fm,
                &tr("plugin_open_github"),
                panel_x + DETAIL_W / 2.0,
                y + 24.0,
                12.0,
                true,
                &paint,
            );
            y += 56.0;
        }

        paint.set_color(theme.text_pri);
        fm.draw_text_cached(DrawTextCachedParams {
            canvas,
            text: &tr("plugin_readme"),
            x: panel_x + 20.0,
            y: y + 20.0,
            size: 14.0,
            bold: true,
            paint: &paint,
        });
        y += 34.0;
        let fallback = tr("plugin_readme_empty");
        let readme = plugin.readme.as_deref().unwrap_or(&fallback);
        let readme = markdown::render(markdown::MarkdownRenderParams {
            canvas,
            markdown: readme,
            origin: (panel_x + 20.0, y),
            width: DETAIL_W - 40.0,
            visible_range: (
                self.plugin_detail_scroll + SETTINGS_HEADER_H,
                self.plugin_detail_scroll + win_h,
            ),
            colors: markdown_colors(theme),
        });
        y += readme.height + 28.0;
        canvas.restore();

        self.plugin_detail_max_scroll = (y - win_h).max(0.0);
        self.plugin_detail_scroll = self
            .plugin_detail_scroll
            .clamp(0.0, self.plugin_detail_max_scroll);
    }
}

fn draw_panel_background(
    canvas: &Canvas,
    theme: &SettingsTheme,
    panel_x: f32,
    win_h: f32,
    progress: f32,
) {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(Color::from_argb((72.0 * progress) as u8, 0, 0, 0));
    canvas.draw_rect(
        Rect::from_xywh(SIDEBAR_W, 0.0, panel_x - SIDEBAR_W, win_h),
        &paint,
    );
    paint.set_color(theme.win_bg);
    canvas.draw_rect(Rect::from_xywh(panel_x, 0.0, DETAIL_W, win_h), &paint);
    paint.set_color(theme.separator);
    canvas.draw_rect(Rect::from_xywh(panel_x, 0.0, 0.5, win_h), &paint);
}

fn draw_panel_header(canvas: &Canvas, theme: &SettingsTheme, panel_x: f32) {
    let fm = FontManager::global();
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(theme.text_pri);
    fm.draw_text_cached(DrawTextCachedParams {
        canvas,
        text: "‹",
        x: panel_x + 20.0,
        y: 39.0,
        size: 28.0,
        bold: false,
        paint: &paint,
    });
    fm.draw_text_cached(DrawTextCachedParams {
        canvas,
        text: &tr("plugin_details"),
        x: panel_x + 54.0,
        y: 37.0,
        size: 15.0,
        bold: true,
        paint: &paint,
    });
}

fn close_rect(panel_x: f32) -> Rect {
    Rect::from_xywh(panel_x + 14.0, 16.0, 32.0, 32.0)
}

fn toggle_rect(panel_x: f32) -> Rect {
    Rect::from_xywh(panel_x + DETAIL_W - 62.0, 82.0, 44.0, 72.0)
}

fn github_rect(panel_x: f32, plugin: &InstalledPlugin) -> Rect {
    Rect::from_xywh(
        panel_x + 20.0,
        plugin_github_y(plugin),
        DETAIL_W - 40.0,
        38.0,
    )
}

fn plugin_github_y(plugin: &InstalledPlugin) -> f32 {
    82.0 + 94.0 + markdown::markdown_height(&plugin.description, DETAIL_W - 40.0) + 18.0
}

fn plugin_readme_y(plugin: &InstalledPlugin) -> f32 {
    let github_height = if safe_github_url(&plugin.github_link) {
        56.0
    } else {
        0.0
    };
    plugin_github_y(plugin) + github_height + 34.0
}

fn safe_github_url(url: &str) -> bool {
    url.starts_with("https://github.com/") || url.starts_with("http://github.com/")
}

fn markdown_colors(theme: &SettingsTheme) -> markdown::MarkdownColors {
    markdown::MarkdownColors {
        text: theme.text_pri,
        secondary: theme.text_sec,
        accent: theme.accent,
        code_background: theme.control_bg,
        quote_background: theme.group_bg,
        separator: theme.separator,
    }
}

fn open_url(url: &str) {
    if !markdown::safe_web_url(url) {
        return;
    }
    let wide = url
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // SAFETY: `wide` is a null-terminated UTF-16 string valid for the duration of the call.
    unsafe {
        let _ = ShellExecuteW(None, None, PCWSTR(wide.as_ptr()), None, None, SW_SHOWNORMAL);
    }
}
