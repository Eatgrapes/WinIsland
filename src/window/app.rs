use crate::core::audio::AudioProcessor;
use crate::core::config::AppConfig;
use crate::core::context::ContextManager;
use crate::core::persistence::load_config;
use crate::core::smtc::SmtcListener;
use crate::plugin::PluginManager;
use crate::plugin::zip_loader::PluginManifest;
use crate::utils::physics::Spring;
use crate::window::tray::TrayManager;
use softbuffer::{Context, Surface};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Instant;
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

fn should_show_widget_view(smtc_enabled: bool, has_media: bool, is_playing: bool) -> bool {
    !(smtc_enabled && has_media && is_playing)
}

pub struct App {
    window: Option<Arc<Window>>,
    context: Option<Context<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    tray: Option<TrayManager>,
    smtc: SmtcListener,
    audio: AudioProcessor,
    config: AppConfig,
    expanded: bool,
    widget_view: bool,
    visible: bool,
    border_weights: [f32; 4],
    target_border_weights: [f32; 4],
    spring_w: Spring,
    spring_h: Spring,
    spring_r: Spring,
    spring_view: Spring,
    os_w: u32,
    os_h: u32,
    win_x: i32,
    win_y: i32,
    frame_count: u64,
    last_media_title: String,
    last_media_playing: bool,
    current_lyric_text: String,
    old_lyric_text: String,
    lyric_transition: f32,
    idle_timer: Instant,
    last_glass_refresh: Instant,
    spring_hide: Spring,
    auto_hidden: bool,
    is_dragging: bool,
    drag_start_py: i32,
    drag_start_hide_val: f32,
    manually_hidden: bool,
    drag_has_moved: bool,
    last_frame_time: Instant,
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
    right_press_time: Option<Instant>,
    right_press_cursor: Option<(i32, i32)>,
    is_right_dragging: bool,
    right_drag_start_pos: Option<(i32, i32)>,
    right_drag_start_offset: Option<(i32, i32)>,
}

impl Default for App {
    fn default() -> Self {
        let config = load_config();
        Self {
            window: None,
            context: None,
            surface: None,
            tray: None,
            config: config.clone(),
            expanded: false,
            widget_view: false,
            visible: true,
            border_weights: [0.0; 4],
            target_border_weights: [0.0; 4],
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
            os_w: 0,
            os_h: 0,
            win_x: 0,
            win_y: 0,
            frame_count: 0,
            last_media_title: String::new(),
            last_media_playing: false,
            current_lyric_text: String::new(),
            old_lyric_text: String::new(),
            lyric_transition: 1.0,
            idle_timer: Instant::now(),
            last_glass_refresh: Instant::now(),
            spring_hide: Spring::new(0.0),
            auto_hidden: false,
            is_dragging: false,
            drag_start_py: 0,
            drag_start_hide_val: 0.0,
            manually_hidden: false,
            drag_has_moved: false,
            last_frame_time: Instant::now(),
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
            right_press_time: None,
            right_press_cursor: None,
            is_right_dragging: false,
            right_drag_start_pos: None,
            right_drag_start_offset: None,
        }
    }
}

struct IslandLayout {
    dock_bottom: bool,
    offset_x: f64,
    island_y: f64,
    current_island_y: f64,
    hide_distance: f64,
    hidden_handle_y: f64,
    hidden_handle_h: f64,
}
