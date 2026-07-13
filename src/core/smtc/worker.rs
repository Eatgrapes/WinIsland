use std::time::{Duration, Instant};

use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

use crate::core::lyrics::fetch_lyrics;

use super::MediaInfo;
use super::PlaybackCommand;
use super::properties::fetch_properties;
use super::session::{auto_allow_new_apps, get_target_session};

pub(super) struct WorkerChannels {
    pub(super) info_tx: watch::Sender<MediaInfo>,
    pub(super) seek_rx: mpsc::UnboundedReceiver<u64>,
    pub(super) playback_rx: mpsc::UnboundedReceiver<PlaybackCommand>,
    pub(super) lyrics_source_rx: mpsc::UnboundedReceiver<String>,
    pub(super) lyrics_fallback_rx: mpsc::UnboundedReceiver<bool>,
    pub(super) lyrics_local_dir_rx: mpsc::UnboundedReceiver<Option<String>>,
    pub(super) allowed_apps_rx: mpsc::UnboundedReceiver<Vec<String>>,
}

pub(super) fn smtc_poll_loop(channels: WorkerChannels, cancel: CancellationToken) {
    let WorkerChannels {
        info_tx,
        mut seek_rx,
        mut playback_rx,
        mut lyrics_source_rx,
        mut lyrics_fallback_rx,
        mut lyrics_local_dir_rx,
        mut allowed_apps_rx,
    } = channels;
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

    let (event_tx, event_rx) = std::sync::mpsc::channel::<()>();
    let handler = TypedEventHandler::new(move |_m, _| {
        let _ = event_tx.send(());
        Ok(())
    });
    let _ = manager.SessionsChanged(&handler);

    let mut current_lyrics_source: String = "163".to_string();
    let mut current_lyrics_fallback: bool = true;
    let mut current_lyrics_local_dir: Option<String> = None;
    let mut current_allowed_apps: Vec<String> = Vec::new();

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

        while let Ok(src) = lyrics_source_rx.try_recv() {
            if src != current_lyrics_source {
                current_lyrics_source = src;
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

        if last_regular_update.elapsed() > Duration::from_millis(300) {
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
