#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod core;
mod icons;
mod plugin;
mod ui;
mod utils;
mod window;
use crate::core::i18n::init_i18n;
use crate::window::app::App;
use std::env;
use std::mem::ManuallyDrop;
use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
use windows::Win32::Foundation::{CloseHandle, GetLastError};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::w;
use winit::event_loop::EventLoop;

fn main() {
    let _ = env_logger::try_init();
    let config = core::persistence::load_config();
    init_i18n(&config.language);

    let args: Vec<String> = env::args().collect();
    let is_restart = args.iter().any(|arg| arg == "--restart");

    if args.iter().any(|arg| arg == "--settings") {
        let _settings_mutex;
        // SAFETY: CreateMutexW creates a named mutex for single-instance enforcement.
        // The name is a static string literal. GetLastError checks if the mutex
        // already exists (ERROR_ALREADY_EXISTS) to bring existing window to front.
        unsafe {
            _settings_mutex = CreateMutexW(None, true, w!("Local\\WinIsland_Settings_Mutex"));
            if GetLastError() == ERROR_ALREADY_EXISTS {
                crate::window::settings::bring_settings_to_front();
                return;
            }
        }
        crate::window::settings::run_settings(config);
    } else {
        if is_restart {
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        let _single_mutex = {
            let start = std::time::Instant::now();
            loop {
                // SAFETY: CreateMutexW creates a named mutex for single-instance lock.
                // The name is a static string literal. On success with no ERROR_ALREADY_EXISTS,
                // the handle is kept in ManuallyDrop to hold the lock for the process lifetime.
                // On ERROR_ALREADY_EXISTS, the handle is closed and we retry or exit.
                unsafe {
                    let h = CreateMutexW(None, true, w!("Local\\WinIsland_SingleInstance_Mutex"));
                    match h {
                        Ok(handle) => {
                            if GetLastError() != ERROR_ALREADY_EXISTS {
                                break ManuallyDrop::new(handle);
                            }
                            let _ = CloseHandle(handle);
                        }
                        Err(_) => {
                            if !is_restart {
                                return;
                            }
                        }
                    }
                }
                if !is_restart || start.elapsed() > std::time::Duration::from_secs(10) {
                    if is_restart {
                        let own_pid = std::process::id();
                        if let Ok(output) = std::process::Command::new("powershell")
                            .args([
                                "-NoProfile",
                                "-Command",
                                &format!(
                                    "Get-Process WinIsland -ErrorAction SilentlyContinue | Where-Object {{$_.Id -ne {own_pid}}} | Stop-Process -Force"
                                ),
                            ])
                            .output()
                            && output.status.success()
                        {
                            std::thread::sleep(std::time::Duration::from_millis(500));
                            continue;
                        }
                    }
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        };

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let _guard = runtime.enter();

        utils::updater::start_update_checker();

        let event_loop = EventLoop::new().unwrap();
        let mut app = App::default();
        event_loop.run_app(&mut app).unwrap();
    }
}
