use std::time::Instant;

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", ts)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Medium = 1,
    High = 2,
}

impl Priority {
    #[allow(dead_code)]
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::Low),
            1 => Some(Self::Medium),
            2 => Some(Self::High),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContextId {
    pub source: String,
    pub uuid: String,
}

impl ContextId {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            uuid: generate_id(),
        }
    }

    pub fn from_encoded(s: &str) -> Option<Self> {
        let (source, uuid) = s.split_once(':')?;
        Some(Self {
            source: source.to_string(),
            uuid: uuid.to_string(),
        })
    }

    pub fn encode(&self) -> String {
        format!("{}:{}", self.source, self.uuid)
    }
}

#[derive(Debug, Clone)]
pub struct PluginContext {
    pub id: ContextId,
    pub priority: Priority,
    pub title: String,
    pub body: String,
    #[allow(dead_code)]
    pub icon: Vec<u8>,
    pub duration_sec: u32,
    pub mini_render: bool,
    #[allow(dead_code)]
    pub mini_text: String,
    #[allow(dead_code)]
    pub created_at: Instant,
    pub expanded_started_at: Option<Instant>,
    pub collapsed_at: Option<Instant>,
    pub mini_timeout_start: Option<Instant>,
}
