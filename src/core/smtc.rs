mod properties;
mod session;
mod worker;

use crate::core::lyrics::LyricLine;
use crate::core::persistence::load_config;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
pub struct MediaInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub source_app_id: String,
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
            source_app_id: String::new(),
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

    pub fn current_lyric(&self, delay_ms: i64) -> Option<&str> {
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
            Ok(idx) => Some(&lyrics[idx].text),
            Err(idx) => {
                if idx > 0 {
                    Some(&lyrics[idx - 1].text)
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(super) enum PlaybackCommand {
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
            worker::smtc_poll_loop(
                worker::WorkerChannels {
                    info_tx,
                    seek_rx,
                    playback_rx,
                    lyrics_source_rx,
                    lyrics_fallback_rx,
                    lyrics_local_dir_rx,
                    allowed_apps_rx,
                },
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
