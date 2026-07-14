use crate::core::persistence::{load_config, save_config};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
};

pub(super) fn auto_allow_new_apps(
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

pub(super) fn get_target_session(
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

pub(super) fn is_music_session(session: &GlobalSystemMediaTransportControlsSession) -> bool {
    if let Ok(pb_info) = session.GetPlaybackInfo()
        && let Ok(playback_type) = pb_info.PlaybackType()
        && let Ok(value) = playback_type.Value()
        && value == windows::Media::MediaPlaybackType::Video
    {
        return false;
    }
    true
}
