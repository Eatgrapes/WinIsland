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
            self.position_ms + self.last_update.elapsed().as_millis() as u64
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
        let (allowed_apps_tx, allowed_apps_rx) = mpsc::unbounded_channel();
        let cancel_token = CancellationToken::new();

        let _ = lyrics_source_tx.send(source);
        let _ = lyrics_fallback_tx.send(fallback);
        let _ = allowed_apps_tx.send(allowed);

        let cancel = cancel_token.clone();
        tokio::task::spawn_blocking(move || {
            smtc_poll_loop(
                info_tx,
                seek_rx,
                playback_rx,
                lyrics_source_rx,
                lyrics_fallback_rx,
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

fn smtc_poll_loop(
    info_tx: watch::Sender<MediaInfo>,
    mut seek_rx: mpsc::UnboundedReceiver<u64>,
    mut playback_rx: mpsc::UnboundedReceiver<PlaybackCommand>,
    mut lyrics_source_rx: mpsc::UnboundedReceiver<String>,
    mut lyrics_fallback_rx: mpsc::UnboundedReceiver<bool>,
    mut allowed_apps_rx: mpsc::UnboundedReceiver<Vec<String>>,
    cancel: CancellationToken,
) {
    let manager = match GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
        Ok(op) => match op.get() {
            Ok(m) => m,
            Err(_) => return,
        },
        Err(_) => return,
    };

    // COM event bridge: COM callback -> std::sync::mpsc -> polling loop
    let (event_tx, event_rx) = std::sync::mpsc::channel::<()>();
    let handler = TypedEventHandler::new(
        move |_m: &Option<GlobalSystemMediaTransportControlsSessionManager>, _| {
            let _ = event_tx.send(());
            Ok(())
        },
    );
    let _ = manager.SessionsChanged(&handler);

    // Local state mirrored from channels
    let mut current_lyrics_source: String = "163".to_string();
    let mut current_lyrics_fallback: bool = true;
    let mut current_allowed_apps: Vec<String> = Vec::new();

    // Drain initial config values from channels
    while let Ok(src) = lyrics_source_rx.try_recv() {
        current_lyrics_source = src;
    }
    while let Ok(fb) = lyrics_fallback_rx.try_recv() {
        current_lyrics_fallback = fb;
    }
    while let Ok(apps) = allowed_apps_rx.try_recv() {
        current_allowed_apps = apps;
    }

    // Initial update
    update_media_info(
        &manager,
        &info_tx,
        &current_lyrics_source,
        current_lyrics_fallback,
        &mut current_allowed_apps,
    );

    let mut last_manager_refresh = Instant::now();
    let mut current_manager = manager;

    while !cancel.is_cancelled() {
        // Refresh manager every 30 seconds
        if last_manager_refresh.elapsed() > Duration::from_secs(30) {
            if let Ok(new_mgr_op) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
                && let Ok(new_mgr) = new_mgr_op.get()
            {
                current_manager = new_mgr;
                let _ = current_manager.SessionsChanged(&handler);
            }
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
                    drop(info);
                    tokio::spawn(async move {
                        if let Some(lyrics) =
                            fetch_lyrics(&title, &artist, duration, &src, fb).await
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

        // Check COM events
        if event_rx.try_recv().is_ok() {
            update_media_info(
                &current_manager,
                &info_tx,
                &current_lyrics_source,
                current_lyrics_fallback,
                &mut current_allowed_apps,
            );
        }

        // Regular update
        update_media_info(
            &current_manager,
            &info_tx,
            &current_lyrics_source,
            current_lyrics_fallback,
            &mut current_allowed_apps,
        );

        std::thread::sleep(Duration::from_millis(300));
    }
}

fn update_media_info(
    manager: &GlobalSystemMediaTransportControlsSessionManager,
    info_tx: &watch::Sender<MediaInfo>,
    lyrics_source: &str,
    lyrics_fallback: bool,
    allowed_apps: &mut Vec<String>,
) {
    *allowed_apps = auto_allow_new_apps(manager, allowed_apps);

    if let Some(session) = get_target_session(manager, allowed_apps) {
        let _ = fetch_properties(&session, info_tx, lyrics_source, lyrics_fallback);
    } else {
        let info = info_tx.borrow();
        if !info.title.is_empty() {
            drop(info);
            let _ = info_tx.send(MediaInfo::default());
        }
    }
}

fn auto_allow_new_apps(
    mgr: &GlobalSystemMediaTransportControlsSessionManager,
    allowed: &[String],
) -> Vec<String> {
    let mut new_allowed = allowed.to_vec();
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
                let mut config = load_config();
                let mut changed = false;

                if !config.smtc_known_apps.contains(&app_id) {
                    let is_first_run = config.smtc_known_apps.is_empty();
                    config.smtc_known_apps.push(app_id.clone());

                    if is_first_run && !config.smtc_apps.contains(&app_id) {
                        config.smtc_apps.push(app_id.clone());
                        if !new_allowed.contains(&app_id) {
                            new_allowed.push(app_id);
                        }
                    }
                    changed = true;
                }

                if changed {
                    save_config(&config);
                }
            }
        }
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

fn fetch_properties(
    session: &GlobalSystemMediaTransportControlsSession,
    info_tx: &watch::Sender<MediaInfo>,
    lyrics_source: &str,
    lyrics_fallback: bool,
) -> windows::core::Result<()> {
    if !is_music_session(session) {
        let info = info_tx.borrow();
        if !info.title.is_empty() {
            drop(info);
            let _ = info_tx.send(MediaInfo::default());
        }
        return Ok(());
    }

    let props = session.TryGetMediaPropertiesAsync()?.get()?;
    let pb_info = session.GetPlaybackInfo()?;
    let is_playing = pb_info.PlaybackStatus()? == windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;

    let smtc_pos = if let Ok(tl) = session.GetTimelineProperties() {
        if let Ok(pos) = tl.Position() {
            let raw = pos.Duration;
            if raw > 0 { (raw / 10_000) as u64 } else { 0 }
        } else {
            0
        }
    } else {
        0
    };

    let duration_secs = if let Ok(tl) = session.GetTimelineProperties() {
        if let Ok(end) = tl.EndTime() {
            let raw = end.Duration;
            if raw > 0 {
                (raw / 10_000_000) as u64
            } else {
                0
            }
        } else {
            0
        }
    } else {
        0
    };

    let duration_ms_from_tl = if let Ok(tl) = session.GetTimelineProperties() {
        if let Ok(end) = tl.EndTime() {
            let raw = end.Duration;
            if raw > 0 { (raw / 10_000) as u64 } else { 0 }
        } else {
            0
        }
    } else {
        0
    };

    let new_title = props.Title()?.to_string();
    let new_artist = props.Artist()?.to_string();
    let new_album = props.AlbumTitle()?.to_string();
    let mut should_fetch_lyrics = false;
    let mut should_fetch_thumbnail = false;

    {
        let mut info = info_tx.borrow().clone();
        let song_changed =
            info.title != new_title || info.artist != new_artist || info.album != new_album;
        if song_changed {
            info.title = new_title.clone();
            info.artist = new_artist.clone();
            info.album = new_album.clone();
            info.duration_secs = duration_secs;
            info.duration_ms = duration_ms_from_tl;
            info.lyrics = None;
            info.thumbnail = None;
            info.thumbnail_hash = 0;
            info.position_ms = smtc_pos;
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
            info.position_ms + info.last_update.elapsed().as_millis() as u64
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
            info.position_ms = smtc_pos;
            info.last_update = Instant::now();
        }

        info.last_smtc_pos = smtc_pos;
        info.is_playing = is_playing;
        info.duration_secs = duration_secs;
        info.duration_ms = duration_ms_from_tl;
        let _ = info_tx.send(info);
    }

    if should_fetch_thumbnail {
        let info_tx_clone = info_tx.clone();
        let session_clone = session.clone();
        let title_clone = new_title.clone();
        let artist_clone = new_artist.clone();
        tokio::task::spawn_blocking(move || {
            for _ in 0..10 {
                let res = (|| -> windows::core::Result<Vec<u8>> {
                    let props = session_clone.TryGetMediaPropertiesAsync()?.get()?;
                    let thumb_ref = props.Thumbnail()?;
                    let stream = thumb_ref.OpenReadAsync()?.get()?;
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
                        .get()?;
                    let reader = windows::Storage::Streams::DataReader::FromBuffer(&res_buffer)?;
                    let mut bytes = vec![0u8; size as usize];
                    reader.ReadBytes(&mut bytes)?;
                    Ok(bytes)
                })();

                if let Ok(bytes) = res {
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
                        new_info.thumbnail = Some(Arc::new(bytes));
                        new_info.thumbnail_hash = hash;
                        let _ = info_tx_clone.send(new_info);
                    }
                    return;
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        });
    }

    if should_fetch_lyrics {
        let info_tx_clone = info_tx.clone();
        let title = new_title.clone();
        let artist = new_artist.clone();
        let src = lyrics_source.to_string();
        let fb = lyrics_fallback;
        tokio::spawn(async move {
            if let Some(lyrics) = fetch_lyrics(&title, &artist, duration_secs, &src, fb).await {
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
    Ok(())
}
