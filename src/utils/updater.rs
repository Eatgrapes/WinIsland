use crate::core::i18n::tr;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;
use windows::Win32::UI::WindowsAndMessaging::{
    IDOK, IDYES, MB_ICONINFORMATION, MB_OKCANCEL, MB_SETFOREGROUND, MB_TOPMOST, MessageBoxW,
};
use windows::core::PCWSTR;

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("WinIsland-Updater")
        .build()
        .unwrap()
});

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VersionInfo {
    pub version: Option<String>,
    pub timestamp: Option<String>,
}

const UPDATE_URL_JSON: &str =
    "https://github.com/Eatgrapes/WinIsland/releases/download/nightly/version_info.json";
const UPDATE_URL_EXE: &str =
    "https://github.com/Eatgrapes/WinIsland/releases/download/nightly/WinIsland.exe";

fn is_version_newer(current: &str, remote: &str) -> bool {
    let current_parts: Vec<&str> = current.split('.').collect();
    let remote_parts: Vec<&str> = remote.split('.').collect();

    for i in 0..std::cmp::max(current_parts.len(), remote_parts.len()) {
        let current_val = current_parts
            .get(i)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        let remote_val = remote_parts
            .get(i)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        if remote_val > current_val {
            return true;
        } else if remote_val < current_val {
            return false;
        }
    }
    false
}

pub fn get_app_dir() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".winisland");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path
}

pub fn start_update_checker() {
    tokio::spawn(async move {
        let app_dir = get_app_dir();
        let mut last_check = tokio::time::Instant::now();

        // Initial check
        if crate::core::persistence::load_config().check_for_updates {
            log::info!("Update checker started");
            do_check(&app_dir, false).await;
        } else {
            log::info!("Update checker: disabled in config");
        }

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            let config = crate::core::persistence::load_config();
            if !config.check_for_updates {
                continue;
            }

            let interval_secs = config.update_check_interval * 3600.0;
            if last_check.elapsed().as_secs_f32() >= interval_secs {
                do_check(&app_dir, false).await;
                last_check = tokio::time::Instant::now();
            }
        }
    });
}

pub fn check_updates_manually() {
    std::thread::spawn(|| {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(r) => r,
            Err(e) => {
                log::error!("Failed to build runtime for manual update check: {:?}", e);
                return;
            }
        };
        rt.block_on(async {
            let app_dir = get_app_dir();
            do_check(&app_dir, true).await;
        });
    });
}

async fn do_check(app_dir: &Path, manual: bool) {
    let config = crate::core::persistence::load_config();
    let channel = config.update_channel.as_str();

    if channel == "beta" {
        do_beta_check(app_dir, manual).await;
    } else {
        do_stable_check(app_dir, manual).await;
    }
}

async fn do_beta_check(app_dir: &Path, manual: bool) {
    let remote_json_str = match HTTP_CLIENT.get(UPDATE_URL_JSON).send().await {
        Ok(resp) => match resp.text().await {
            Ok(s) => s,
            Err(_) => {
                log::warn!("Update check (Beta): failed to read remote version info");
                if manual {
                    show_error_box(tr("update_failed_title"), tr("update_failed_desc")).await;
                }
                return;
            }
        },
        Err(_) => {
            log::warn!("Update check (Beta): HTTP request failed for version_info.json");
            if manual {
                show_error_box(tr("update_failed_title"), tr("update_failed_desc")).await;
            }
            return;
        }
    };

    let remote_info: VersionInfo = match serde_json::from_str(&remote_json_str) {
        Ok(info) => info,
        Err(_) => {
            log::warn!("Update check (Beta): failed to parse remote version info");
            if manual {
                show_error_box(tr("update_failed_title"), tr("update_failed_desc")).await;
            }
            return;
        }
    };

    let remote_timestamp = match &remote_info.timestamp {
        Some(t) => t,
        None => {
            log::warn!("Update check (Beta): remote version info does not contain timestamp");
            if manual {
                show_error_box(tr("update_failed_title"), tr("update_failed_desc")).await;
            }
            return;
        }
    };

    let needs_update = remote_timestamp.as_str() > env!("BUILD_TIMESTAMP");

    if needs_update {
        log::info!(
            "Update available (Beta): {} -> {}",
            env!("BUILD_TIMESTAMP"),
            remote_timestamp
        );
        let channel_name = tr("channel_beta");
        let title_w: Vec<u16> = format!("{} ({})\0", tr("update_available_title"), channel_name)
            .encode_utf16()
            .collect();
        let text_w: Vec<u16> = tr("update_available_desc")
            .replace("{}", remote_timestamp)
            .add_null()
            .encode_utf16()
            .collect();

        let result = tokio::task::spawn_blocking(move || unsafe {
            MessageBoxW(
                None,
                PCWSTR(text_w.as_ptr()),
                PCWSTR(title_w.as_ptr()),
                MB_OKCANCEL | MB_ICONINFORMATION | MB_TOPMOST | MB_SETFOREGROUND,
            )
        })
        .await;

        if let Ok(r) = result
            && (r == IDOK || r == IDYES)
        {
            perform_update(UPDATE_URL_EXE, remote_json_str, app_dir.to_path_buf()).await;
        }
    } else {
        log::info!(
            "Update check (Beta): current version is up-to-date (local: {}, remote: {})",
            env!("BUILD_TIMESTAMP"),
            remote_timestamp
        );
        if manual {
            show_error_box(tr("update_no_update_title"), tr("update_no_update_desc")).await;
        }
    }
}

async fn do_stable_check(app_dir: &Path, manual: bool) {
    let resp = match HTTP_CLIENT
        .get("https://github.com/Eatgrapes/WinIsland/releases/latest")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            log::warn!(
                "Update check (Stable): HTTP request failed for latest release: {:?}",
                e
            );
            if manual {
                show_error_box(tr("update_failed_title"), tr("update_failed_desc")).await;
            }
            return;
        }
    };

    let path = resp.url().path();
    let tag_name = if path.contains("/tag/") {
        path.split("/tag/").last().map(|tag| tag.to_string())
    } else {
        None
    };

    let tag_name = match tag_name {
        Some(t) => t,
        None => {
            log::warn!(
                "Update check (Stable): failed to extract latest release tag from URL path '{}'",
                path
            );
            if manual {
                show_error_box(tr("update_failed_title"), tr("update_failed_desc")).await;
            }
            return;
        }
    };

    let remote_version = tag_name.trim_start_matches('v').trim_start_matches('V');
    let needs_update = is_version_newer(crate::core::config::APP_VERSION, remote_version);

    if needs_update {
        log::info!(
            "Update available (Stable): {} -> {}",
            crate::core::config::APP_VERSION,
            remote_version
        );

        let download_url = format!(
            "https://github.com/Eatgrapes/WinIsland/releases/download/{}/WinIsland.exe",
            tag_name
        );

        let channel_name = tr("channel_stable");
        let title_w: Vec<u16> = format!("{} ({})\0", tr("update_available_title"), channel_name)
            .encode_utf16()
            .collect();
        let text_w: Vec<u16> = tr("update_available_desc")
            .replace("{}", remote_version)
            .add_null()
            .encode_utf16()
            .collect();

        let result = tokio::task::spawn_blocking(move || unsafe {
            MessageBoxW(
                None,
                PCWSTR(text_w.as_ptr()),
                PCWSTR(title_w.as_ptr()),
                MB_OKCANCEL | MB_ICONINFORMATION | MB_TOPMOST | MB_SETFOREGROUND,
            )
        })
        .await;

        if let Ok(r) = result
            && (r == IDOK || r == IDYES)
        {
            let local_version_info = VersionInfo {
                version: Some(remote_version.to_string()),
                timestamp: None,
            };
            let serialized = serde_json::to_string(&local_version_info).unwrap_or_default();
            perform_update(&download_url, serialized, app_dir.to_path_buf()).await;
        }
    } else {
        log::info!(
            "Update check (Stable): current version is up-to-date ({})",
            crate::core::config::APP_VERSION
        );
        if manual {
            show_error_box(tr("update_no_update_title"), tr("update_no_update_desc")).await;
        }
    }
}

async fn perform_update(download_url: &str, remote_json_str: String, app_dir: PathBuf) {
    log::info!("Update: downloading new executable from {}", download_url);
    let resp = match HTTP_CLIENT.get(download_url).send().await {
        Ok(r) => r,
        Err(_) => {
            log::error!("Update: download request failed");
            show_error_box(tr("update_failed_title"), tr("update_failed_dl")).await;
            return;
        }
    };

    if !resp.status().is_success() {
        log::error!("Update: download failed with status {}", resp.status());
        show_error_box(tr("update_failed_title"), tr("update_failed_dl")).await;
        return;
    }

    let bytes = match resp.bytes().await {
        Ok(b) => b.to_vec(),
        Err(_) => {
            log::error!("Update: download failed (read response)");
            show_error_box(tr("update_failed_title"), tr("update_failed_dl")).await;
            return;
        }
    };
    log::info!("Update: downloaded {} bytes", bytes.len());

    let current_exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => {
            log::error!("Update: failed to get current exe path");
            show_error_box(tr("update_failed_title"), tr("update_failed_save")).await;
            return;
        }
    };
    let new_exe_path = current_exe.with_extension("exe.new");

    if fs::write(&new_exe_path, &bytes).is_err() {
        log::error!(
            "Update: failed to write new exe to {}",
            new_exe_path.display()
        );
        show_error_box(tr("update_failed_title"), tr("update_failed_save")).await;
        return;
    }

    let local_json_path = app_dir.join("version_info.json");
    let _ = fs::write(local_json_path, remote_json_str);
    log::info!(
        "Update: new exe written to {}, spawning installer",
        new_exe_path.display()
    );

    let current_exe_str = current_exe.to_string_lossy().into_owned();
    let new_exe_str = new_exe_path.to_string_lossy().into_owned();

    // Escape single quotes for PowerShell: '' -> ''
    let ps_escape = |s: &str| s.replace('\'', "''");

    let pid = std::process::id();
    let script = format!(
        "Start-Sleep -Seconds 1; \
         while (Get-Process -Id {} -ErrorAction SilentlyContinue) {{ Start-Sleep -Milliseconds 100 }}; \
         Move-Item -Path '{}' -Destination '{}' -Force; \
         Start-Process -FilePath '{}'",
        pid,
        ps_escape(&new_exe_str),
        ps_escape(&current_exe_str),
        ps_escape(&current_exe_str)
    );

    let _ = Command::new("powershell")
        .args(["-WindowStyle", "Hidden", "-Command", &script])
        .spawn();

    std::process::exit(0);
}

async fn show_error_box(title: String, text: String) {
    let title_w: Vec<u16> = title.add_null().encode_utf16().collect();
    let text_w: Vec<u16> = text.add_null().encode_utf16().collect();
    // SAFETY: MessageBoxW displays a modal error dialog with the provided
    // null-terminated UTF-16 strings. All pointers are valid for the call duration.
    tokio::task::spawn_blocking(move || unsafe {
        MessageBoxW(
            None,
            PCWSTR(text_w.as_ptr()),
            PCWSTR(title_w.as_ptr()),
            MB_ICONINFORMATION | MB_TOPMOST,
        );
    })
    .await
    .ok();
}

trait AddNull {
    fn add_null(&self) -> String;
}
impl AddNull for String {
    fn add_null(&self) -> String {
        format!("{}\0", self)
    }
}
