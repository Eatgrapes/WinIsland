#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod core;
mod window;
mod utils;
mod icons;
mod ui;
use crate::window::app::App;
use std::env;
use windows::core::w;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
use windows::Win32::System::Threading::CreateMutexW;
use winit::event_loop::EventLoop;
use crate::core::i18n::init_i18n;

fn main() {
    let config = crate::core::persistence::load_config();
    init_i18n(&config.language);

    let args: Vec<String> = env::args().collect();
    if args.iter().any(|arg| arg == "--settings") {
        crate::window::settings::run_settings(config);
    } else if args.iter().any(|arg| arg == "--music-settings") {
        crate::window::music_settings::run_music_settings(config);
    } else if args.iter().any(|arg| arg == "--notification-settings") {
        crate::window::notification_settings::run_notification_settings(config);
    } else {
        unsafe {
            let _ = CreateMutexW(None, true, w!("Local\\WinIsland_SingleInstance_Mutex"));
            if GetLastError() == ERROR_ALREADY_EXISTS {
                return;
            }
        }
        crate::utils::updater::start_update_checker();
        
        let event_loop = EventLoop::new().unwrap();
        let mut app = App::default();
        event_loop.run_app(&mut app).unwrap();
    }
}
