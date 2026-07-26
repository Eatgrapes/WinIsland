use crate::core::audio::AudioProcessor;
use crate::core::config::AppConfig;
use crate::core::context::ContextManager;
use crate::core::persistence::{get_config_path, load_config};
use crate::core::smtc::{MediaInfo, SmtcListener};
use crate::plugin::PluginManager;
use crate::plugin::zip_loader::PluginManifest;
use crate::ui::compact::CompactOverlay;
use crate::utils::physics::Spring;
use crate::window::d3d::D3DRenderer;
use crate::window::settings::SettingsApp;
use crate::window::tray::TrayManager;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};
use winit::dpi::PhysicalPosition;
use winit::window::Window;

mod events;
mod frame;
mod input;
mod layout;
mod lifecycle;
mod startup;
mod system;

type InstallResult = Result<(PluginManifest, PathBuf, Vec<String>), String>;
const RIGHT_DRAG_THRESHOLD: i32 = 4;
pub(super) const DEFAULT_ANIMATION_REFRESH_RATE_MILLIHERTZ: u32 = 144_000;
pub(super) const DEFAULT_ANIMATION_FRAME_INTERVAL: Duration = Duration::from_micros(6_944);

#[derive(Clone, Copy)]
enum HideEdge {
    Top,
    Bottom,
    Left,
    Right,
}

fn should_show_widget_view(smtc_enabled: bool, has_media: bool) -> bool {
    !(smtc_enabled && has_media)
}

pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<D3DRenderer>,
    settings: Option<SettingsApp>,
    tray: Option<TrayManager>,
    smtc: SmtcListener,
    audio: AudioProcessor,
    compact_overlay: CompactOverlay,
    config: AppConfig,
    expanded: bool,
    widget_view: bool,
    visible: bool,
    springs: IslandSprings,
    os_w: u32,
    os_h: u32,
    win_x: i32,
    win_y: i32,
    configured_win_x: i32,
    configured_win_y: i32,
    smtc_media_info: MediaInfo,
    last_media_title: String,
    lyrics: LyricState,
    idle_timer: Instant,
    last_glass_refresh: Instant,
    hide: HideState,
    is_dragging: bool,
    dismissing_notification: bool,
    drag_start_px: i32,
    drag_start_py: i32,
    drag_start_hide_val: f32,
    drag_has_moved: bool,
    last_update_time: Instant,
    last_render_time: Instant,
    last_topmost_check: Instant,
    last_fullscreen_check: Instant,
    last_config_check: Instant,
    last_monitor_check: Instant,
    position_restore_after: Option<Instant>,
    last_working_set_trim: Instant,
    last_config_modified: Option<SystemTime>,
    next_frame_deadline: Instant,
    animation_frame_interval: Duration,
    last_mon_size: (u32, u32),
    last_mon_pos: (i32, i32),
    seek: SeekDrag,
    is_fullscreen_suppressed: bool,
    is_cursor_suppressed: bool,
    touch_id: Option<u64>,
    touch_pos: PhysicalPosition<f64>,
    ctx_mgr: ContextManager,
    plugin_mgr: PluginManager,
    plugin_media_source: Option<crate::core::smtc::MediaInfo>,
    pending_install: Option<mpsc::Receiver<InstallResult>>,
    right_press_cursor: Option<(i32, i32)>,
    is_right_dragging: bool,
    right_drag_start_offset: Option<(i32, i32)>,
}

impl Default for App {
    fn default() -> Self {
        let config = load_config();
        let last_config_modified = std::fs::metadata(get_config_path())
            .and_then(|metadata| metadata.modified())
            .ok();
        crate::utils::font::FontManager::global()
            .set_custom_font_path(config.custom_font_path.as_deref());
        Self {
            window: None,
            renderer: None,
            settings: None,
            tray: None,
            config: config.clone(),
            expanded: false,
            widget_view: false,
            visible: true,
            springs: IslandSprings::new(&config),
            smtc: SmtcListener::new(
                config.lyrics_source.clone(),
                config.lyrics_fallback,
                config.smtc_apps.clone(),
            ),
            audio: AudioProcessor::new(),
            compact_overlay: CompactOverlay::default(),
            os_w: 0,
            os_h: 0,
            win_x: 0,
            win_y: 0,
            configured_win_x: 0,
            configured_win_y: 0,
            smtc_media_info: MediaInfo::default(),
            last_media_title: String::new(),
            lyrics: LyricState::default(),
            idle_timer: Instant::now(),
            last_glass_refresh: Instant::now(),
            hide: HideState::default(),
            is_dragging: false,
            dismissing_notification: false,
            drag_start_px: 0,
            drag_start_py: 0,
            drag_start_hide_val: 0.0,
            drag_has_moved: false,
            last_update_time: Instant::now(),
            last_render_time: Instant::now(),
            last_topmost_check: Instant::now(),
            last_fullscreen_check: Instant::now(),
            last_config_check: Instant::now(),
            last_monitor_check: Instant::now(),
            position_restore_after: None,
            last_working_set_trim: Instant::now(),
            last_config_modified,
            next_frame_deadline: Instant::now(),
            animation_frame_interval: DEFAULT_ANIMATION_FRAME_INTERVAL,
            last_mon_size: (0, 0),
            last_mon_pos: (0, 0),
            seek: SeekDrag::default(),
            is_fullscreen_suppressed: false,
            is_cursor_suppressed: false,
            touch_id: None,
            touch_pos: PhysicalPosition::new(0.0, 0.0),
            ctx_mgr: ContextManager::new(),
            plugin_mgr: PluginManager::default(),
            plugin_media_source: None,
            pending_install: None,
            right_press_cursor: None,
            is_right_dragging: false,
            right_drag_start_offset: None,
        }
    }
}

#[derive(Default)]
struct SeekDrag {
    active: bool,
    bar_left: f32,
    bar_right: f32,
    duration_ms: u64,
    preview_ms: u64,
}

impl SeekDrag {
    fn begin(&mut self, bar_left: f32, bar_right: f32, duration_ms: u64, preview_ms: u64) {
        self.active = true;
        self.bar_left = bar_left;
        self.bar_right = bar_right;
        self.duration_ms = duration_ms;
        self.preview_ms = preview_ms;
    }

    fn preview_at(&mut self, click_x: f32) {
        let bar_width = self.bar_right - self.bar_left;
        let ratio = if bar_width > 0.0 {
            ((click_x - self.bar_left) / bar_width).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.preview_ms = (ratio as f64 * self.duration_ms as f64) as u64;
    }
}

struct LyricState {
    current_text: String,
    old_text: String,
    transition: f32,
    scroll_offset: f32,
    scroll_pause: f32,
}

impl Default for LyricState {
    fn default() -> Self {
        Self {
            current_text: String::new(),
            old_text: String::new(),
            transition: 1.0,
            scroll_offset: 0.0,
            scroll_pause: 0.0,
        }
    }
}

impl LyricState {
    fn transition_to(&mut self, text: String) {
        self.old_text = std::mem::replace(&mut self.current_text, text);
        self.transition = 0.0;
        self.scroll_offset = 0.0;
        self.scroll_pause = 0.0;
    }
}

struct HideState {
    auto: bool,
    manual: bool,
    fullscreen: bool,
    fullscreen_reveal_override: bool,
    origin: Option<(i32, i32)>,
    edge: HideEdge,
}

impl Default for HideState {
    fn default() -> Self {
        Self {
            auto: false,
            manual: false,
            fullscreen: false,
            fullscreen_reveal_override: false,
            origin: None,
            edge: HideEdge::Top,
        }
    }
}

impl HideState {
    fn is_hidden(&self) -> bool {
        self.auto || self.fullscreen || self.manual
    }
}

struct IslandSprings {
    w: Spring,
    h: Spring,
    r: Spring,
    view: Spring,
    hide: Spring,
}

impl IslandSprings {
    fn new(config: &AppConfig) -> Self {
        Self {
            w: Spring::new(config.base_width * config.global_scale),
            h: Spring::new(config.base_height * config.global_scale),
            r: Spring::new((config.base_height * config.global_scale) / 2.0),
            view: Spring::new(0.0),
            hide: Spring::new(0.0),
        }
    }

    fn any_animating(&self) -> bool {
        self.w.velocity.abs() > 0.001
            || self.h.velocity.abs() > 0.001
            || self.r.velocity.abs() > 0.001
            || self.view.velocity.abs() > 0.001
            || self.hide.velocity.abs() > 0.001
    }
}

struct IslandLayout {
    offset_x: f64,
    island_y: f64,
    current_island_x: f64,
    current_island_y: f64,
    stable_island_y: f64,
    hide_distance: f64,
    content_hide_ratio: f32,
    hidden_reveal_x: f64,
    hidden_reveal_y: f64,
    hidden_reveal_w: f64,
    hidden_reveal_h: f64,
}

impl App {
    fn is_hidden(&self) -> bool {
        self.hide.is_hidden()
    }

    fn reveal_island(&mut self) {
        self.hide.auto = false;
        self.hide.fullscreen = false;
        self.hide.manual = false;
        if self.is_fullscreen_suppressed {
            self.hide.fullscreen_reveal_override = true;
        }
        self.springs.hide.velocity = -0.65;
        self.idle_timer = Instant::now();
    }
}
