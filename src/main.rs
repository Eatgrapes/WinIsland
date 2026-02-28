#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod core;
mod window;
mod utils;

use crate::window::app::App;
use winit::event_loop::EventLoop;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.iter().any(|arg| arg == "--settings") {
        let config = crate::core::persistence::load_config();
        crate::window::settings::run_settings(config);
    } else {
        let event_loop = EventLoop::new().unwrap();
        let mut app = App::default();
        event_loop.run_app(&mut app).unwrap();
    }
}
