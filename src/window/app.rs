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
    spring_w: Spring,
    spring_h: Spring,
    spring_r: Spring,
    spring_view: Spring,
    os_w: u32,
    os_h: u32,
    win_x: i32,
    win_y: i32,
    smtc_media_info: MediaInfo,
    last_media_title: String,
    current_lyric_text: String,
    old_lyric_text: String,
    lyric_transition: f32,
    idle_timer: Instant,
    last_glass_refresh: Instant,
    spring_hide: Spring,
    auto_hidden: bool,
    fullscreen_hidden: bool,
    hide_origin: Option<(i32, i32)>,
    hide_edge: HideEdge,
    is_dragging: bool,
    dismissing_notification: bool,
    drag_start_px: i32,
    drag_start_py: i32,
    drag_start_hide_val: f32,
    manually_hidden: bool,
    drag_has_moved: bool,
    last_update_time: Instant,
    last_render_time: Instant,
    last_topmost_check: Instant,
    last_fullscreen_check: Instant,
    last_config_check: Instant,
    last_monitor_check: Instant,
    last_working_set_trim: Instant,
    last_config_modified: Option<SystemTime>,
    next_frame_deadline: Instant,
    animation_frame_interval: Duration,
    last_mon_size: (u32, u32),
    last_mon_pos: (i32, i32),
    lyric_scroll_offset: f32,
    lyric_scroll_pause: f32,
    seeking_progress: bool,
    seeking_bar_left: f32,
    seeking_bar_right: f32,
    seeking_duration_ms: u64,
    seeking_preview_ms: u64,
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
            spring_w: Spring::new(config.base_width * config.global_scale),
            spring_h: Spring::new(config.base_height * config.global_scale),
            spring_r: Spring::new((config.base_height * config.global_scale) / 2.0),
            spring_view: Spring::new(0.0),
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
            smtc_media_info: MediaInfo::default(),
            last_media_title: String::new(),
            current_lyric_text: String::new(),
            old_lyric_text: String::new(),
            lyric_transition: 1.0,
            idle_timer: Instant::now(),
            last_glass_refresh: Instant::now(),
            spring_hide: Spring::new(0.0),
            auto_hidden: false,
            fullscreen_hidden: false,
            hide_origin: None,
            hide_edge: HideEdge::Top,
            is_dragging: false,
            dismissing_notification: false,
            drag_start_px: 0,
            drag_start_py: 0,
            drag_start_hide_val: 0.0,
            manually_hidden: false,
            drag_has_moved: false,
            last_update_time: Instant::now(),
            last_render_time: Instant::now(),
            last_topmost_check: Instant::now(),
            last_fullscreen_check: Instant::now(),
            last_config_check: Instant::now(),
            last_monitor_check: Instant::now(),
            last_working_set_trim: Instant::now(),
            last_config_modified,
            next_frame_deadline: Instant::now(),
            animation_frame_interval: DEFAULT_ANIMATION_FRAME_INTERVAL,
            last_mon_size: (0, 0),
            last_mon_pos: (0, 0),
            lyric_scroll_offset: 0.0,
            lyric_scroll_pause: 0.0,
            seeking_progress: false,
            seeking_bar_left: 0.0,
            seeking_bar_right: 0.0,
            seeking_duration_ms: 0,
            seeking_preview_ms: 0,
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
        self.auto_hidden || self.fullscreen_hidden || self.manually_hidden
    }

    fn reveal_island(&mut self) {
        self.auto_hidden = false;
        self.fullscreen_hidden = false;
        self.manually_hidden = false;
        self.spring_hide.velocity = -0.65;
        self.idle_timer = Instant::now();
    }
}
