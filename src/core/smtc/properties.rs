use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use skia_safe::Data;
use tokio::sync::watch;
use windows::Media::Control::GlobalSystemMediaTransportControlsSession;
use windows::Storage::Streams::DataReader;

use crate::core::lyrics::LyricsMode;
use crate::utils::cover::compress_smtc_thumbnail;

use super::session::is_music_session;
use super::{LyricsFetchRequest, MediaInfo, WinRtGuard, spawn_lyrics_fetch};

const MAX_THUMBNAIL_BYTES: u64 = 16 * 1024 * 1024;
const MAX_THUMBNAIL_STREAM_BYTES: u64 = 64 * 1024 * 1024;
const THUMBNAIL_STALE: windows::core::HRESULT = windows::core::HRESULT(-2);
const THUMBNAIL_TOO_LARGE: windows::core::HRESULT = windows::core::HRESULT(-3);
const THUMBNAIL_COMPRESSION_FAILED: windows::core::HRESULT = windows::core::HRESULT(-4);

#[derive(Clone)]
struct ThumbnailFetchRequest {
    session: GlobalSystemMediaTransportControlsSession,
    title: String,
    artist: String,
    album: String,
    is_song_change: bool,
}

pub(super) struct ThumbnailFetcher {
    state: Arc<ThumbnailWorkerState>,
}

struct ThumbnailWorkerState {
    request: Mutex<Option<ThumbnailFetchRequest>>,
    changed: Condvar,
    closed: std::sync::atomic::AtomicBool,
}

impl ThumbnailFetcher {
    pub(super) fn new(info_tx: watch::Sender<MediaInfo>) -> Option<Self> {
        let state = Arc::new(ThumbnailWorkerState {
            request: Mutex::new(None),
            changed: Condvar::new(),
            closed: std::sync::atomic::AtomicBool::new(false),
        });
        let worker_state = Arc::clone(&state);
        let spawn_result = std::thread::Builder::new()
            .name("winisland-smtc-thumbnail".to_string())
            .spawn(move || {
                let _winrt_guard = match WinRtGuard::new() {
                    Ok(guard) => guard,
                    Err(error) => {
                        log::warn!("SMTC: failed to initialize thumbnail worker: {error}");
                        return;
                    }
                };
                loop {
                    let mut pending = worker_state
                        .request
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    while pending.is_none()
                        && !worker_state
                            .closed
                            .load(std::sync::atomic::Ordering::Acquire)
                    {
                        pending = worker_state
                            .changed
                            .wait(pending)
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                    }
                    if worker_state
                        .closed
                        .load(std::sync::atomic::Ordering::Acquire)
                    {
                        return;
                    }
                    let request = pending.take();
                    drop(pending);
                    if let Some(request) = request {
                        fetch_thumbnail(request, &info_tx);
                    }
                }
            });
        match spawn_result {
            Ok(_) => Some(Self { state }),
            Err(error) => {
                log::warn!("SMTC: failed to start thumbnail worker: {error}");
                None
            }
        }
    }

    fn request(
        &self,
        session: &GlobalSystemMediaTransportControlsSession,
        title: String,
        artist: String,
        album: String,
        is_song_change: bool,
    ) {
        let mut request = self
            .state
            .request
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *request = Some(ThumbnailFetchRequest {
            session: session.clone(),
            title,
            artist,
            album,
            is_song_change,
        });
        drop(request);
        self.state.changed.notify_one();
    }
}

impl Drop for ThumbnailFetcher {
    fn drop(&mut self) {
        self.state
            .closed
            .store(true, std::sync::atomic::Ordering::Release);
        self.state.changed.notify_one();
    }
}

thread_local! {
    static LAST_TIMELINE_FETCH: std::cell::Cell<Option<Instant>> = const { std::cell::Cell::new(None) };
    static LAST_FETCHED_SMTC_POS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static LAST_FETCHED_DURATION_SECS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static LAST_FETCHED_DURATION_MS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

pub(super) fn fetch_properties(
    session: &GlobalSystemMediaTransportControlsSession,
    info_tx: &watch::Sender<MediaInfo>,
    lyrics_mode: LyricsMode,
    lyrics_source: &str,
    local_dir: Option<&str>,
    thumbnail_fetcher: Option<&ThumbnailFetcher>,
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
    let source_app_id = session
        .SourceAppUserModelId()
        .map(|id| id.to_string())
        .unwrap_or_default();

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

    let (smtc_pos, duration_secs, duration_ms_from_tl) = read_timeline(session, should_fetch);

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
            info.lyrics_fetch_id = info.lyrics_fetch_id.wrapping_add(1);
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
        } else if info.thumbnail.is_none()
            && !new_title.is_empty()
            && (info.is_playing != is_playing
                || info.last_thumbnail_fetch.elapsed() >= Duration::from_secs(5))
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

        let mut seek_guard_active = !song_changed
            && info
                .seek_guard_until
                .is_some_and(|until| Instant::now() < until);
        if seek_guard_active
            && smtc_pos > 0
            && (smtc_pos as i64 - info.seek_target_ms as i64).abs() < 1500
        {
            info.seek_guard_until = None;
            seek_guard_active = false;
        }

        let should_sync = song_changed
            || (info.is_playing != is_playing)
            || (smtc_pos > 0 && info.position_ms == 0)
            || (smtc_changed && (diff_with_extrapolated > 2000 || !is_playing));

        if should_sync && !seek_guard_active {
            if smtc_pos > 0 || !song_changed {
                info.position_ms = smtc_pos;
            }
            info.last_update = Instant::now();
        }

        let was_playing = info.is_playing;
        info.last_smtc_pos = smtc_pos;
        info.is_playing = is_playing;
        info.source_app_id = source_app_id;
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

    if should_fetch_thumbnail && let Some(thumbnail_fetcher) = thumbnail_fetcher {
        thumbnail_fetcher.request(
            session,
            new_title.clone(),
            new_artist.clone(),
            new_album.clone(),
            should_fetch_lyrics,
        );
    }

    if should_fetch_lyrics {
        let request_id = info_tx.borrow().lyrics_fetch_id;
        spawn_lyrics_fetch(
            info_tx,
            LyricsFetchRequest {
                title: new_title.clone(),
                artist: new_artist.clone(),
                duration_secs,
                mode: lyrics_mode,
                source: lyrics_source.to_string(),
                local_dir: local_dir.map(str::to_string),
                request_id,
            },
        );
    }
    Ok(())
}

fn read_timeline(
    session: &GlobalSystemMediaTransportControlsSession,
    should_fetch: bool,
) -> (u64, u64, u64) {
    if !should_fetch {
        return (
            LAST_FETCHED_SMTC_POS.with(|cell| cell.get()),
            LAST_FETCHED_DURATION_SECS.with(|cell| cell.get()),
            LAST_FETCHED_DURATION_MS.with(|cell| cell.get()),
        );
    }

    let Ok(tl) = session.GetTimelineProperties() else {
        LAST_TIMELINE_FETCH.with(|cell| cell.set(Some(Instant::now())));
        return (0, 0, 0);
    };

    let smtc_pos = tl
        .Position()
        .ok()
        .map(|pos| {
            if pos.Duration > 0 {
                (pos.Duration / 10_000) as u64
            } else {
                0
            }
        })
        .unwrap_or(0);
    let (duration_secs, duration_ms) = tl
        .EndTime()
        .ok()
        .map(|end| {
            if end.Duration > 0 {
                (
                    (end.Duration / 10_000_000) as u64,
                    (end.Duration / 10_000) as u64,
                )
            } else {
                (0, 0)
            }
        })
        .unwrap_or((0, 0));

    LAST_TIMELINE_FETCH.with(|cell| cell.set(Some(Instant::now())));
    LAST_FETCHED_SMTC_POS.with(|cell| cell.set(smtc_pos));
    LAST_FETCHED_DURATION_SECS.with(|cell| cell.set(duration_secs));
    LAST_FETCHED_DURATION_MS.with(|cell| cell.set(duration_ms));
    (smtc_pos, duration_secs, duration_ms)
}

fn fetch_thumbnail(request: ThumbnailFetchRequest, info_tx: &watch::Sender<MediaInfo>) {
    if request.is_song_change {
        std::thread::sleep(Duration::from_millis(800));
    }
    for attempt in 0..10 {
        if !thumbnail_request_is_current(info_tx, &request) {
            return;
        }
        let res = (|| -> windows::core::Result<Vec<u8>> {
            let props = request.session.TryGetMediaPropertiesAsync()?.join()?;
            let fetched_title = props.Title()?.to_string();
            let fetched_artist = props.Artist()?.to_string();
            let fetched_album = props.AlbumTitle()?.to_string();
            if fetched_title != request.title
                || fetched_artist != request.artist
                || fetched_album != request.album
            {
                return Err(windows::core::Error::new(
                    THUMBNAIL_STALE,
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
            if size > MAX_THUMBNAIL_STREAM_BYTES {
                return Err(windows::core::Error::new(
                    THUMBNAIL_TOO_LARGE,
                    "Thumbnail exceeds 64 MiB",
                ));
            }
            if size > MAX_THUMBNAIL_BYTES {
                return compress_smtc_thumbnail(&stream, size).ok_or_else(|| {
                    windows::core::Error::new(
                        THUMBNAIL_COMPRESSION_FAILED,
                        "Failed to compress oversized thumbnail",
                    )
                });
            }
            let size = size as u32;
            let buffer = windows::Storage::Streams::Buffer::Create(size)?;
            let res_buffer = stream
                .ReadAsync(
                    &buffer,
                    size,
                    windows::Storage::Streams::InputStreamOptions::None,
                )?
                .join()?;
            let reader = DataReader::FromBuffer(&res_buffer)?;
            let actual_size = reader.UnconsumedBufferLength()?;
            if actual_size == 0 || actual_size > size {
                return Err(windows::core::Error::new(
                    windows::core::HRESULT(-1),
                    "Invalid thumbnail length",
                ));
            }
            let mut bytes = vec![0u8; actual_size as usize];
            reader.ReadBytes(&mut bytes)?;
            Ok(bytes)
        })();

        if let Ok(bytes) = res {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            let hash = hasher.finish();

            let current = info_tx.borrow();
            if current.title == request.title
                && current.artist == request.artist
                && current.album == request.album
                && current.thumbnail_hash != hash
            {
                drop(current);
                let mut new_info = info_tx.borrow().clone();
                let byte_len = bytes.len();
                new_info.thumbnail = Some(Data::new_copy(&bytes));
                new_info.thumbnail_hash = hash;
                let _ = info_tx.send(new_info);
                log::info!(
                    "SMTC: thumbnail fetched ({} bytes, hash={:#x})",
                    byte_len,
                    hash
                );
            }
            return;
        }
        if res
            .as_ref()
            .is_err_and(|error| error.code() == THUMBNAIL_STALE)
        {
            return;
        }
        if res.as_ref().is_err_and(|error| {
            matches!(
                error.code(),
                THUMBNAIL_TOO_LARGE | THUMBNAIL_COMPRESSION_FAILED
            )
        }) {
            log::warn!(
                "SMTC: could not safely load oversized thumbnail for '{}' - '{}'",
                request.title,
                request.artist
            );
            return;
        }
        let delay = if attempt < 3 { 300 } else { 500 };
        std::thread::sleep(Duration::from_millis(delay));
    }
    log::warn!(
        "SMTC: thumbnail fetch failed for '{}' - '{}' after 10 attempts",
        request.title,
        request.artist
    );
}

fn thumbnail_request_is_current(
    info_tx: &watch::Sender<MediaInfo>,
    request: &ThumbnailFetchRequest,
) -> bool {
    let current = info_tx.borrow();
    current.title == request.title
        && current.artist == request.artist
        && current.album == request.album
        && current.thumbnail.is_none()
}
