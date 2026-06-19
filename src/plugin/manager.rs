#![allow(dead_code)]

use super::loader::NativePlugin;
use super::types::read_c_str;
use super::types::{
    ContentProvider, Plugin, PluginError, PluginType, ShortcutProvider, ThemeProvider,
};
use super::zip_loader::{self, PluginManifest};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::sync::{Mutex, OnceLock};

/// Buffered plugin media source, drained by the main thread each frame.
pub struct PendingMediaSource {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub position_ms: u64,
    pub is_playing: bool,
    pub cover_data: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Global router — C callbacks route through thread-safe pending buffers
// ---------------------------------------------------------------------------

static PENDING_CONTEXTS: OnceLock<Mutex<Vec<crate::core::context::PluginContext>>> =
    OnceLock::new();
static PENDING_CLOSE: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
static PENDING_MEDIA_SOURCE: OnceLock<Mutex<Option<PendingMediaSource>>> = OnceLock::new();
static PLUGIN_HANDLES: OnceLock<Mutex<HashMap<isize, String>>> = OnceLock::new();
static HOST_STATE: OnceLock<Mutex<crate::plugin::types::HostState>> = OnceLock::new();
/// Leaked `'static` HostApiC — plugins hold a raw pointer to this.
static HOST_API: OnceLock<Box<crate::plugin::types::HostApiC>> = OnceLock::new();

/// Initialise the global plugin→host routing. Must be called once at startup.
///
/// Returns a `*const` to a leaked `'static` HostApiC that plugins can safely
/// store and call through for the entire process lifetime.
pub fn init_host_api() -> *const crate::plugin::types::HostApiC {
    PENDING_CONTEXTS.get_or_init(|| Mutex::new(Vec::new()));
    PENDING_CLOSE.get_or_init(|| Mutex::new(Vec::new()));
    PENDING_MEDIA_SOURCE.get_or_init(|| Mutex::new(None));
    PLUGIN_HANDLES.get_or_init(|| Mutex::new(HashMap::new()));
    HOST_STATE.get_or_init(|| Mutex::new(crate::plugin::types::HostState::default()));

    let api = HOST_API.get_or_init(|| {
        Box::new(crate::plugin::types::HostApiC {
            send_context: host_send_context,
            close_context: host_close_context,
            query_host_state: host_query_host_state,
            set_media_source: host_set_media_source,
            clear_media_source: host_clear_media_source,
        })
    });
    api.as_ref() as *const _
}

/// Drain all pending plugin contexts and push them into the ContextManager.
/// Called once per frame from the main loop.
pub fn drain_pending_contexts(ctx_mgr: &mut crate::core::context::ContextManager) {
    if let Some(buf) = PENDING_CONTEXTS.get()
        && let Ok(mut v) = buf.lock()
    {
        for ctx in v.drain(..) {
            ctx_mgr.push_context(ctx);
        }
    }
    if let Some(buf) = PENDING_CLOSE.get()
        && let Ok(mut v) = buf.lock()
    {
        for encoded in v.drain(..) {
            if let Some(id) = crate::core::context::ContextId::from_encoded(&encoded) {
                ctx_mgr.close_context(&id);
            }
        }
    }
}

/// Register a plugin handle → plugin_id mapping.
pub fn register_plugin_handle(handle: isize, plugin_id: &str) {
    if let Some(map) = PLUGIN_HANDLES.get()
        && let Ok(mut m) = map.lock()
    {
        m.insert(handle, plugin_id.to_string());
    }
}

/// Remove a plugin handle mapping on unload.
pub fn deregister_plugin_handle(handle: isize) {
    if let Some(map) = PLUGIN_HANDLES.get()
        && let Ok(mut m) = map.lock()
    {
        m.remove(&handle);
    }
}

/// Update the cached host state (called from app.rs when SMTC changes).
pub fn update_host_state(state: crate::plugin::types::HostState) {
    if let Some(s) = HOST_STATE.get()
        && let Ok(mut m) = s.lock()
    {
        *m = state;
    }
}

/// Drain the pending plugin media source. Returns `None` if cleared or empty.
pub fn drain_pending_media_source() -> Option<PendingMediaSource> {
    PENDING_MEDIA_SOURCE.get()?.lock().ok()?.take()
}

unsafe extern "C" fn host_set_media_source(
    _handle: crate::plugin::types::PluginHandle,
    data: crate::plugin::types::MediaSourceC,
) -> crate::plugin::types::PluginResultC {
    let raw = read_c_str(&data.title);
    if raw.is_empty() {
        return crate::plugin::types::PluginResultC::err("title is empty");
    }

    let cover_data = if !data.cover_data.is_null() && data.cover_len > 0 {
        unsafe { std::slice::from_raw_parts(data.cover_data, data.cover_len as usize) }.to_vec()
    } else {
        Vec::new()
    };

    if let Some(buf) = PENDING_MEDIA_SOURCE.get()
        && let Ok(mut m) = buf.lock()
    {
        *m = Some(PendingMediaSource {
            title: raw,
            artist: read_c_str(&data.artist),
            album: read_c_str(&data.album),
            duration_ms: data.duration_ms,
            position_ms: data.position_ms,
            is_playing: data.is_playing,
            cover_data,
        });
    }
    crate::plugin::types::PluginResultC::ok()
}

unsafe extern "C" fn host_clear_media_source(
    _handle: crate::plugin::types::PluginHandle,
) -> crate::plugin::types::PluginResultC {
    if let Some(buf) = PENDING_MEDIA_SOURCE.get()
        && let Ok(mut m) = buf.lock()
    {
        *m = None;
    }
    crate::plugin::types::PluginResultC::ok()
}

unsafe extern "C" fn host_send_context(
    handle: crate::plugin::types::PluginHandle,
    data: crate::plugin::types::ContextDataC,
) -> crate::plugin::types::ContextIdC {
    let plugin_id = PLUGIN_HANDLES
        .get()
        .and_then(|m| m.lock().ok())
        .and_then(|m| m.get(&(handle as isize)).cloned())
        .unwrap_or_default();

    let mut ctx = crate::core::context::PluginContext::from(&data);
    ctx.id.source = plugin_id.clone();

    let encoded_id = if let Some(buf) = PENDING_CONTEXTS.get()
        && let Ok(mut v) = buf.lock()
    {
        ctx.id.uuid = crate::core::context::ContextId::new(&plugin_id).uuid;
        let encoded = ctx.id.encode();
        v.push(ctx);
        encoded
    } else {
        String::new()
    };

    let mut id_buf = [0u8; 128];
    let len = encoded_id.len().min(127);
    id_buf[..len].copy_from_slice(encoded_id.as_bytes());
    crate::plugin::types::ContextIdC { id: id_buf }
}

unsafe extern "C" fn host_close_context(
    handle: crate::plugin::types::PluginHandle,
    id_str: *const std::ffi::c_char,
) -> crate::plugin::types::PluginResultC {
    let raw = unsafe { std::ffi::CStr::from_ptr(id_str) };
    let s = raw.to_string_lossy();
    if let Some(context_id) = crate::core::context::ContextId::from_encoded(&s) {
        let plugin_id = PLUGIN_HANDLES
            .get()
            .and_then(|m| m.lock().ok())
            .and_then(|m| m.get(&(handle as isize)).cloned())
            .unwrap_or_default();
        // Only allow closing own contexts
        if context_id.source != plugin_id {
            return crate::plugin::types::PluginResultC::err(
                "Cannot close another plugin's context",
            );
        }
        if let Some(buf) = PENDING_CLOSE.get()
            && let Ok(mut v) = buf.lock()
        {
            v.push(context_id.encode());
        }
        crate::plugin::types::PluginResultC::ok()
    } else {
        crate::plugin::types::PluginResultC::err("Invalid context ID format")
    }
}

unsafe extern "C" fn host_query_host_state(
    _handle: crate::plugin::types::PluginHandle,
) -> crate::plugin::types::HostStateC {
    HOST_STATE
        .get()
        .and_then(|m| m.lock().ok())
        .map(|m| crate::plugin::types::HostStateC::from(&*m))
        .unwrap_or_else(|| crate::plugin::types::HostStateC {
            media_title: [0u8; 256],
            media_artist: [0u8; 256],
            is_playing: false,
            theme: [0u8; 32],
        })
}

// ---------------------------------------------------------------------------
// PluginManager
// ---------------------------------------------------------------------------

pub struct PluginManager {
    entries: RwLock<Vec<NativePlugin>>,
    plugin_dir: PathBuf,
}

impl PluginManager {
    pub fn new<P: AsRef<Path>>(plugin_dir: P) -> Self {
        let plugin_dir = plugin_dir.as_ref().to_path_buf();
        let _ = std::fs::create_dir_all(&plugin_dir);

        Self {
            entries: RwLock::new(Vec::new()),
            plugin_dir,
        }
    }

    pub fn load_all(&self) {
        let dlls = discover_plugins(&self.plugin_dir);
        log::info!(
            "Discovering plugins in {}: {} DLL(s) found",
            self.plugin_dir.display(),
            dlls.len()
        );
        for dll_path in dlls {
            self.load_dll(&dll_path);
        }
    }

    pub(crate) fn load_dll(&self, dll_path: &Path) {
        match NativePlugin::load(dll_path) {
            Ok(native) => {
                let plugin_id = native.metadata().id.clone();

                // C4: reject duplicate plugin IDs
                let entries = match self.entries.read() {
                    Ok(g) => g,
                    Err(_) => {
                        log::error!("Lock poisoned while loading plugin '{}'", plugin_id);
                        return;
                    }
                };
                if entries.iter().any(|p| p.metadata().id == plugin_id) {
                    log::warn!("Plugin '{}' already loaded, skipping duplicate", plugin_id);
                    return;
                }
                drop(entries);

                if let Ok(mut entries) = self.entries.write() {
                    entries.push(native);
                    log::info!(
                        "Loaded plugin: {} ({})",
                        entries.last().unwrap().metadata().name,
                        plugin_id
                    );
                } else {
                    log::error!("Lock poisoned while adding plugin '{}'", plugin_id);
                }
            }
            Err(e) => {
                log::warn!("Failed to load plugin '{}': {}", dll_path.display(), e);
            }
        }
    }

    pub fn install_from_zip(&self, zip_path: &Path) -> Result<PluginManifest, String> {
        let (manifest, _extracted_dir, dll_paths) =
            zip_loader::extract_plugin(zip_path, &self.plugin_dir)?;

        for dll_path in &dll_paths {
            self.load_dll(Path::new(dll_path));
        }

        log::info!(
            "Installed plugin '{}' v{} by {}",
            manifest.name,
            manifest.version,
            manifest.author
        );
        Ok(manifest)
    }

    pub fn read_manifest_from_zip(&self, zip_path: &Path) -> Result<PluginManifest, String> {
        zip_loader::read_manifest_from_zip(zip_path)
    }

    pub fn validate_zip(&self, zip_path: &Path) -> Result<(), String> {
        zip_loader::validate_zip(zip_path)
    }

    pub fn cancel_pending_install(&self, manifest: &PluginManifest) {
        let dir_name = manifest.safe_dir_name();
        let path = self.plugin_dir.join(&dir_name);
        if path.exists() {
            let _ = std::fs::remove_dir_all(&path);
        }
    }

    pub fn unload(&self, plugin_id: &str) -> Result<(), PluginError> {
        let mut entries = self
            .entries
            .write()
            .map_err(|e| PluginError::ExecutionError(format!("Lock poisoned: {}", e)))?;
        let idx = entries
            .iter()
            .position(|p| p.metadata().id == plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;
        let plugin = entries.remove(idx);
        log::info!(
            "Plugin unloaded: {} ({})",
            plugin.metadata().name,
            plugin_id
        );
        Ok(())
    }

    pub fn list_content_providers(&self) -> Vec<String> {
        let entries = match self.entries.read() {
            Ok(g) => g,
            Err(e) => {
                log::error!("Plugin lock poisoned: {}", e);
                return Vec::new();
            }
        };
        entries
            .iter()
            .filter(|p| p.plugin_type() == PluginType::Content)
            .map(|p| p.metadata().id.clone())
            .collect()
    }

    pub fn list_theme_providers(&self) -> Vec<String> {
        let entries = match self.entries.read() {
            Ok(g) => g,
            Err(e) => {
                log::error!("Plugin lock poisoned: {}", e);
                return Vec::new();
            }
        };
        entries
            .iter()
            .filter(|p| p.plugin_type() == PluginType::Theme)
            .map(|p| p.metadata().id.clone())
            .collect()
    }

    pub fn list_shortcut_providers(&self) -> Vec<String> {
        let entries = match self.entries.read() {
            Ok(g) => g,
            Err(e) => {
                log::error!("Plugin lock poisoned: {}", e);
                return Vec::new();
            }
        };
        entries
            .iter()
            .filter(|p| p.plugin_type() == PluginType::Shortcut)
            .map(|p| p.metadata().id.clone())
            .collect()
    }

    /// Call `set_host_api` on every loaded plugin and register its handle.
    pub fn init_plugin_host_api(&self, api: *const crate::plugin::types::HostApiC) {
        let entries = match self.entries.read() {
            Ok(g) => g,
            Err(e) => {
                log::error!("Plugin lock poisoned: {}", e);
                return;
            }
        };
        for plugin in entries.iter() {
            let handle = plugin.handle_raw();
            plugin.set_host_api(api);
            register_plugin_handle(handle, &plugin.metadata().id);
        }
    }

    pub fn with_content<F, R>(&self, plugin_id: &str, f: F) -> Result<R, PluginError>
    where
        F: FnOnce(&dyn ContentProvider) -> R,
    {
        let entries = self
            .entries
            .read()
            .map_err(|e| PluginError::ExecutionError(format!("Lock poisoned: {}", e)))?;
        let entry = entries
            .iter()
            .find(|p| p.metadata().id == plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        if entry.plugin_type() != PluginType::Content {
            return Err(PluginError::InvalidPlugin(format!(
                "Plugin '{}' is not a ContentProvider",
                plugin_id
            )));
        }

        Ok(f(entry))
    }

    pub fn with_content_mut<F, R>(&self, plugin_id: &str, f: F) -> Result<R, PluginError>
    where
        F: FnOnce(&mut dyn ContentProvider) -> R,
    {
        let mut entries = self
            .entries
            .write()
            .map_err(|e| PluginError::ExecutionError(format!("Lock poisoned: {}", e)))?;
        let entry = entries
            .iter_mut()
            .find(|p| p.metadata().id == plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        if entry.plugin_type() != PluginType::Content {
            return Err(PluginError::InvalidPlugin(format!(
                "Plugin '{}' is not a ContentProvider",
                plugin_id
            )));
        }

        Ok(f(entry))
    }

    pub fn with_theme<F, R>(&self, plugin_id: &str, f: F) -> Result<R, PluginError>
    where
        F: FnOnce(&dyn ThemeProvider) -> R,
    {
        let entries = self
            .entries
            .read()
            .map_err(|e| PluginError::ExecutionError(format!("Lock poisoned: {}", e)))?;
        let entry = entries
            .iter()
            .find(|p| p.metadata().id == plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        if entry.plugin_type() != PluginType::Theme {
            return Err(PluginError::InvalidPlugin(format!(
                "Plugin '{}' is not a ThemeProvider",
                plugin_id
            )));
        }

        Ok(f(entry))
    }

    pub fn with_shortcut_mut<F, R>(&self, plugin_id: &str, f: F) -> Result<R, PluginError>
    where
        F: FnOnce(&mut dyn ShortcutProvider) -> R,
    {
        let mut entries = self
            .entries
            .write()
            .map_err(|e| PluginError::ExecutionError(format!("Lock poisoned: {}", e)))?;
        let entry = entries
            .iter_mut()
            .find(|p| p.metadata().id == plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        if entry.plugin_type() != PluginType::Shortcut {
            return Err(PluginError::InvalidPlugin(format!(
                "Plugin '{}' is not a ShortcutProvider",
                plugin_id
            )));
        }

        Ok(f(entry))
    }

    /// Iterate over all content plugins and call `f` with each one.
    /// Stops early if `f` returns `Some<T>` on any plugin.
    pub fn find_content<F, T>(&self, mut f: F) -> Option<T>
    where
        F: FnMut(&mut dyn ContentProvider) -> Option<T>,
    {
        let mut entries = self.entries.write().ok()?;
        entries
            .iter_mut()
            .filter(|p| p.plugin_type() == PluginType::Content)
            .find_map(|entry| f(entry as &mut dyn ContentProvider))
    }

    pub fn plugin_dir(&self) -> &Path {
        &self.plugin_dir
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        let dir = dirs::config_dir()
            .unwrap_or_default()
            .join("WinIsland")
            .join("plugins");
        Self::new(dir)
    }
}

fn discover_plugins(plugin_dir: &Path) -> Vec<PathBuf> {
    if !plugin_dir.exists() {
        return Vec::new();
    }

    let mut result = Vec::new();
    match std::fs::read_dir(plugin_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Ok(sub) = std::fs::read_dir(&path) {
                        for e in sub.flatten() {
                            let p = e.path();
                            if p.extension().is_some_and(|ext| ext == "dll") {
                                result.push(p);
                            }
                        }
                    }
                } else if path.extension().is_some_and(|ext| ext == "dll") {
                    result.push(path);
                }
            }
        }
        Err(e) => {
            log::warn!(
                "Failed to read plugin directory '{}': {}",
                plugin_dir.display(),
                e
            );
        }
    }
    result
}
