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
use windows::Win32::Foundation::GetLastError;
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::w;
use winit::event_loop::EventLoop;

fn main() {
    let config = core::persistence::load_config();
    init_i18n(&config.language);

    let args: Vec<String> = env::args().collect();
    let is_restart = args.iter().any(|arg| arg == "--restart");

    if args.iter().any(|arg| arg == "--settings") {
        let _settings_mutex;
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
                unsafe {
                    let h = CreateMutexW(None, true, w!("Local\\WinIsland_SingleInstance_Mutex"));
                    if GetLastError() != ERROR_ALREADY_EXISTS {
                        break ManuallyDrop::new(h);
                    }
                }
                if !is_restart || start.elapsed() > std::time::Duration::from_secs(10) {
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
