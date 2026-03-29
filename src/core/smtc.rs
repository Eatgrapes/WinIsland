use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, Duration};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSession,
};
use windows::Foundation::TypedEventHandler;
use crate::core::persistence::{load_config, save_config};
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
    allowed_apps: Arc<Mutex<Vec<String>>>,
}

impl SmtcListener {
    pub fn new(source: String, fallback: bool, allowed: Vec<String>) -> Self {
        let listener = Self {
            info: Arc::new(Mutex::new(MediaInfo::default())),
            active: Arc::new(AtomicBool::new(true)),
            lyrics_source: Arc::new(Mutex::new(source)),
            lyrics_fallback: Arc::new(Mutex::new(fallback)),
            allowed_apps: Arc::new(Mutex::new(allowed)),
        };
        listener.init();
        listener
    }

    pub fn set_allowed_apps(&self, apps: Vec<String>) {
        *self.allowed_apps.lock().unwrap() = apps;
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

    fn auto_allow_new_apps(mgr: &GlobalSystemMediaTransportControlsSessionManager, allowed: &Arc<Mutex<Vec<String>>>) {
        if let Ok(sessions) = mgr.GetSessions() {
            if let Ok(count) = sessions.Size() {
                for i in 0..count {
                    if let Ok(session) = sessions.GetAt(i) {
                        if let Ok(pb_info) = session.GetPlaybackInfo() {
                            if let Ok(playback_type) = pb_info.PlaybackType() {
                                if let Ok(value) = playback_type.Value() {
                                    if value == windows::Media::MediaPlaybackType::Music {
                                        if let Ok(id) = session.SourceAppUserModelId() {
                                            let app_id = id.to_string();
                                            let mut apps = allowed.lock().unwrap();
                                            if !apps.contains(&app_id) {
                                                apps.push(app_id);
                                                let mut config = load_config();
                                                config.smtc_apps = apps.clone();
                                                save_config(&config);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn get_target_session(mgr: &GlobalSystemMediaTransportControlsSessionManager, allowed: &[String]) -> Option<GlobalSystemMediaTransportControlsSession> {
        if allowed.is_empty() {
            return None;
        }
        let mut audio_session = None;
        if let Ok(sessions) = mgr.GetSessions() {
            if let Ok(count) = sessions.Size() {
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
                        if let Ok(pb_info) = session.GetPlaybackInfo() {
                            if let Ok(playback_type) = pb_info.PlaybackType() {
                                if let Ok(value) = playback_type.Value() {
                                    if value == windows::Media::MediaPlaybackType::Music {
                                        if let Ok(status) = pb_info.PlaybackStatus() {
                                            if status == windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing {
                                                return Some(session);
                                            }
                                        }
                                        if audio_session.is_none() {
                                            audio_session = Some(session);
                                        }
                                    }
                                }
                            }
                        }
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
            if let Ok(pb_info) = session.GetPlaybackInfo() {
                if let Ok(playback_type) = pb_info.PlaybackType() {
                    if let Ok(value) = playback_type.Value() {
                        if value == windows::Media::MediaPlaybackType::Video {
                            return None;
                        }
                    }
                }
            }
            return Some(session);
        }
        None
    }

    fn init(&self) {
        let info_clone = self.info.clone();
        let active_clone = self.active.clone();
        let source_clone = self.lyrics_source.clone();
        let fallback_clone = self.lyrics_fallback.clone();
        let allowed_clone = self.allowed_apps.clone();
        std::thread::spawn(move || {
            let manager = match GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
                Ok(op) => match op.get() {
                    Ok(m) => m,
                    Err(_) => return,
                },
                Err(_) => return,
            };

            let update_info = |mgr: &GlobalSystemMediaTransportControlsSessionManager, arc: &Arc<Mutex<MediaInfo>>, src: &Arc<Mutex<String>>, fb: &Arc<Mutex<bool>>, allowed: &Arc<Mutex<Vec<String>>>| {
                Self::auto_allow_new_apps(mgr, allowed);
                let apps = allowed.lock().unwrap().clone();
                if let Some(session) = Self::get_target_session(mgr, &apps) {
                    let _ = Self::fetch_properties(&session, arc, src, fb);
                } else {
                    if let Ok(mut info) = arc.lock() {
                        if !info.title.is_empty() {
                            *info = MediaInfo::default();
                        }
                    }
                }
            };

            update_info(&manager, &info_clone, &source_clone, &fallback_clone, &allowed_clone);

            let info_for_handler = info_clone.clone();
            let source_for_handler = source_clone.clone();
            let fallback_for_handler = fallback_clone.clone();
            let allowed_for_handler = allowed_clone.clone();
            let handler = TypedEventHandler::new(move |m: &Option<GlobalSystemMediaTransportControlsSessionManager>, _| {
                if let Some(mgr) = m {
                    let _ = update_info(mgr, &info_for_handler, &source_for_handler, &fallback_for_handler, &allowed_for_handler);
                }
                Ok(())
            });
            let _ = manager.SessionsChanged(&handler);

            let mut last_manager_refresh = Instant::now();
            let mut current_manager = manager;

            while active_clone.load(Ordering::Relaxed) {
                if last_manager_refresh.elapsed() > Duration::from_secs(30) {
                    if let Ok(new_mgr_op) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
                        if let Ok(new_mgr) = new_mgr_op.get() {
                            current_manager = new_mgr;
                            let _ = current_manager.SessionsChanged(&handler);
                        }
                    }
                    last_manager_refresh = Instant::now();
                }

                update_info(&current_manager, &info_clone, &source_clone, &fallback_clone, &allowed_clone);
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
        let new_album = props.AlbumTitle()?.to_string();
        let mut should_fetch_lyrics = false;
        let mut should_fetch_thumbnail = false;

        if let Ok(mut info) = info_arc.lock() {
            let song_changed = info.title != new_title || info.artist != new_artist || info.album != new_album;
            if song_changed {
                info.title = new_title.clone();
                info.artist = new_artist.clone();
                info.album = new_album.clone();
                info.duration_secs = duration_secs;
                info.lyrics = None;
                info.thumbnail = None;
                info.position_ms = smtc_pos;
                info.last_smtc_pos = smtc_pos;
                info.last_update = Instant::now();
                should_fetch_lyrics = true;
                should_fetch_thumbnail = true;
            } else if info.is_playing != is_playing && info.thumbnail.is_none() && !new_title.is_empty() {
                should_fetch_thumbnail = true;
            }
            
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
            info.duration_secs = duration_secs;
        }

        if should_fetch_thumbnail {
            let arc_clone = info_arc.clone();
            let session_clone = session.clone();
            let title_clone = new_title.clone();
            let artist_clone = new_artist.clone();
            std::thread::spawn(move || {
                for _ in 0..10 {
                    let res = (|| -> windows::core::Result<Vec<u8>> {
                        let props = session_clone.TryGetMediaPropertiesAsync()?.get()?;
                        let thumb_ref = props.Thumbnail()?;
                        let stream = thumb_ref.OpenReadAsync()?.get()?;
                        let size = stream.Size()?;
                        if size == 0 { 
                            return Err(windows::core::Error::new(windows::core::HRESULT(-1), "Empty thumbnail")); 
                        }
                        let buffer = windows::Storage::Streams::Buffer::Create(size as u32)?;
                        let res_buffer = stream.ReadAsync(&buffer, size as u32, windows::Storage::Streams::InputStreamOptions::None)?.get()?;
                        let reader = windows::Storage::Streams::DataReader::FromBuffer(&res_buffer)?;
                        let mut bytes = vec![0u8; size as usize];
                        reader.ReadBytes(&mut bytes)?;
                        Ok(bytes)
                    })();

                    if let Ok(bytes) = res {
                        if let Ok(mut info) = arc_clone.lock() {
                            if info.title == title_clone && info.artist == artist_clone {
                                info.thumbnail = Some(Arc::new(bytes));
                                return;
                            }
                        }
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(500));
                }
            });
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
