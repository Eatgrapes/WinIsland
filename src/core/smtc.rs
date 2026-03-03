use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, Duration};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSession,
};
use windows::Foundation::TypedEventHandler;
use crate::core::lyrics::{LyricLine, fetch_lyrics};

#[derive(Clone, Debug)]
pub struct MediaInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub is_playing: bool,
    pub thumbnail: Option<Arc<Vec<u8>>>,
    pub spectrum: [f32; 6],
    pub position_ms: u64,
    pub last_update: Instant,
    pub lyrics: Option<Arc<Vec<LyricLine>>>,
    pub last_smtc_pos: u64,
    pub duration_secs: u64,
}

impl Default for MediaInfo {
    fn default() -> Self {
        Self {
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            is_playing: false,
            thumbnail: None,
            spectrum: [0.0; 6],
            position_ms: 0,
            last_update: Instant::now(),
            lyrics: None,
            last_smtc_pos: 0,
            duration_secs: 0,
        }
    }
}

impl MediaInfo {
    pub fn current_lyric(&self) -> Option<String> {
        let lyrics = self.lyrics.as_ref()?;
        if lyrics.is_empty() { return None; }

        let current_pos = if self.is_playing {
            self.position_ms + self.last_update.elapsed().as_millis() as u64
        } else {
            self.position_ms
        };
        
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

pub struct SmtcListener {
    info: Arc<Mutex<MediaInfo>>,
    active: Arc<AtomicBool>,
    lyrics_source: Arc<Mutex<String>>,
    lyrics_fallback: Arc<Mutex<bool>>,
}

impl SmtcListener {
    pub fn new(source: String, fallback: bool) -> Self {
        let listener = Self {
            info: Arc::new(Mutex::new(MediaInfo::default())),
            active: Arc::new(AtomicBool::new(true)),
            lyrics_source: Arc::new(Mutex::new(source)),
            lyrics_fallback: Arc::new(Mutex::new(fallback)),
        };
        listener.init();
        listener
    }

    pub fn set_lyrics_source(&self, source: String) {
        {
            let mut s = self.lyrics_source.lock().unwrap();
            if *s == source { return; }
            *s = source.clone();
        }

        let (title, artist, duration_secs) = {
            let mut info = self.info.lock().unwrap();
            if info.title.is_empty() { return; }
            info.lyrics = None;
            (info.title.clone(), info.artist.clone(), info.duration_secs)
        };

        let arc_clone = self.info.clone();
        let source_arc = self.lyrics_source.clone();
        let fallback_arc = self.lyrics_fallback.clone();
        std::thread::spawn(move || {
            let src = source_arc.lock().unwrap().clone();
            let fb = *fallback_arc.lock().unwrap();
            if let Some(lyrics) = fetch_lyrics(&title, &artist, duration_secs, &src, fb) {
                if let Ok(mut info) = arc_clone.lock() {
                    if info.title == title && info.artist == artist {
                        info.lyrics = Some(lyrics);
                    }
                }
            }
        });
    }

    pub fn set_lyrics_fallback(&self, fallback: bool) {
        *self.lyrics_fallback.lock().unwrap() = fallback;
    }

    pub fn get_info(&self) -> MediaInfo {
        self.info.lock().unwrap().clone()
    }

    fn init(&self) {
        let info_clone = self.info.clone();
        let active_clone = self.active.clone();
        let source_clone = self.lyrics_source.clone();
        let fallback_clone = self.lyrics_fallback.clone();
        std::thread::spawn(move || {
            let manager = match GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
                Ok(op) => match op.get() {
                    Ok(m) => m,
                    Err(_) => return,
                },
                Err(_) => return,
            };

            let update_info = |mgr: &GlobalSystemMediaTransportControlsSessionManager, arc: &Arc<Mutex<MediaInfo>>, src: &Arc<Mutex<String>>, fb: &Arc<Mutex<bool>>| {
                if let Ok(session) = mgr.GetCurrentSession() {
                    let _ = Self::fetch_properties(&session, arc, src, fb);
                } else {
                    if let Ok(mut info) = arc.lock() {
                        if !info.title.is_empty() {
                            *info = MediaInfo::default();
                        }
                    }
                }
            };

            update_info(&manager, &info_clone, &source_clone, &fallback_clone);

            let info_for_handler = info_clone.clone();
            let source_for_handler = source_clone.clone();
            let fallback_for_handler = fallback_clone.clone();
            let handler = TypedEventHandler::new(move |m: &Option<GlobalSystemMediaTransportControlsSessionManager>, _| {
                if let Some(mgr) = m {
                    let _ = update_info(mgr, &info_for_handler, &source_for_handler, &fallback_for_handler);
                }
                Ok(())
            });
            let _ = manager.SessionsChanged(&handler);

            while active_clone.load(Ordering::Relaxed) {
                if let Ok(session) = manager.GetCurrentSession() {
                    let _ = Self::fetch_properties(&session, &info_clone, &source_clone, &fallback_clone);
                }
                std::thread::sleep(Duration::from_millis(300));
            }
        });
    }

    fn fetch_properties(session: &GlobalSystemMediaTransportControlsSession, info_arc: &Arc<Mutex<MediaInfo>>, source: &Arc<Mutex<String>>, fallback: &Arc<Mutex<bool>>) -> windows::core::Result<()> {
        let props = session.TryGetMediaPropertiesAsync()?.get()?;
        let pb_info = session.GetPlaybackInfo()?;
        let is_playing = pb_info.PlaybackStatus()? == windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;

        let smtc_pos = if let Ok(tl) = session.GetTimelineProperties() {
            if let Ok(pos) = tl.Position() {
                (pos.Duration / 10000) as u64
            } else { 0 }
        } else { 0 };

        let duration_secs = if let Ok(tl) = session.GetTimelineProperties() {
            if let Ok(end) = tl.EndTime() { (end.Duration / 10_000_000) as u64 } else { 0 }
        } else { 0 };

        let new_title = props.Title()?.to_string();
        let new_artist = props.Artist()?.to_string();
        let mut should_fetch_lyrics = false;
        let mut should_fetch_thumbnail = false;

        if let Ok(mut info) = info_arc.lock() {
            let song_changed = info.title != new_title || info.artist != new_artist;
            if song_changed {
                info.title = new_title.clone();
                info.artist = new_artist.clone();
                info.lyrics = None;
                info.thumbnail = None;
                info.position_ms = smtc_pos;
                info.last_smtc_pos = smtc_pos;
                info.last_update = Instant::now();
                should_fetch_lyrics = true;
                should_fetch_thumbnail = true;
            }

            info.album = props.AlbumTitle()?.to_string();
            info.duration_secs = duration_secs;
            
            let current_extrapolated = if info.is_playing {
                info.position_ms + info.last_update.elapsed().as_millis() as u64
            } else {
                info.position_ms
            };

            let smtc_changed = smtc_pos != info.last_smtc_pos;
            let diff_with_extrapolated = (smtc_pos as i64 - current_extrapolated as i64).abs();

            let should_sync = if song_changed {
                true
            } else if info.is_playing != is_playing {
                true
            } else if smtc_changed && diff_with_extrapolated > 2000 {
                true
            } else if smtc_pos > 0 && info.position_ms == 0 && is_playing {
                true
            } else {
                false
            };

            if should_sync {
                info.position_ms = smtc_pos;
                info.last_update = Instant::now();
            }
            
            info.last_smtc_pos = smtc_pos;
            info.is_playing = is_playing;
        }

        if should_fetch_thumbnail {
            if let Ok(thumb_ref) = props.Thumbnail() {
                if let Ok(stream_async) = thumb_ref.OpenReadAsync() {
                    if let Ok(stream) = stream_async.get() {
                        if let Ok(size) = stream.Size() {
                            let buffer = windows::Storage::Streams::Buffer::Create(size as u32).unwrap();
                            if let Ok(res_buffer_async) = stream.ReadAsync(&buffer, size as u32, windows::Storage::Streams::InputStreamOptions::None) {
                                if let Ok(res_buffer) = res_buffer_async.get() {
                                    if let Ok(reader) = windows::Storage::Streams::DataReader::FromBuffer(&res_buffer) {
                                        let mut bytes = vec![0u8; size as usize];
                                        let _ = reader.ReadBytes(&mut bytes);
                                        if let Ok(mut info) = info_arc.lock() {
                                            info.thumbnail = Some(Arc::new(bytes));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if should_fetch_lyrics {
            let arc_clone = info_arc.clone();
            let source_arc_clone = source.clone();
            let fallback_arc_clone = fallback.clone();
            std::thread::spawn(move || {
                let src = source_arc_clone.lock().unwrap().clone();
                let fb = *fallback_arc_clone.lock().unwrap();
                if let Some(lyrics) = fetch_lyrics(&new_title, &new_artist, duration_secs, &src, fb) {
                    if let Ok(mut info) = arc_clone.lock() {
                        if info.title == new_title && info.artist == new_artist {
                            info.lyrics = Some(lyrics);
                        }
                    }
                }
            });
        }
        Ok(())
    }
}
