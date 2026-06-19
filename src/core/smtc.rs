use crate::core::lyrics::{LyricLine, fetch_lyrics};
use crate::core::persistence::{load_config, save_config};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

#[derive(Clone, Debug)]
pub struct MediaInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub is_playing: bool,
    pub thumbnail: Option<Arc<Vec<u8>>>,
    pub thumbnail_hash: u64,
    pub spectrum: [f32; 6],
    pub position_ms: u64,
    pub last_update: Instant,
    pub last_thumbnail_fetch: Instant,
    pub lyrics: Option<Arc<Vec<LyricLine>>>,
    pub last_smtc_pos: u64,
    pub duration_secs: u64,
    pub duration_ms: u64,
}

impl Default for MediaInfo {
    fn default() -> Self {
        Self {
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            is_playing: false,
            thumbnail: None,
            thumbnail_hash: 0,
            spectrum: [0.0; 6],
            position_ms: 0,
            last_update: Instant::now(),
            last_thumbnail_fetch: Instant::now() - Duration::from_secs(10),
            lyrics: None,
            last_smtc_pos: 0,
            duration_secs: 0,
            duration_ms: 0,
        }
    }
}

impl MediaInfo {
    pub fn effective_duration_ms(&self) -> u64 {
        if self.duration_ms > 0 {
            self.duration_ms
        } else if self.duration_secs > 0 {
            self.duration_secs * 1000
        } else {
            0
        }
    }

    pub fn current_lyric(&self, delay_ms: i64) -> Option<String> {
        let lyrics = self.lyrics.as_ref()?;
        if lyrics.is_empty() {
            return None;
        }

        let raw_pos = if self.is_playing {
            self.position_ms
                .saturating_add(self.last_update.elapsed().as_millis() as u64)
        } else {
            self.position_ms
        };
        let current_pos = (raw_pos as i64 + delay_ms).max(0) as u64;

        match lyrics.binary_search_by_key(&current_pos, |line| line.time_ms) {
            Ok(idx) => Some(lyrics[idx].text.clone()),
            Err(idx) => {
                if idx > 0 {
                    Some(lyrics[idx - 1].text.clone())
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
enum PlaybackCommand {
    Toggle,
    Next,
    Prev,
}

pub struct SmtcListener {
    info_rx: watch::Receiver<MediaInfo>,
    seek_tx: mpsc::UnboundedSender<u64>,
    playback_tx: mpsc::UnboundedSender<PlaybackCommand>,
    lyrics_source_tx: mpsc::UnboundedSender<String>,
    lyrics_fallback_tx: mpsc::UnboundedSender<bool>,
    lyrics_local_dir_tx: mpsc::UnboundedSender<Option<String>>,
    allowed_apps_tx: mpsc::UnboundedSender<Vec<String>>,
    cancel_token: CancellationToken,
}

impl SmtcListener {
    pub fn new(source: String, fallback: bool, allowed: Vec<String>) -> Self {
        let (info_tx, info_rx) = watch::channel(MediaInfo::default());
        let (seek_tx, seek_rx) = mpsc::unbounded_channel();
        let (playback_tx, playback_rx) = mpsc::unbounded_channel();
        let (lyrics_source_tx, lyrics_source_rx) = mpsc::unbounded_channel();
        let (lyrics_fallback_tx, lyrics_fallback_rx) = mpsc::unbounded_channel();
        let (lyrics_local_dir_tx, lyrics_local_dir_rx) = mpsc::unbounded_channel();
        let (allowed_apps_tx, allowed_apps_rx) = mpsc::unbounded_channel();
        let cancel_token = CancellationToken::new();

        let _ = lyrics_source_tx.send(source);
        let _ = lyrics_fallback_tx.send(fallback);
        let _ = lyrics_local_dir_tx.send(load_config().lyrics_local_dir);
        let _ = allowed_apps_tx.send(allowed);

        let cancel = cancel_token.clone();
        tokio::task::spawn_blocking(move || {
            smtc_poll_loop(
                info_tx,
                seek_rx,
                playback_rx,
                lyrics_source_rx,
                lyrics_fallback_rx,
                lyrics_local_dir_rx,
                allowed_apps_rx,
                cancel,
            );
        });

        Self {
            info_rx,
            seek_tx,
            playback_tx,
            lyrics_source_tx,
            lyrics_fallback_tx,
            lyrics_local_dir_tx,
            allowed_apps_tx,
            cancel_token,
        }
    }

    pub fn set_allowed_apps(&self, apps: Vec<String>) {
        let _ = self.allowed_apps_tx.send(apps);
    }

    pub fn set_lyrics_source(&self, source: String) {
        let _ = self.lyrics_source_tx.send(source);
    }

    pub fn set_lyrics_fallback(&self, fallback: bool) {
        let _ = self.lyrics_fallback_tx.send(fallback);
    }

    pub fn set_lyrics_local_dir(&self, dir: Option<String>) {
        let _ = self.lyrics_local_dir_tx.send(dir);
    }

    pub fn get_info(&self) -> MediaInfo {
        self.info_rx.borrow().clone()
    }

    pub fn request_seek(&self, position_ms: u64) {
        let _ = self.seek_tx.send(position_ms);
    }

    pub fn request_toggle_play(&self) {
        let _ = self.playback_tx.send(PlaybackCommand::Toggle);
    }

    pub fn request_next(&self) {
        let _ = self.playback_tx.send(PlaybackCommand::Next);
    }

    pub fn request_prev(&self) {
        let _ = self.playback_tx.send(PlaybackCommand::Prev);
    }
}

impl Drop for SmtcListener {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}

#[allow(clippy::too_many_arguments)]
fn smtc_poll_loop(
    info_tx: watch::Sender<MediaInfo>,
    mut seek_rx: mpsc::UnboundedReceiver<u64>,
    mut playback_rx: mpsc::UnboundedReceiver<PlaybackCommand>,
    mut lyrics_source_rx: mpsc::UnboundedReceiver<String>,
    mut lyrics_fallback_rx: mpsc::UnboundedReceiver<bool>,
    mut lyrics_local_dir_rx: mpsc::UnboundedReceiver<Option<String>>,
    mut allowed_apps_rx: mpsc::UnboundedReceiver<Vec<String>>,
    cancel: CancellationToken,
) {
    // SAFETY: CoInitializeEx initializes COM for this thread. We use
    // COINIT_MULTITHREADED because tokio's spawn_blocking pool is MTA.
    // If it fails (e.g. already initialized with a different mode), we
    // skip creating the guard so CoUninitialize is not called unbalanced.
    let com_initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
    struct ComGuard;
    impl Drop for ComGuard {
        fn drop(&mut self) {
            // SAFETY: CoUninitialize balances the successful CoInitializeEx
            // that triggered the creation of this guard.
            unsafe { CoUninitialize() };
        }
    }
    let _com_guard = com_initialized.then_some(ComGuard);

    let manager = match GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
        Ok(op) => match op.join() {
            Ok(m) => m,
            Err(_) => {
                log::error!("SMTC: failed to get session manager");
                return;
            }
        },
        Err(_) => {
            log::error!("SMTC: RequestAsync failed");
            return;
        }
    };
    log::info!(
        "SMTC: session manager created (COM initialized: {})",
        com_initialized
    );

    // COM event bridge: COM callback -> std::sync::mpsc -> polling loop
    let (event_tx, event_rx) = std::sync::mpsc::channel::<()>();
    let handler = TypedEventHandler::new(move |_m, _| {
        let _ = event_tx.send(());
        Ok(())
    });
    let _ = manager.SessionsChanged(&handler);

    // Local state mirrored from channels
    let mut current_lyrics_source: String = "163".to_string();
    let mut current_lyrics_fallback: bool = true;
    let mut current_lyrics_local_dir: Option<String> = None;
    let mut current_allowed_apps: Vec<String> = Vec::new();

    // Drain initial config values from channels
    while let Ok(src) = lyrics_source_rx.try_recv() {
        current_lyrics_source = src;
    }
    while let Ok(fb) = lyrics_fallback_rx.try_recv() {
        current_lyrics_fallback = fb;
    }
    while let Ok(dir) = lyrics_local_dir_rx.try_recv() {
        current_lyrics_local_dir = dir;
    }
    while let Ok(apps) = allowed_apps_rx.try_recv() {
        current_allowed_apps = apps;
    }

    let mut last_session_seen = Instant::now();
    let mut last_was_playing = false;

    // Initial update with retries for SMTC timeline readiness.
    // Some music apps (Spotify, Netease) take 1-2s to populate
    // TimelineProperties after session creation, so we retry up
    // to 2 seconds (10 × 200ms).
    for attempt in 0..10 {
        update_media_info(
            &manager,
            &info_tx,
            &current_lyrics_source,
            current_lyrics_fallback,
            current_lyrics_local_dir.as_deref(),
            &mut current_allowed_apps,
            true,
            &mut last_session_seen,
            &mut last_was_playing,
        );
        let info = info_tx.borrow();
        let timeline_ready = info.duration_ms > 0
            || info.position_ms > 0
            || !info.is_playing
            || info.title.is_empty();
        if timeline_ready {
            if attempt > 0 {
                log::info!("SMTC: initial timeline ready after {} retries", attempt + 1);
            }
            drop(info);
            break;
        }
        drop(info);
        if attempt < 9 {
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    let mut last_manager_refresh = Instant::now();
    let mut current_manager = manager;
    let mut last_regular_update = Instant::now();
    let mut regular_poll_count = 0u32;

    while !cancel.is_cancelled() {
        // Refresh manager every 30 seconds
        if last_manager_refresh.elapsed() > Duration::from_secs(30) {
            if let Ok(new_mgr_op) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
                && let Ok(new_mgr) = new_mgr_op.join()
            {
                current_manager = new_mgr;
                let _ = current_manager.SessionsChanged(&handler);
            }
            log::info!("SMTC: manager refreshed (30s interval)");
            last_manager_refresh = Instant::now();
        }

        // Drain config channels
        while let Ok(src) = lyrics_source_rx.try_recv() {
            if src != current_lyrics_source {
                current_lyrics_source = src;
                // Re-fetch lyrics for current song on source change
                let info = info_tx.borrow();
                if !info.title.is_empty() {
                    let title = info.title.clone();
                    let artist = info.artist.clone();
                    let duration = info.duration_secs;
                    let src = current_lyrics_source.clone();
                    let fb = current_lyrics_fallback;
                    let info_tx_clone = info_tx.clone();
                    let local_dir = current_lyrics_local_dir.clone();
                    drop(info);
                    tokio::spawn(async move {
                        if let Some(lyrics) =
                            fetch_lyrics(&title, &artist, duration, &src, fb, local_dir.as_deref())
                                .await
                        {
                            let current = info_tx_clone.borrow();
                            if current.title == title && current.artist == artist {
                                drop(current);
                                let mut new_info = info_tx_clone.borrow().clone();
                                new_info.lyrics = Some(lyrics);
                                let _ = info_tx_clone.send(new_info);
                            }
                        }
                    });
                }
            }
        }
        while let Ok(fb) = lyrics_fallback_rx.try_recv() {
            current_lyrics_fallback = fb;
        }
        while let Ok(dir) = lyrics_local_dir_rx.try_recv() {
            if dir != current_lyrics_local_dir {
                current_lyrics_local_dir = dir;
                // Re-fetch lyrics for current song when local dir changes
                let info = info_tx.borrow();
                if !info.title.is_empty() {
                    let title = info.title.clone();
                    let artist = info.artist.clone();
                    let duration = info.duration_secs;
                    let src = current_lyrics_source.clone();
                    let fb = current_lyrics_fallback;
                    let info_tx_clone = info_tx.clone();
                    let local_dir = current_lyrics_local_dir.clone();
                    drop(info);
                    tokio::spawn(async move {
                        if let Some(lyrics) =
                            fetch_lyrics(&title, &artist, duration, &src, fb, local_dir.as_deref())
                                .await
                        {
                            let current = info_tx_clone.borrow();
                            if current.title == title && current.artist == artist {
                                drop(current);
                                let mut new_info = info_tx_clone.borrow().clone();
                                new_info.lyrics = Some(lyrics);
                                let _ = info_tx_clone.send(new_info);
                            }
                        }
                    });
                }
            }
        }
        while let Ok(apps) = allowed_apps_rx.try_recv() {
            current_allowed_apps = apps;
        }

        // Handle seek request (keep only the latest)
        let mut seek_pos = None;
        while let Ok(v) = seek_rx.try_recv() {
            seek_pos = Some(v);
        }
        if let Some(seek_pos) = seek_pos
            && let Some(session) = get_target_session(&current_manager, &current_allowed_apps)
        {
            log::info!("SMTC: seek to {}ms", seek_pos);
            let ticks = seek_pos as i64 * 10_000;
            let _ = session.TryChangePlaybackPositionAsync(ticks);
            let mut info = info_tx.borrow().clone();
            info.position_ms = seek_pos;
            info.last_update = Instant::now();
            // Do not update last_smtc_pos here: SMTC timeline can lag after seek, and treating
            // seek_pos as authoritative would make the next poll think SMTC changed and sync back.
            let _ = info_tx.send(info);
        }

        // Handle playback commands
        while let Ok(cmd) = playback_rx.try_recv() {
            log::info!("SMTC: playback command {:?}", cmd);
            if let Some(session) = get_target_session(&current_manager, &current_allowed_apps) {
                match cmd {
                    PlaybackCommand::Toggle => {
                        if let Ok(pb_info) = session.GetPlaybackInfo()
                            && let Ok(status) = pb_info.PlaybackStatus()
                        {
                            if status == windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing {
                                    let _ = session.TryPauseAsync();
                                } else {
                                    let _ = session.TryPlayAsync();
                                }
                        }
                    }
                    PlaybackCommand::Next => {
                        let _ = session.TrySkipNextAsync();
                    }
                    PlaybackCommand::Prev => {
                        let _ = session.TrySkipPreviousAsync();
                    }
                }
            }
        }

        // Check COM events — when triggered, immediately update and reset the regular timer
        if event_rx.try_recv().is_ok() {
            log::info!("SMTC: session change event received, updating immediately");
            update_media_info(
                &current_manager,
                &info_tx,
                &current_lyrics_source,
                current_lyrics_fallback,
                current_lyrics_local_dir.as_deref(),
                &mut current_allowed_apps,
                true,
                &mut last_session_seen,
                &mut last_was_playing,
            );
            last_regular_update = Instant::now();
        }

        // Regular update — only if last update was > 300ms ago
        if last_regular_update.elapsed() > Duration::from_millis(300) {
            // Periodically run auto_allow as a safety net: every 10th poll (~3s)
            // in case COM session-change events were missed (e.g. handler lost
            // during manager refresh).
            regular_poll_count += 1;
            let do_auto_allow = regular_poll_count.is_multiple_of(10);
            update_media_info(
                &current_manager,
                &info_tx,
                &current_lyrics_source,
                current_lyrics_fallback,
                current_lyrics_local_dir.as_deref(),
                &mut current_allowed_apps,
                do_auto_allow,
                &mut last_session_seen,
                &mut last_was_playing,
            );
            last_regular_update = Instant::now();
        }

        std::thread::sleep(Duration::from_millis(300));
    }
}

#[allow(clippy::too_many_arguments)]
fn update_media_info(
    manager: &GlobalSystemMediaTransportControlsSessionManager,
    info_tx: &watch::Sender<MediaInfo>,
    lyrics_source: &str,
    lyrics_fallback: bool,
    local_dir: Option<&str>,
    allowed_apps: &mut Vec<String>,
    auto_allow: bool,
    last_session_seen: &mut Instant,
    last_was_playing: &mut bool,
) {
    if auto_allow {
        *allowed_apps = auto_allow_new_apps(manager, allowed_apps);
    }

    if let Some(session) = get_target_session(manager, allowed_apps) {
        *last_session_seen = Instant::now();
        let _ = fetch_properties(&session, info_tx, lyrics_source, lyrics_fallback, local_dir);
        *last_was_playing = info_tx.borrow().is_playing;
    } else if *last_was_playing {
        let info = info_tx.borrow();
        if !info.title.is_empty() {
            drop(info);
            let _ = info_tx.send(MediaInfo::default());
            log::info!("SMTC: app closed while playing, cleared immediately");
        }
        *last_was_playing = false;
    } else if last_session_seen.elapsed() > Duration::from_secs(15) {
        let info = info_tx.borrow();
        if !info.title.is_empty() {
            drop(info);
            let _ = info_tx.send(MediaInfo::default());
            log::info!("SMTC: paused session lost for >15s, cleared media info");
        }
    }
}

fn auto_allow_new_apps(
    mgr: &GlobalSystemMediaTransportControlsSessionManager,
    allowed: &[String],
) -> Vec<String> {
    let mut new_allowed = allowed.to_vec();
    let mut new_app_ids: Vec<String> = Vec::new();
    if let Ok(sessions) = mgr.GetSessions()
        && let Ok(count) = sessions.Size()
    {
        for i in 0..count {
            if let Ok(session) = sessions.GetAt(i)
                && let Ok(pb_info) = session.GetPlaybackInfo()
                && let Ok(playback_type) = pb_info.PlaybackType()
                && let Ok(value) = playback_type.Value()
                && value == windows::Media::MediaPlaybackType::Music
                && let Ok(id) = session.SourceAppUserModelId()
            {
                let app_id = id.to_string();
                if !new_app_ids.contains(&app_id) {
                    new_app_ids.push(app_id);
                }
            }
        }
    }

    if new_app_ids.is_empty() {
        return new_allowed;
    }

    let mut config = load_config();
    let mut changed = false;

    for app_id in &new_app_ids {
        if !config.smtc_known_apps.contains(app_id) {
            let is_first_run = config.smtc_known_apps.is_empty();
            config.smtc_known_apps.push(app_id.clone());

            if is_first_run && !config.smtc_apps.contains(app_id) {
                config.smtc_apps.push(app_id.clone());
                if !new_allowed.contains(app_id) {
                    new_allowed.push(app_id.clone());
                }
            }
            changed = true;
        }
    }

    if changed {
        save_config(&config);
        log::info!("SMTC: auto-allowed new session(s): {:?}", new_app_ids);
    }

    new_allowed
}

fn get_target_session(
    mgr: &GlobalSystemMediaTransportControlsSessionManager,
    allowed: &[String],
) -> Option<GlobalSystemMediaTransportControlsSession> {
    if allowed.is_empty() {
        return None;
    }
    let mut audio_session = None;
    if let Ok(sessions) = mgr.GetSessions()
        && let Ok(count) = sessions.Size()
    {
        for i in 0..count {
            if let Ok(session) = sessions.GetAt(i) {
                if let Ok(id) = session.SourceAppUserModelId() {
                    let app_id = id.to_string();
                    if !allowed.iter().any(|a| a == &app_id) {
                        continue;
                    }
                } else {
                    continue;
                }
                if !is_music_session(&session) {
                    continue;
                }
                if let Ok(pb_info) = session.GetPlaybackInfo()
                        && let Ok(status) = pb_info.PlaybackStatus()
                            && status == windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing {
                                return Some(session);
                            }
                if audio_session.is_none() {
                    audio_session = Some(session);
                }
            }
        }
    }
    if let Some(session) = audio_session {
        return Some(session);
    }
    if let Ok(session) = mgr.GetCurrentSession() {
        if let Ok(id) = session.SourceAppUserModelId() {
            let app_id = id.to_string();
            if !allowed.iter().any(|a| a == &app_id) {
                return None;
            }
        } else {
            return None;
        }
        if is_music_session(&session) {
            return Some(session);
        }
    }
    None
}

fn is_music_session(session: &GlobalSystemMediaTransportControlsSession) -> bool {
    if let Ok(pb_info) = session.GetPlaybackInfo()
        && let Ok(playback_type) = pb_info.PlaybackType()
        && let Ok(value) = playback_type.Value()
        && value == windows::Media::MediaPlaybackType::Video
    {
        return false;
    }
    true
}

thread_local! {
    static LAST_TIMELINE_FETCH: std::cell::Cell<Option<Instant>> = const { std::cell::Cell::new(None) };
    static LAST_FETCHED_SMTC_POS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static LAST_FETCHED_DURATION_SECS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static LAST_FETCHED_DURATION_MS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

fn fetch_properties(
    session: &GlobalSystemMediaTransportControlsSession,
    info_tx: &watch::Sender<MediaInfo>,
    lyrics_source: &str,
    lyrics_fallback: bool,
    local_dir: Option<&str>,
) -> windows::core::Result<()> {
    if !is_music_session(session) {
        let info = info_tx.borrow();
        if !info.title.is_empty() {
            drop(info);
            let _ = info_tx.send(MediaInfo::default());
        }
        return Ok(());
    }

    let props = session.TryGetMediaPropertiesAsync()?.join()?;
    let pb_info = session.GetPlaybackInfo()?;
    let is_playing = pb_info.PlaybackStatus()? == windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;

    let new_title = props.Title()?.to_string();
    let new_artist = props.Artist()?.to_string();
    let new_album = props.AlbumTitle()?.to_string();

    let song_changed = {
        let info = info_tx.borrow();
        info.title != new_title || info.artist != new_artist || info.album != new_album
    };

    let should_fetch = song_changed
        || (info_tx.borrow().is_playing != is_playing)
        || LAST_TIMELINE_FETCH.with(|cell| match cell.get() {
            Some(last) => last.elapsed() >= Duration::from_millis(500),
            None => true,
        });

    let mut smtc_pos = LAST_FETCHED_SMTC_POS.with(|cell| cell.get());
    let mut duration_secs = LAST_FETCHED_DURATION_SECS.with(|cell| cell.get());
    let mut duration_ms_from_tl = LAST_FETCHED_DURATION_MS.with(|cell| cell.get());

    if should_fetch {
        if let Ok(tl) = session.GetTimelineProperties() {
            if let Ok(pos) = tl.Position() {
                let raw = pos.Duration;
                smtc_pos = if raw > 0 { (raw / 10_000) as u64 } else { 0 };
            } else {
                smtc_pos = 0;
            }

            if let Ok(end) = tl.EndTime() {
                let raw = end.Duration;
                if raw > 0 {
                    duration_secs = (raw / 10_000_000) as u64;
                    duration_ms_from_tl = (raw / 10_000) as u64;
                } else {
                    duration_secs = 0;
                    duration_ms_from_tl = 0;
                }
            } else {
                duration_secs = 0;
                duration_ms_from_tl = 0;
            }

            LAST_TIMELINE_FETCH.with(|cell| cell.set(Some(Instant::now())));
            LAST_FETCHED_SMTC_POS.with(|cell| cell.set(smtc_pos));
            LAST_FETCHED_DURATION_SECS.with(|cell| cell.set(duration_secs));
            LAST_FETCHED_DURATION_MS.with(|cell| cell.set(duration_ms_from_tl));
        } else {
            smtc_pos = 0;
            duration_secs = 0;
            duration_ms_from_tl = 0;
            LAST_TIMELINE_FETCH.with(|cell| cell.set(Some(Instant::now())));
        }
    }

    let mut should_fetch_lyrics = false;
    let mut should_fetch_thumbnail = false;

    {
        let mut info = info_tx.borrow().clone();
        let song_changed =
            info.title != new_title || info.artist != new_artist || info.album != new_album;
        if song_changed {
            log::info!(
                "SMTC: track changed -> {} - {} / {}",
                new_title,
                new_artist,
                new_album
            );
            info.title = new_title.clone();
            info.artist = new_artist.clone();
            info.album = new_album.clone();
            info.duration_secs = duration_secs;
            info.duration_ms = duration_ms_from_tl;
            info.lyrics = None;
            info.thumbnail = None;
            info.thumbnail_hash = 0;
            if smtc_pos > 0 {
                info.position_ms = smtc_pos;
            }
            info.last_smtc_pos = smtc_pos;
            info.last_update = Instant::now();
            info.last_thumbnail_fetch = Instant::now();
            should_fetch_lyrics = true;
            should_fetch_thumbnail = true;
        } else if (info.is_playing != is_playing
            && info.thumbnail.is_none()
            && !new_title.is_empty())
            || (!new_title.is_empty()
                && info.last_thumbnail_fetch.elapsed() >= Duration::from_secs(5))
        {
            info.last_thumbnail_fetch = Instant::now();
            should_fetch_thumbnail = true;
        }
        let current_extrapolated = if info.is_playing {
            info.position_ms
                .saturating_add(info.last_update.elapsed().as_millis() as u64)
        } else {
            info.position_ms
        };

        let smtc_changed = smtc_pos != info.last_smtc_pos;
        let diff_with_extrapolated = (smtc_pos as i64 - current_extrapolated as i64).abs();

        let should_sync = song_changed
            || (info.is_playing != is_playing)
            || (smtc_pos > 0 && info.position_ms == 0)
            || (smtc_changed && (diff_with_extrapolated > 2000 || !is_playing));

        if should_sync {
            if smtc_pos > 0 || !song_changed {
                info.position_ms = smtc_pos;
            }
            info.last_update = Instant::now();
        }

        let was_playing = info.is_playing;
        info.last_smtc_pos = smtc_pos;
        info.is_playing = is_playing;
        if !song_changed && was_playing != is_playing {
            log::info!(
                "SMTC: playback state -> {}",
                if is_playing { "Playing" } else { "Paused" }
            );
        }
        info.duration_secs = duration_secs;
        info.duration_ms = duration_ms_from_tl;
        let _ = info_tx.send(info);
    }

    if should_fetch_thumbnail {
        let info_tx_clone = info_tx.clone();
        let session_clone = session.clone();
        let title_clone = new_title.clone();
        let artist_clone = new_artist.clone();
        let is_song_change = should_fetch_lyrics;
        tokio::task::spawn_blocking(move || {
            if is_song_change {
                std::thread::sleep(Duration::from_millis(800));
            }
            for attempt in 0..10 {
                let res = (|| -> windows::core::Result<(String, String, Vec<u8>)> {
                    let props = session_clone.TryGetMediaPropertiesAsync()?.join()?;
                    let fetched_title = props.Title()?.to_string();
                    let fetched_artist = props.Artist()?.to_string();
                    if fetched_title != title_clone || fetched_artist != artist_clone {
                        // HRESULT(-2) is a sentinel value to signal stale media properties,
                        // not a standard COM error code. The caller retries on this error.
                        return Err(windows::core::Error::new(
                            windows::core::HRESULT(-2),
                            "Stale properties",
                        ));
                    }
                    let thumb_ref = props.Thumbnail()?;
                    let stream = thumb_ref.OpenReadAsync()?.join()?;
                    let size = stream.Size()?;
                    if size == 0 {
                        return Err(windows::core::Error::new(
                            windows::core::HRESULT(-1),
                            "Empty thumbnail",
                        ));
                    }
                    let buffer = windows::Storage::Streams::Buffer::Create(size as u32)?;
                    let res_buffer = stream
                        .ReadAsync(
                            &buffer,
                            size as u32,
                            windows::Storage::Streams::InputStreamOptions::None,
                        )?
                        .join()?;
                    let reader = windows::Storage::Streams::DataReader::FromBuffer(&res_buffer)?;
                    let mut bytes = vec![0u8; size as usize];
                    reader.ReadBytes(&mut bytes)?;
                    Ok((fetched_title, fetched_artist, bytes))
                })();

                if let Ok((_t, _a, bytes)) = res {
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    bytes.hash(&mut hasher);
                    let hash = hasher.finish();

                    let current = info_tx_clone.borrow();
                    if current.title == title_clone
                        && current.artist == artist_clone
                        && current.thumbnail_hash != hash
                    {
                        drop(current);
                        let mut new_info = info_tx_clone.borrow().clone();
                        new_info.thumbnail = Some(Arc::new(bytes.clone()));
                        new_info.thumbnail_hash = hash;
                        let _ = info_tx_clone.send(new_info);
                        log::info!(
                            "SMTC: thumbnail fetched ({} bytes, hash={:#x})",
                            bytes.len(),
                            hash
                        );
                    }
                    return;
                }
                let delay = if attempt < 3 { 300 } else { 500 };
                std::thread::sleep(Duration::from_millis(delay));
            }
            log::warn!(
                "SMTC: thumbnail fetch failed for '{}' - '{}' after 10 attempts",
                title_clone,
                artist_clone
            );
        });
    }

    if should_fetch_lyrics {
        let info_tx_clone = info_tx.clone();
        let title = new_title.clone();
        let artist = new_artist.clone();
        let src = lyrics_source.to_string();
        let fb = lyrics_fallback;
        let local_dir = local_dir.map(|s| s.to_string());
        tokio::spawn(async move {
            let lyrics = fetch_lyrics(
                &title,
                &artist,
                duration_secs,
                &src,
                fb,
                local_dir.as_deref(),
            )
            .await;
            match lyrics {
                Some(lyrics) => {
                    log::info!("SMTC: lyrics fetched ({} lines from {})", lyrics.len(), src);
                    let current = info_tx_clone.borrow();
                    if current.title == title && current.artist == artist {
                        drop(current);
                        let mut new_info = info_tx_clone.borrow().clone();
                        new_info.lyrics = Some(lyrics);
                        let _ = info_tx_clone.send(new_info);
                    }
                }
                None => {
                    log::warn!(
                        "SMTC: lyrics fetch returned none for '{}' - '{}'",
                        title,
                        artist
                    );
                }
            }
        });
    }
    Ok(())
}
