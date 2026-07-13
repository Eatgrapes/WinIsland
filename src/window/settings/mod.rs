use crate::core::config::{AppConfig, WidgetKind};
use crate::core::i18n::tr;
use crate::utils::anim::AnimPool;
use crate::utils::color::*;
use crate::utils::font::FontManager;
use crate::utils::icon::get_app_icon;
use crate::utils::settings_ui::items::*;
use crate::utils::settings_ui::*;
use skia_safe::Rect;
use softbuffer::{Context, Surface};
use std::sync::Arc;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{DWMWINDOWATTRIBUTE, DwmSetWindowAttribute};
use windows::Win32::System::Threading::{MUTEX_ALL_ACCESS, OpenMutexW};
use windows::core::w;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{Window, WindowId};

pub mod input;
pub mod items;
pub mod pages;
pub mod renderer;
pub mod sidebar;

pub(crate) const WIN_W: f32 = 666.0;
pub(crate) const WIN_H: f32 = 666.0;
pub(crate) const SIDEBAR_W: f32 = 180.0;
pub(crate) const SIDEBAR_ROW_H: f32 = 32.0;
pub(crate) const CONTENT_START_Y: f32 = 10.0;
pub(crate) const SUB_TAB_H: f32 = 40.0;
pub(crate) const SUB_TAB_START_Y: f32 = 50.0;

pub(crate) const POPUP_OPACITY_KEY: u64 = 1;
pub(crate) const SIDEBAR_KEY_BASE: u64 = 1_000;
pub(crate) const SCROLL_STIFFNESS: f32 = 55.0;
pub(crate) const SCROLL_DAMPING: f32 = 16.0;

pub(crate) fn widget_drag_move_needs_redraw(
    dragging: bool,
    current_slot: Option<usize>,
    new_slot: Option<usize>,
) -> bool {
    dragging || current_slot != new_slot
}

pub(crate) fn settings_frame_should_continue(
    has_anim: bool,
    has_popup: bool,
    is_scrolling: bool,
    is_widget_dragging: bool,
) -> bool {
    has_anim || has_popup || is_scrolling || is_widget_dragging
}

#[derive(Clone, PartialEq)]
pub(crate) enum PopupKind {
    LyricsSource,
    Language,
    Monitor,
    IslandStyle,
    DockPositionPopup,
    SettingsTheme,
    UpdateChannel,
}

pub(crate) struct PopupState {
    pub(crate) kind: PopupKind,
    #[allow(dead_code)]
    pub(crate) button_rect: Rect,
    pub(crate) menu_rect: Rect,
    pub(crate) options: Vec<String>,
    pub(crate) values: Vec<String>,
    pub(crate) selected_idx: usize,
    pub(crate) hover_idx: Option<usize>,
}

impl PopupState {
    pub(crate) fn new(
        kind: PopupKind,
        button_rect: Rect,
        options: Vec<String>,
        values: Vec<String>,
        selected_idx: usize,
        win_w: f32,
        win_h: f32,
    ) -> Self {
        let mut max_w: f32 = 120.0;
        let fm = FontManager::global();
        for opt in &options {
            let w = fm.measure_text_cached(opt, 12.0, skia_safe::FontStyle::normal());
            if w > max_w {
                max_w = w;
            }
        }
        let menu_w = max_w + 36.0;
        let menu_h = options.len() as f32 * POPUP_ITEM_H + POPUP_MENU_PAD * 2.0;
        let menu_x = (button_rect.right - menu_w).clamp(0.0, win_w - menu_w - 10.0);
        let menu_y = (button_rect.bottom + 4.0).clamp(0.0, win_h - menu_h - 10.0);
        let menu_rect = Rect::from_xywh(menu_x, menu_y, menu_w, menu_h);

        Self {
            kind,
            button_rect,
            menu_rect,
            options,
            values,
            selected_idx,
            hover_idx: None,
        }
    }

    pub(crate) fn menu_rect(&self) -> Rect {
        self.menu_rect
    }

    pub(crate) fn item_rect(&self, idx: usize) -> Rect {
        let inner_top = self.menu_rect.top + POPUP_MENU_PAD;
        let y = inner_top + idx as f32 * POPUP_ITEM_H;
        Rect::from_xywh(
            self.menu_rect.left + POPUP_MENU_PAD,
            y,
            self.menu_rect.width() - POPUP_MENU_PAD * 2.0,
            POPUP_ITEM_H,
        )
    }

    pub(crate) fn hit_test_item(&self, mx: f32, my: f32) -> Option<usize> {
        let menu = self.menu_rect;
        if mx < menu.left || mx > menu.right || my < menu.top || my > menu.bottom {
            return None;
        }
        let inner_top = menu.top + POPUP_MENU_PAD;
        let inner_bottom = menu.bottom - POPUP_MENU_PAD;
        if my < inner_top || my > inner_bottom {
            return None;
        }
        let rel_y = my - inner_top;
        let idx = (rel_y / POPUP_ITEM_H).floor() as i32;
        if idx < 0 {
            return None;
        }
        let idx = idx as usize;
        if idx >= self.options.len() {
            return None;
        }
        Some(idx)
    }
}

pub struct SettingsApp {
    pub(crate) window: Option<Arc<Window>>,
    pub(crate) surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    pub(crate) config: AppConfig,
    pub(crate) active_page: usize,
    pub(crate) active_sub_page: usize,
    pub(crate) sub_tab_hover: i32,
    pub(crate) switch_anim: SwitchAnimator,
    pub(crate) anim: AnimPool,
    pub(crate) logical_mouse_pos: (f32, f32),
    pub(crate) last_hover_mouse_pos: (f32, f32),
    pub(crate) frame_count: u64,
    pub(crate) scroll_y: f32,
    pub(crate) target_scroll_y: f32,
    pub(crate) scroll_vel_y: f32,
    pub(crate) last_frame_time: Instant,
    pub(crate) detected_apps: Vec<String>,
    pub(crate) sidebar_hover: i32,
    pub(crate) popup: Option<PopupState>,
    pub(crate) hover_row: Option<usize>,
    pub(crate) total_rows: usize,
    pub(crate) is_light: bool,
    pub(crate) cached_items: Vec<SettingsItem>,
    pub(crate) items_dirty: bool,
    pub(crate) cached_content_height: f32,
    pub(crate) cached_max_scroll: f32,
    pub(crate) cached_row_tops: Vec<f32>,
    pub(crate) cached_row_heights: Vec<f32>,
    pub(crate) win_w: f32,
    pub(crate) win_h: f32,
    pub(crate) focused: bool,
    pub(crate) dots_hovered: bool,
    pub(crate) widget_dragging: Option<WidgetKind>,
    pub(crate) widget_drag_hover_slot: Option<usize>,
    pub(crate) widget_preview_hover_slot: Option<usize>,
}

impl SettingsApp {
    pub fn new(config: AppConfig) -> Self {
        let switch_anim = SwitchAnimator::new(&[
            config.adaptive_border,
            config.motion_blur,
            config.cover_rotate,
            config.auto_start,
            config.auto_hide,
            config.right_click_drag,
            config.check_for_updates,
            config.smtc_enabled,
            config.show_lyrics,
            config.lyrics_fallback,
            config.lyrics_scroll,
        ]);
        Self {
            window: None,
            surface: None,
            config,
            active_page: 0,
            active_sub_page: 0,
            sub_tab_hover: -1,
            switch_anim,
            anim: AnimPool::new(),
            logical_mouse_pos: (0.0, 0.0),
            last_hover_mouse_pos: (-1.0, -1.0),
            frame_count: 0,
            scroll_y: 0.0,
            target_scroll_y: 0.0,
            scroll_vel_y: 0.0,
            last_frame_time: Instant::now(),
            detected_apps: Vec::new(),
            sidebar_hover: -1,
            popup: None,
            hover_row: None,
            total_rows: 0,
            is_light: false,
            cached_items: Vec::new(),
            items_dirty: true,
            cached_content_height: 0.0,
            cached_max_scroll: 0.0,
            cached_row_tops: Vec::new(),
            cached_row_heights: Vec::new(),
            win_w: WIN_W,
            win_h: WIN_H,
            focused: true,
            dots_hovered: false,
            widget_dragging: None,
            widget_drag_hover_slot: None,
            widget_preview_hover_slot: None,
        }
    }

    pub(crate) fn theme(&self) -> SettingsTheme {
        if self.is_light {
            light_settings_theme()
        } else {
            dark_settings_theme()
        }
    }

    pub(crate) fn update_theme(&mut self) {
        self.is_light = match self.config.settings_theme.as_str() {
            "light" => true,
            "dark" => false,
            _ => {
                if let Some(win) = &self.window {
                    win.theme() == Some(winit::window::Theme::Light)
                } else {
                    false
                }
            }
        };
        if let Some(win) = &self.window {
            Self::apply_titlebar_theme(win, self.is_light);
            win.request_redraw();
        }
    }

    pub(crate) fn apply_titlebar_theme(window: &Window, is_light: bool) {
        if let Ok(handle) = window.window_handle()
            && let RawWindowHandle::Win32(raw) = handle.as_raw()
        {
            let hwnd = HWND(raw.hwnd.get() as _);
            let use_dark: i32 = if is_light { 0 } else { 1 };
            unsafe {
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWINDOWATTRIBUTE(20),
                    &use_dark as *const _ as *const _,
                    std::mem::size_of::<i32>() as u32,
                );
            }
        }
    }

    pub(crate) fn get_monitor_list(&self) -> Vec<String> {
        use windows::Win32::Graphics::Gdi::*;
        let mut monitors: Vec<String> = Vec::new();
        unsafe {
            let mut idx = 0u32;
            let mut active_count = 0;
            loop {
                let mut dd: DISPLAY_DEVICEW = std::mem::zeroed();
                dd.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
                if EnumDisplayDevicesW(None, idx, &mut dd, 0).as_bool() {
                    if (dd.StateFlags & DISPLAY_DEVICE_ACTIVE) != DISPLAY_DEVICE_STATE_FLAGS(0) {
                        active_count += 1;
                        let name = String::from_utf16_lossy(&dd.DeviceName)
                            .trim_end_matches('\0')
                            .to_string();
                        let mut dm: DISPLAY_DEVICEW = std::mem::zeroed();
                        dm.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
                        let mut label = if EnumDisplayDevicesW(
                            windows::core::PCWSTR(dd.DeviceName.as_ptr()),
                            0,
                            &mut dm,
                            0,
                        )
                        .as_bool()
                        {
                            let friendly = String::from_utf16_lossy(&dm.DeviceString)
                                .trim_end_matches('\0')
                                .to_string();
                            if friendly.is_empty() {
                                name.clone()
                            } else {
                                friendly
                            }
                        } else {
                            name.clone()
                        };
                        label = format!("Display {}: {}", active_count, label);
                        monitors.push(label);
                    }
                    idx += 1;
                } else {
                    break;
                }
            }
        }
        if monitors.is_empty() {
            monitors.push("Primary".to_string());
        }
        monitors
    }

    pub(crate) fn sync_switch_targets(&mut self) {
        self.switch_anim.set_target(0, self.config.adaptive_border);
        self.switch_anim.set_target(1, self.config.motion_blur);
        self.switch_anim.set_target(2, self.config.cover_rotate);
        self.switch_anim.set_target(3, self.config.auto_start);
        self.switch_anim.set_target(4, self.config.auto_hide);
        self.switch_anim.set_target(5, self.config.right_click_drag);
        self.switch_anim
            .set_target(6, self.config.check_for_updates);
        self.switch_anim.set_target(7, self.config.smtc_enabled);
        self.switch_anim.set_target(8, self.config.show_lyrics);
        let fb_on = if self.config.show_lyrics {
            self.config.lyrics_fallback
        } else {
            false
        };
        self.switch_anim.set_target(9, fb_on);
        let fw_on = if self.config.show_lyrics {
            self.config.lyrics_scroll
        } else {
            false
        };
        self.switch_anim.set_target(10, fw_on);
    }

    pub(crate) fn update_detected_apps(&mut self) {
        use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager;
        let mut changed = false;
        if let Ok(manager_async) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            && let Ok(manager) = manager_async.join()
            && let Ok(sessions) = manager.GetSessions()
            && let Ok(size) = sessions.Size()
        {
            for i in 0..size {
                if let Ok(session) = sessions.GetAt(i)
                    && let Ok(id) = session.SourceAppUserModelId()
                {
                    let name = id.to_string();
                    if !self.detected_apps.contains(&name) {
                        self.detected_apps.push(name);
                        changed = true;
                    }
                }
            }
        }
        for app in &self.config.smtc_known_apps {
            if !self.detected_apps.contains(app) {
                self.detected_apps.push(app.clone());
                changed = true;
            }
        }
        if changed {
            self.items_dirty = true;
        }
    }
}

impl ApplicationHandler for SettingsApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title("WinIsland Settings")
            .with_inner_size(LogicalSize::new(WIN_W as f64, WIN_H as f64))
            .with_resizable(true)
            .with_decorations(false)
            .with_transparent(true)
            .with_window_icon(get_app_icon());
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        self.window = Some(window.clone());
        let context = Context::new(window.clone()).unwrap();
        let mut surface = Surface::new(&context, window.clone()).unwrap();
        let size = window.inner_size();
        self.win_w = size.width as f32;
        self.win_h = size.height as f32;
        resize_surface(&mut surface, size.width, size.height);
        self.surface = Some(surface);
        self.update_theme();
        self.update_detected_apps();
    }

    fn window_event(&mut self, _el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => _el.exit(),
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                if let Some(win) = &self.window {
                    win.request_redraw();
                }
            }
            WindowEvent::ThemeChanged(theme) if self.config.settings_theme == "system" => {
                self.is_light = theme == winit::window::Theme::Light;
                if let Some(win) = &self.window {
                    Self::apply_titlebar_theme(win, self.is_light);
                    win.request_redraw();
                }
            }
            WindowEvent::Resized(new_size) => {
                self.win_w = new_size.width as f32;
                self.win_h = new_size.height as f32;
                if let Some(surface) = &mut self.surface {
                    resize_surface(surface, new_size.width, new_size.height);
                    if let Some(win) = &self.window {
                        win.request_redraw();
                    }
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let (Some(win), Some(surface)) = (&self.window, &mut self.surface) {
                    let size = win.inner_size();
                    resize_surface(surface, size.width, size.height);
                    win.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.logical_key {
                    Key::Named(NamedKey::F11) => {}
                    Key::Named(NamedKey::ArrowLeft) => {
                        if self.active_page == 0 {
                            if self.active_sub_page > 0 {
                                self.active_sub_page -= 1;
                                self.scroll_y = 0.0;
                                self.target_scroll_y = 0.0;
                                self.scroll_vel_y = 0.0;
                                self.mark_items_dirty();
                                if let Some(win) = &self.window {
                                    win.request_redraw();
                                }
                            }
                        } else if self.active_page > 0 {
                            self.active_page -= 1;
                            self.scroll_y = 0.0;
                            self.target_scroll_y = 0.0;
                            self.scroll_vel_y = 0.0;
                            self.mark_items_dirty();
                            if let Some(win) = &self.window {
                                win.request_redraw();
                            }
                        }
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        if self.active_page == 0 {
                            if self.active_sub_page < 2 {
                                self.active_sub_page += 1;
                                self.scroll_y = 0.0;
                                self.target_scroll_y = 0.0;
                                self.scroll_vel_y = 0.0;
                                self.mark_items_dirty();
                                if let Some(win) = &self.window {
                                    win.request_redraw();
                                }
                            }
                        } else if self.active_page < 3 {
                            self.active_page += 1;
                            self.scroll_y = 0.0;
                            self.target_scroll_y = 0.0;
                            self.scroll_vel_y = 0.0;
                            self.mark_items_dirty();
                            if let Some(win) = &self.window {
                                win.request_redraw();
                            }
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self
                    .window
                    .as_ref()
                    .map(|w| w.scale_factor() as f32)
                    .unwrap_or(1.0);
                let new_pos = (position.x as f32 / scale, position.y as f32 / scale);
                let mouse_moved = (new_pos.0 - self.last_hover_mouse_pos.0).abs() > 0.5
                    || (new_pos.1 - self.last_hover_mouse_pos.1).abs() > 0.5;
                self.logical_mouse_pos = new_pos;

                if self.widget_dragging.is_some() {
                    let new_slot = self
                        .widget_preview_hit_at_mouse()
                        .and_then(|hit| match hit {
                            WidgetPreviewHit::Slot(slot) => Some(slot),
                            _ => None,
                        });
                    let needs_redraw =
                        widget_drag_move_needs_redraw(true, self.widget_drag_hover_slot, new_slot);
                    if new_slot != self.widget_drag_hover_slot {
                        self.widget_drag_hover_slot = new_slot;
                    }
                    if needs_redraw && let Some(win) = &self.window {
                        win.request_redraw();
                    }
                } else if self.active_page == 2 {
                    let new_slot = self
                        .widget_preview_hit_at_mouse()
                        .and_then(|hit| match hit {
                            WidgetPreviewHit::Slot(slot) => Some(slot),
                            _ => None,
                        });
                    if new_slot != self.widget_preview_hover_slot {
                        self.widget_preview_hover_slot = new_slot;
                        if let Some(win) = &self.window {
                            win.request_redraw();
                        }
                    }
                }

                if let Some(popup) = &mut self.popup {
                    let (mx, my) = self.logical_mouse_pos;
                    let new_hover = popup.hit_test_item(mx, my);
                    if new_hover != popup.hover_idx {
                        popup.hover_idx = new_hover;
                        if let Some(win) = &self.window {
                            win.request_redraw();
                        }
                    }
                }

                if mouse_moved {
                    self.last_hover_mouse_pos = new_pos;
                    let (mx, my) = self.logical_mouse_pos;
                    let mut new_hover: i32 = -1;
                    if mx < SIDEBAR_W {
                        let start_y = 60.0;
                        for i in 0..4 {
                            let row_y = start_y + i as f32 * (SIDEBAR_ROW_H + 2.0);
                            if my >= row_y
                                && my <= row_y + SIDEBAR_ROW_H
                                && (SIDEBAR_PAD..=SIDEBAR_W - SIDEBAR_PAD).contains(&mx)
                            {
                                new_hover = i;
                            }
                        }
                    }
                    if new_hover != self.sidebar_hover {
                        self.sidebar_hover = new_hover;
                        for idx in 0..4 {
                            if idx == new_hover as usize {
                                self.anim.set(SIDEBAR_KEY_BASE + idx as u64, 1.0);
                            } else {
                                self.anim.set(SIDEBAR_KEY_BASE + idx as u64, 0.0);
                            }
                        }
                        if let Some(win) = &self.window {
                            win.request_redraw();
                        }
                    }

                    let scale = self
                        .window
                        .as_ref()
                        .map(|w| w.scale_factor() as f32)
                        .unwrap_or(1.0);
                    let content_w = self.win_w / scale - SIDEBAR_W;

                    if self.active_page == 0
                        && mx >= SIDEBAR_W
                        && (SUB_TAB_START_Y..=SUB_TAB_START_Y + SUB_TAB_H).contains(&my)
                    {
                        let tabs = [
                            tr("section_appearance"),
                            tr("section_effects"),
                            tr("section_behavior"),
                        ];
                        let tab_count = tabs.len() as i32;
                        let tab_w = content_w / tab_count as f32;
                        let rel_x = mx - SIDEBAR_W;
                        let tab_idx = (rel_x / tab_w) as i32;
                        let new_sub_hover = if tab_idx >= 0 && tab_idx < tab_count {
                            tab_idx
                        } else {
                            -1
                        };
                        if new_sub_hover != self.sub_tab_hover {
                            self.sub_tab_hover = new_sub_hover;
                            if let Some(win) = &self.window {
                                win.request_redraw();
                            }
                        }
                    } else if self.sub_tab_hover != -1 {
                        self.sub_tab_hover = -1;
                        if let Some(win) = &self.window {
                            win.request_redraw();
                        }
                    }

                    if mx >= SIDEBAR_W {
                        let content_x = mx - SIDEBAR_W;
                        let content_y = my + self.scroll_y;
                        let mut new_row: Option<usize> = None;
                        self.ensure_items_cache();
                        if content_x >= CONTENT_PADDING && content_x <= content_w - CONTENT_PADDING
                        {
                            let idx = match self
                                .cached_row_tops
                                .binary_search_by(|y| y.total_cmp(&content_y))
                            {
                                Ok(i) => Some(i),
                                Err(0) => None,
                                Err(i) => Some(i - 1),
                            };
                            if let Some(i) = idx
                                && content_y <= self.cached_row_tops[i] + self.cached_row_heights[i]
                            {
                                new_row = Some(i);
                            }
                        }
                        if new_row != self.hover_row {
                            if let Some(old) = self.hover_row {
                                self.anim.set(HOVER_ROW_KEY_BASE + old as u64, 0.0);
                            }
                            if let Some(new) = new_row {
                                self.anim.set(HOVER_ROW_KEY_BASE + new as u64, 1.0);
                            }
                            self.hover_row = new_row;
                        }
                    } else if self.hover_row.is_some() {
                        if let Some(old) = self.hover_row {
                            self.anim.set(HOVER_ROW_KEY_BASE + old as u64, 0.0);
                        }
                        self.hover_row = None;
                    }
                }

                let cursor = if self.get_hover_state() {
                    winit::window::CursorIcon::Pointer
                } else {
                    winit::window::CursorIcon::Default
                };
                if let Some(win) = &self.window {
                    win.set_cursor(cursor);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if self.popup.is_some() {
                    self.popup = None;
                    self.anim.set_with_speed(POPUP_OPACITY_KEY, 0.0, 0.3);
                    if let Some(win) = &self.window {
                        win.request_redraw();
                    }
                    return;
                }
                let (mx, _) = self.logical_mouse_pos;
                if mx >= SIDEBAR_W {
                    let diff = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => y * 40.0,
                        winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                    };
                    self.target_scroll_y =
                        (self.target_scroll_y - diff).clamp(0.0, self.cached_max_scroll);
                    if let Some(win) = &self.window {
                        win.request_redraw();
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                let (mx, my) = self.logical_mouse_pos;
                let is_on_red = (mx - 20.0).powi(2) + (my - 24.0).powi(2) <= 36.0;
                let is_on_yellow = (mx - 40.0).powi(2) + (my - 24.0).powi(2) <= 36.0;
                let is_on_green = (mx - 60.0).powi(2) + (my - 24.0).powi(2) <= 36.0;

                if is_on_red {
                    _el.exit();
                } else if is_on_yellow {
                    if let Some(win) = &self.window {
                        win.set_minimized(true);
                    }
                } else if is_on_green {
                    if let Some(win) = &self.window {
                        let maximized = win.is_maximized();
                        win.set_maximized(!maximized);
                    }
                } else {
                    let is_in_sidebar_title = mx < SIDEBAR_W && my < 60.0;
                    let is_in_content_title = mx >= SIDEBAR_W && my < 50.0;
                    if (is_in_sidebar_title || is_in_content_title) && self.popup.is_none() {
                        if let Some(win) = &self.window {
                            let _ = win.drag_window();
                        }
                    } else if self.handle_widget_drag_press() {
                        if let Some(win) = &self.window {
                            win.request_redraw();
                        }
                    } else {
                        self.handle_click(_el);
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                if self.handle_widget_drag_release()
                    && let Some(win) = &self.window
                {
                    win.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => self.draw(),
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        if self.window.is_none() {
            return;
        }

        let frame_start = Instant::now();
        self.frame_count += 1;
        if self.frame_count.is_multiple_of(30) {
            // SAFETY: OpenMutexW opens an existing named mutex. The mutex name is a static
            // string literal. CloseHandle is called on the valid handle returned by OpenMutexW.
            unsafe {
                let h = OpenMutexW(
                    MUTEX_ALL_ACCESS,
                    false,
                    w!("Local\\WinIsland_SingleInstance_Mutex"),
                );
                if let Ok(handle) = h {
                    let _ = windows::Win32::Foundation::CloseHandle(handle);
                } else {
                    _el.exit();
                    return;
                }
            }
        }
        if self.frame_count.is_multiple_of(120) {
            self.update_detected_apps();
        }

        let has_anim = self.switch_anim.is_animating() || self.anim.is_animating();
        let has_popup = self.popup.is_some();
        let is_scrolling = (self.target_scroll_y - self.scroll_y).abs() > 0.1;
        let is_widget_dragging = self.widget_dragging.is_some();

        if !settings_frame_should_continue(has_anim, has_popup, is_scrolling, is_widget_dragging) {
            return;
        }

        let mut redraw = is_widget_dragging || self.switch_anim.tick();
        if self.anim.tick() {
            redraw = true;
        }

        self.ensure_items_cache();
        let max_scroll = self.cached_max_scroll;
        self.target_scroll_y = self.target_scroll_y.clamp(0.0, max_scroll);

        let dt = self
            .last_frame_time
            .elapsed()
            .as_secs_f32()
            .clamp(0.001, 0.05);
        self.last_frame_time = Instant::now();

        let diff = self.target_scroll_y - self.scroll_y;
        let accel = diff * SCROLL_STIFFNESS - self.scroll_vel_y * SCROLL_DAMPING;
        self.scroll_vel_y += accel * dt;
        self.scroll_y += self.scroll_vel_y * dt;

        if self.scroll_y < 0.0 {
            self.scroll_y = 0.0;
            self.scroll_vel_y = 0.0;
        } else if self.scroll_y > max_scroll {
            self.scroll_y = max_scroll;
            self.scroll_vel_y = 0.0;
        }

        if diff.abs() > 0.05 || self.scroll_vel_y.abs() > 0.05 {
            redraw = true;
        } else if (self.scroll_y - self.target_scroll_y).abs() > f32::EPSILON {
            self.scroll_y = self.target_scroll_y;
            self.scroll_vel_y = 0.0;
        }

        if redraw {
            if let Some(win) = &self.window {
                win.request_redraw();
            }
            let target = Duration::from_millis(16);
            let elapsed = frame_start.elapsed();
            if elapsed < target {
                std::thread::sleep(target - elapsed);
            }
        }
    }
}

pub fn run_settings(config: AppConfig) {
    let el = EventLoop::new().unwrap();
    let mut app = SettingsApp::new(config);
    el.run_app(&mut app).unwrap();
}

pub fn bring_settings_to_front() {
    crate::utils::win32::bring_window_to_front("WinIsland Settings");
}

pub(crate) fn resize_surface(
    surface: &mut Surface<Arc<Window>, Arc<Window>>,
    width: u32,
    height: u32,
) {
    if let (Some(w), Some(h)) = (
        std::num::NonZeroU32::new(width),
        std::num::NonZeroU32::new(height),
    ) {
        let _ = surface.resize(w, h);
    }
}
