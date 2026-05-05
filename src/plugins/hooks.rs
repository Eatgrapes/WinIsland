#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum HookType {
    MediaChange,
    PlaybackStateChange,
    AudioSpectrum,
    Render,
    Tick,
    KeyPress,
    Shutdown,
}

pub struct HookManager {
    hooks: std::collections::HashMap<HookType, Vec<Box<dyn FnMut() + Send + 'static>>>,
}

impl HookManager {
    pub fn new() -> Self {
        Self {
            hooks: std::collections::HashMap::new(),
        }
    }
    
    pub fn register_hook<F>(&mut self, hook_type: HookType, callback: F)
    where
        F: FnMut() + Send + 'static,
    {
        self.hooks.entry(hook_type).or_insert_with(Vec::new).push(Box::new(callback));
    }
    
    pub fn trigger_hooks(&mut self, hook_type: HookType) {
        if let Some(callbacks) = self.hooks.get_mut(&hook_type) {
            for callback in callbacks.iter_mut() {
                callback();
            }
        }
    }
}
