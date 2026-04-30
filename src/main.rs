#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod core;
mod icons;
mod ui;
mod utils;
mod window;
use crate::core::i18n::init_i18n;
use crate::window::app::App;
use std::env;
use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::w;
use winit::event_loop::EventLoop;

fn main() {
    let config = crate::core::persistence::load_config();
    init_i18n(&config.language);

    let args: Vec<String> = env::args().collect();
    if args.iter().any(|arg| arg == "--settings") {
        crate::window::settings::run_settings(config);
    } else {
        unsafe {
            let _ = CreateMutexW(None, true, w!("Local\\WinIsland_SingleInstance_Mutex"));
            if GetLastError() == ERROR_ALREADY_EXISTS {
                return;
            }
        }

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let _guard = runtime.enter();

        crate::utils::updater::start_update_checker();

        let event_loop = EventLoop::new().unwrap();
        let mut app = App::default();
        event_loop.run_app(&mut app).unwrap();
    }
}
