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
    DETAIL_ICON_SIZE, DETAIL_W, draw_centered_text, draw_plugin_icon, draw_toggle, ellipsize_text,
    markdown,
};

const DETAIL_PADDING: f32 = 20.0;
const DETAIL_HEADER_Y: f32 = 80.0;
const DETAIL_DESCRIPTION_Y: f32 = 170.0;
const GITHUB_BUTTON_W: f32 = 116.0;
const GITHUB_BUTTON_H: f32 = 28.0;

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
        let Some(plugin) = self.selected_plugin() else {
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

        let content_y = mouse_y + self.plugin_detail_scroll;
        if toggle_rect(panel_x).contains(Point::new(mouse_x, content_y)) {
            self.plugin_request = Some(PluginSettingsRequest::SetEnabled {
                id: plugin.id.clone(),
                enabled: !plugin.enabled,
            });
            return true;
        }
        if safe_github_url(&plugin.github_link)
            && github_rect(panel_x, plugin).contains(Point::new(mouse_x, content_y))
        {
            open_url(&plugin.github_link);
            return true;
        }

        let fallback = tr("plugin_readme_empty");
        let readme = plugin.readme.as_deref().unwrap_or(&fallback);
        if let Some(link) = markdown::links(
            readme,
            panel_x + 20.0,
            plugin_readme_y(plugin),
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
        let Some(plugin) = self.selected_plugin() else {
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
        let y = DETAIL_HEADER_Y;
        draw_plugin_icon(
            direct_context,
            canvas,
            plugin,
            Rect::from_xywh(
                panel_x + DETAIL_PADDING,
                y,
                DETAIL_ICON_SIZE,
                DETAIL_ICON_SIZE,
            ),
        );
        let info_x = panel_x + DETAIL_PADDING + DETAIL_ICON_SIZE + 14.0;
        let toggle_x = panel_x + DETAIL_W - DETAIL_PADDING - 36.0;
        let name = ellipsize_text(
            fm,
            &plugin.name,
            17.0,
            skia_safe::FontStyle::bold(),
            (toggle_x - info_x - 10.0).max(30.0),
        );
        paint.set_color(theme.text_pri);
        fm.draw_text_cached(DrawTextCachedParams {
            canvas,
            text: &name,
            x: info_x,
            y: y + 22.0,
            size: 17.0,
            bold: true,
            paint: &paint,
        });
        paint.set_color(theme.text_sec);
        let subtitle = format!("{} · v{}", plugin.author, plugin.version);
        let subtitle = ellipsize_text(
            fm,
            &subtitle,
            11.5,
            skia_safe::FontStyle::normal(),
            DETAIL_W - (info_x - panel_x) - DETAIL_PADDING,
        );
        fm.draw_text_cached(DrawTextCachedParams {
            canvas,
            text: &subtitle,
            x: info_x,
            y: y + 43.0,
            size: 11.5,
            bold: false,
            paint: &paint,
        });
        draw_toggle(canvas, theme, plugin.enabled, toggle_x, y + 2.0);

        if safe_github_url(&plugin.github_link) {
            let button = github_rect(panel_x, plugin);
            paint.set_color(theme.control_bg);
            canvas.draw_round_rect(button, GITHUB_BUTTON_H / 2.0, GITHUB_BUTTON_H / 2.0, &paint);
            paint.set_color(theme.accent);
            let label = ellipsize_text(
                fm,
                &tr("plugin_open_github"),
                11.0,
                skia_safe::FontStyle::bold(),
                GITHUB_BUTTON_W - 20.0,
            );
            draw_centered_text(
                canvas,
                fm,
                &label,
                button.center_x(),
                button.top + 18.0,
                11.0,
                true,
                &paint,
            );
        }

        let mut y = DETAIL_DESCRIPTION_Y;
        let description = markdown::render(markdown::MarkdownRenderParams {
            canvas,
            markdown: &plugin.description,
            origin: (panel_x + DETAIL_PADDING, y),
            width: DETAIL_W - DETAIL_PADDING * 2.0,
            visible_range: (
                self.plugin_detail_scroll + SETTINGS_HEADER_H,
                self.plugin_detail_scroll + win_h,
            ),
            colors: markdown_colors(theme),
        });
        y += description.height + 14.0;
        let fallback = tr("plugin_readme_empty");
        let readme = plugin.readme.as_deref().unwrap_or(&fallback);
        let readme = markdown::render(markdown::MarkdownRenderParams {
            canvas,
            markdown: readme,
            origin: (panel_x + DETAIL_PADDING, y),
            width: DETAIL_W - DETAIL_PADDING * 2.0,
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
        text: &tr("plugin_details"),
        x: panel_x + DETAIL_PADDING,
        y: 37.0,
        size: 15.0,
        bold: true,
        paint: &paint,
    });
}

fn toggle_rect(panel_x: f32) -> Rect {
    Rect::from_xywh(
        panel_x + DETAIL_W - DETAIL_PADDING - 42.0,
        DETAIL_HEADER_Y - 4.0,
        48.0,
        32.0,
    )
}

fn github_rect(panel_x: f32, _plugin: &InstalledPlugin) -> Rect {
    Rect::from_xywh(
        panel_x + DETAIL_PADDING + DETAIL_ICON_SIZE + 14.0,
        DETAIL_HEADER_Y + 50.0,
        GITHUB_BUTTON_W,
        GITHUB_BUTTON_H,
    )
}

fn plugin_readme_y(plugin: &InstalledPlugin) -> f32 {
    DETAIL_DESCRIPTION_Y
        + markdown::markdown_height(&plugin.description, DETAIL_W - DETAIL_PADDING * 2.0)
        + 14.0
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
