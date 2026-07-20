use crate::core::lyrics::fetch_lyrics;
use skia_safe::Data;
use std::time::{Duration, Instant};

use tokio::sync::watch;
use windows::Media::Control::GlobalSystemMediaTransportControlsSession;

use super::MediaInfo;
use super::session::is_music_session;

thread_local! {
    static LAST_TIMELINE_FETCH: std::cell::Cell<Option<Instant>> = const { std::cell::Cell::new(None) };
    static LAST_FETCHED_SMTC_POS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static LAST_FETCHED_DURATION_SECS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static LAST_FETCHED_DURATION_MS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

pub(super) fn fetch_properties(
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
                        let byte_len = bytes.len();
                        new_info.thumbnail = Some(Data::new_copy(&bytes));
                        new_info.thumbnail_hash = hash;
                        let _ = info_tx_clone.send(new_info);
                        log::info!(
                            "SMTC: thumbnail fetched ({} bytes, hash={:#x})",
                            byte_len,
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
