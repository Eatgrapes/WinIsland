use crate::core::i18n::tr;
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
        .build()
        .unwrap()
});

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VersionInfo {
    pub timestamp: String,
}

const UPDATE_URL_JSON: &str =
    "https://github.com/Eatgrapes/WinIsland/releases/download/nightly/version_info.json";
const UPDATE_URL_EXE: &str =
    "https://github.com/Eatgrapes/WinIsland/releases/download/nightly/WinIsland.exe";

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
            do_check(&app_dir).await;
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

async fn do_check(app_dir: &Path) {
    let local_json_path = app_dir.join("version_info.json");

    let remote_json_str = match HTTP_CLIENT.get(UPDATE_URL_JSON).send().await {
        Ok(resp) => match resp.text().await {
            Ok(s) => s,
            Err(_) => return,
        },
        Err(_) => return,
    };

    let remote_info: VersionInfo = match serde_json::from_str(&remote_json_str) {
        Ok(info) => info,
        Err(_) => return,
    };

    let mut needs_update = false;
    if local_json_path.exists() {
        if let Ok(local_content) = fs::read_to_string(&local_json_path) {
            if let Ok(local_info) = serde_json::from_str::<VersionInfo>(&local_content) {
                if remote_info.timestamp > local_info.timestamp {
                    needs_update = true;
                }
            } else {
                needs_update = true;
            }
        } else {
            needs_update = true;
        }
    } else {
        needs_update = true;
    }

    if needs_update {
        let title_w: Vec<u16> = format!("{}\0", tr("update_available_title"))
            .encode_utf16()
            .collect();
        let text_w: Vec<u16> = tr("update_available_desc")
            .replace("{}", &remote_info.timestamp)
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
            && (r == IDOK || r == IDYES) {
                perform_update(remote_json_str, app_dir.to_path_buf()).await;
            }
    }
}

async fn perform_update(remote_json_str: String, app_dir: PathBuf) {
    let bytes = match HTTP_CLIENT.get(UPDATE_URL_EXE).send().await {
        Ok(r) => match r.bytes().await {
            Ok(b) => b.to_vec(),
            Err(_) => {
                show_error_box(tr("update_failed_title"), tr("update_failed_dl")).await;
                return;
            }
        },
        Err(_) => {
            show_error_box(tr("update_failed_title"), tr("update_failed_dl")).await;
            return;
        }
    };

    let current_exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => {
            show_error_box(tr("update_failed_title"), tr("update_failed_save")).await;
            return;
        }
    };
    let new_exe_path = current_exe.with_extension("exe.new");

    if fs::write(&new_exe_path, &bytes).is_err() {
        show_error_box(tr("update_failed_title"), tr("update_failed_save")).await;
        return;
    }

    let local_json_path = app_dir.join("version_info.json");
    let _ = fs::write(local_json_path, remote_json_str);

    let current_exe_str = current_exe.to_string_lossy().into_owned();
    let new_exe_str = new_exe_path.to_string_lossy().into_owned();

    let pid = std::process::id();
    let script = format!(
        "Start-Sleep -Seconds 1; \
         while (Get-Process -Id {} -ErrorAction SilentlyContinue) {{ Start-Sleep -Milliseconds 100 }}; \
         Move-Item -Path '{}' -Destination '{}' -Force; \
         Start-Process -FilePath '{}'",
        pid, new_exe_str, current_exe_str, current_exe_str
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
