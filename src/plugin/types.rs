pub use winisland_plugin_api::*;

pub fn read_c_str(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
}

impl From<&PluginMetadataC> for PluginMetadata {
    fn from(value: &PluginMetadataC) -> Self {
        Self {
            id: read_c_str(&value.id),
            name: read_c_str(&value.name),
            version: read_c_str(&value.version),
            author: read_c_str(&value.author),
            description: read_c_str(&value.description),
        }
    }
}

#[derive(Debug)]
pub enum PluginError {
    NotFound(String),
    LoadFailed(String),
    InvalidPlugin(String),
    ExecutionError(String),
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(message) => write!(f, "Plugin not found: {message}"),
            Self::LoadFailed(message) => write!(f, "Failed to load plugin: {message}"),
            Self::InvalidPlugin(message) => write!(f, "Invalid plugin: {message}"),
            Self::ExecutionError(message) => write!(f, "Plugin execution error: {message}"),
        }
    }
}

impl std::error::Error for PluginError {}

#[derive(Debug, Clone, Default)]
pub struct HostState {
    pub media_title: String,
    pub media_artist: String,
    pub is_playing: bool,
    pub theme: String,
}

impl From<&HostState> for HostStateV1 {
    fn from(value: &HostState) -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>() as u32,
            flags: 0,
            media_title: str_to_fixed(&value.media_title),
            media_artist: str_to_fixed(&value.media_artist),
            is_playing: u8::from(value.is_playing),
            reserved: [0; 7],
            theme: str_to_fixed(&value.theme),
        }
    }
}

pub fn context_from_ffi(
    _owner: PluginToken,
    id: ResourceId,
    value: &ContextDataV1,
) -> crate::core::context::PluginContext {
    let priority = match value.priority {
        PRIORITY_LOW => crate::core::context::Priority::Low,
        PRIORITY_HIGH => crate::core::context::Priority::High,
        _ => crate::core::context::Priority::Medium,
    };
    let timeout = if value.timeout_ms == 0 {
        None
    } else {
        Some(std::time::Instant::now() + std::time::Duration::from_millis(value.timeout_ms as u64))
    };
    crate::core::context::PluginContext {
        id,
        priority,
        title: read_c_str(&value.title),
        body: read_c_str(&value.body),
        compact_text: read_c_str(&value.compact_text),
        show_compact: value.flags & CONTEXT_FLAG_SHOW_COMPACT != 0,
        expires_at: timeout,
        updated_at: std::time::Instant::now(),
    }
}

pub fn widget_from_ffi(
    plugin_id: &str,
    id: ResourceId,
    value: &WidgetDataV1,
) -> crate::core::plugin_widget::PluginWidget {
    let key = read_c_str(&value.key);
    crate::core::plugin_widget::PluginWidget {
        id,
        plugin_id: plugin_id.to_string(),
        key: (!key.is_empty()).then_some(key),
        span_cols: value.span_cols,
        span_rows: value.span_rows,
        title: read_c_str(&value.title),
        body: read_c_str(&value.body),
        on_draw: value.on_draw,
        callback_data: value.callback_data as usize,
    }
}
