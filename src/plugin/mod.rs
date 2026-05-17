pub mod loader;
pub mod manager;
pub mod types;

pub use manager::PluginManager;

pub fn init() -> PluginManager {
    let manager = PluginManager::default();
    manager.load_all();
    manager
}
