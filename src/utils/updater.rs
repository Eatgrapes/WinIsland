use crate::core::i18n::{tr, tr_args};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use windows::Win32::UI::WindowsAndMessaging::{
    IDOK, IDYES, MB_ICONINFORMATION, MB_OKCANCEL, MB_SETFOREGROUND, MB_TOPMOST, MessageBoxW,
};
use windows::core::PCWSTR;

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent(format!("WinIsland/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .unwrap()
});

/// Compare local version info against config/build — no separate timestamp struct needed.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VersionInfo {
    pub timestamp: String,
}

const GITHUB_API_NIGHTLY: &str =
    "https://api.github.com/repos/Eatgrapes/WinIsland/releases/tags/nightly";

/// Build timestamp (Unix seconds) embedded at compile time by build.rs.
static BUILD_TS: once_cell::sync::Lazy<u64> =
    once_cell::sync::Lazy::new(|| match option_env!("WINISLAND_BUILD_TS") {
        Some(s) => s.parse::<u64>().unwrap_or(0),
        None => 0,
    });

fn build_timestamp_iso() -> String {
    // Convert BUILD_TS (Unix seconds) to ISO-8601 UTC.
    // A simple approach: format manually since we avoid pulling in chrono.
    let secs = *BUILD_TS;
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let h = time_secs / 3600;
    let m = (time_secs % 3600) / 60;
    let s = time_secs % 60;

    // Days since 1970-01-01
    let mut y = 1970i64;
    let mut remaining = days as i64;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let months_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut mo = 0usize;
    let mut d = remaining;
    for (i, md) in months_days.iter().enumerate() {
        if d < *md {
            mo = i;
            break;
        }
        d -= *md;
    }
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        mo + 1,
        d + 1,
        h,
        m,
        s
    )
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn get_app_dir() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".winisland");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path
}

/// Ensure local version_info.json reflects at least the embedded build timestamp.
/// This prevents showing update prompts for the same build after manual exe replacement.
pub fn sync_local_version(app_dir: &Path) {
    if *BUILD_TS == 0 {
        return;
    }
    let local_path = app_dir.join("version_info.json");
    let build_iso = build_timestamp_iso();

    let should_update = if let Ok(content) = fs::read_to_string(&local_path) {
        if let Ok(info) = serde_json::from_str::<VersionInfo>(&content) {
            info.timestamp.as_str() < build_iso.as_str()
        } else {
            true
        }
    } else {
        true
    };

    if should_update {
        let info = VersionInfo {
            timestamp: build_iso,
        };
        if let Ok(json) = serde_json::to_string(&info) {
            let _ = fs::write(local_path, json);
        }
    }
}

pub fn start_update_checker() {
    // Sync local version first so manual replacement doesn't re-prompt
    let app_dir = get_app_dir();
    sync_local_version(&app_dir);

    tokio::spawn(async move {
        let app_dir = get_app_dir();
        let mut last_check = tokio::time::Instant::now();

        if crate::core::persistence::load_config().check_for_updates {
            log::info!("Update checker started");
            do_check(&app_dir).await;
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
                do_check(&app_dir).await;
                last_check = tokio::time::Instant::now();
            }
        }
    });
}

/// HTTP GET with retry (3 attempts, exponential backoff).
async fn http_get_with_retry(url: &str) -> Option<String> {
    let mut delay_ms = 1000u64;
    for attempt in 1..=3 {
        match HTTP_CLIENT.get(url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(s) => return Some(s),
                        Err(e) => {
                            log::warn!("Update: attempt {}/3 read failed: {}", attempt, e);
                        }
                    }
                } else {
                    log::warn!(
                        "Update: attempt {}/3 HTTP {} for {}",
                        attempt,
                        resp.status(),
                        url
                    );
                }
            }
            Err(e) => {
                log::warn!("Update: attempt {}/3 request failed: {}", attempt, e);
            }
        }
        if attempt < 3 {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            delay_ms *= 2;
        }
    }
    None
}

/// Download raw bytes with retry.
async fn download_bytes_with_retry(url: &str) -> Option<Vec<u8>> {
    let mut delay_ms = 1000u64;
    for attempt in 1..=3 {
        match HTTP_CLIENT.get(url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.bytes().await {
                        Ok(b) => return Some(b.to_vec()),
                        Err(e) => {
                            log::warn!("Update: download attempt {}/3 read failed: {}", attempt, e);
                        }
                    }
                } else {
                    log::warn!(
                        "Update: download attempt {}/3 HTTP {} for {}",
                        attempt,
                        resp.status(),
                        url
                    );
                }
            }
            Err(e) => {
                log::warn!(
                    "Update: download attempt {}/3 request failed: {}",
                    attempt,
                    e
                );
            }
        }
        if attempt < 3 {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            delay_ms *= 2;
        }
    }
    None
}

/// Resolve the actual download URL for WinIsland.exe from the latest nightly release.
/// Falls back to the hardcoded URL if the API call fails.
async fn resolve_nightly_download_url() -> String {
    let fallback = "https://github.com/Eatgrapes/WinIsland/releases/download/nightly/WinIsland.exe";

    let body = match http_get_with_retry(GITHUB_API_NIGHTLY).await {
        Some(b) => b,
        None => {
            log::warn!("Update: GitHub API unavailable, using fallback URL");
            return fallback.to_string();
        }
    };

    // Parse the release JSON to find the WinIsland.exe asset
    let json: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("Update: failed to parse release JSON: {}", e);
            return fallback.to_string();
        }
    };

    if let Some(assets) = json["assets"].as_array() {
        for asset in assets {
            if let Some(name) = asset["name"].as_str()
                && (name == "WinIsland.exe" || name == "WinIsland_x64.exe")
                && let Some(url) = asset["browser_download_url"].as_str()
            {
                log::info!("Update: resolved download URL via GitHub API");
                return url.to_string();
            }
        }
    }

    log::warn!("Update: WinIsland.exe asset not found in nightly release, using fallback");
    fallback.to_string()
}

async fn do_check(app_dir: &Path) {
    let local_json_path = app_dir.join("version_info.json");

    let remote_json_str = match http_get_with_retry(
        "https://github.com/Eatgrapes/WinIsland/releases/download/nightly/version_info.json",
    )
    .await
    {
        Some(s) => s,
        None => {
            log::warn!("Update check: failed to fetch remote version info after retries");
            return;
        }
    };

    let remote_info: VersionInfo = match serde_json::from_str(&remote_json_str) {
        Ok(info) => info,
        Err(_) => {
            log::warn!("Update check: failed to parse remote version info");
            return;
        }
    };

    let mut needs_update = false;
    if local_json_path.exists() {
        if let Ok(local_content) = fs::read_to_string(&local_json_path) {
            if let Ok(local_info) = serde_json::from_str::<VersionInfo>(&local_content) {
                if remote_info.timestamp > local_info.timestamp {
                    needs_update = true;
                } else {
                    log::info!(
                        "Update check: current version is up-to-date ({})",
                        local_info.timestamp
                    );
                }
            } else {
                log::info!("Update check: local version info corrupted, will update");
                needs_update = true;
            }
        } else {
            log::warn!("Update check: failed to read local version file");
            needs_update = true;
        }
    } else {
        log::info!("Update check: no local version info, fresh install");
        needs_update = true;
    }

    if needs_update {
        log::info!(
            "Update available: {} -> {}",
            if local_json_path.exists() {
                "current"
            } else {
                "none"
            },
            remote_info.timestamp
        );

        let title_w: Vec<u16> = format!("{}\0", tr("update_available_title"))
            .encode_utf16()
            .collect();
        let text_w: Vec<u16> = tr_args("update_available_desc", &[&remote_info.timestamp])
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
            perform_update(remote_json_str, app_dir.to_path_buf()).await;
        }
    }
}

async fn perform_update(remote_json_str: String, app_dir: PathBuf) {
    let download_url = resolve_nightly_download_url().await;
    log::info!("Update: downloading new executable from {}", download_url);

    let bytes = match download_bytes_with_retry(&download_url).await {
        Some(b) => b,
        None => {
            log::error!("Update: download failed after retries");
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
