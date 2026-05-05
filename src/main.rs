#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod core;
mod window;
mod utils;
mod icons;
mod ui;
mod plugins;
use crate::window::app::App;
use std::env;
use std::sync::Arc;
use windows::core::w;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
use windows::Win32::System::Threading::CreateMutexW;
use winit::event_loop::EventLoop;
use crate::core::i18n::init_i18n;
use crate::plugins::manager::PluginManager;

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

        let plugin_manager = Arc::new(PluginManager::new(Arc::new(config.clone())));
        let plugin_results = plugin_manager.load_all_plugins();
        
        for result in plugin_results {
            match result {
                Ok(info) => println!("Loaded plugin: {} v{} by {}", info.name, info.version, info.author),
                Err(e) => eprintln!("Failed to load plugin: {:?}", e),
            }
        }

        let event_loop = EventLoop::new().unwrap();
        let mut app = App::new(Arc::clone(&plugin_manager));
        event_loop.run_app(&mut app).unwrap();
    }
}
