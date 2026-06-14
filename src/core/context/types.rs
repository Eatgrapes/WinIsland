use std::time::Instant;

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", ts)
}

/// 上下文优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// 媒体播放，后台常驻（SMTC）
    Low = 0,
    /// 进行中的短期活动（插件默认）
    Medium = 1,
    /// 需要即时注意的通知
    High = 2,
}

impl Priority {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::Low),
            1 => Some(Self::Medium),
            2 => Some(Self::High),
            _ => None,
        }
    }

    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

/// 上下文唯一标识
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContextId {
    /// "smtc" 或插件 ID
    pub source: String,
    /// 随机 UUID
    pub uuid: String,
}

impl ContextId {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            uuid: generate_id(),
        }
    }

    /// 从 "source:uuid" 格式解析
    pub fn from_encoded(s: &str) -> Option<Self> {
        let (source, uuid) = s.split_once(':')?;
        Some(Self {
            source: source.to_string(),
            uuid: uuid.to_string(),
        })
    }

    /// 编码为 "source:uuid"
    pub fn encode(&self) -> String {
        format!("{}:{}", self.source, self.uuid)
    }
}

/// 插件或系统发送的上下文
#[derive(Debug, Clone)]
pub struct PluginContext {
    pub id: ContextId,
    pub priority: Priority,
    /// 标题（mini 显示，expanded 标题行）
    pub title: String,
    /// expanded 正文
    pub body: String,
    /// 图标 PNG bytes
    pub icon: Vec<u8>,
    /// expanded 停留秒数
    pub duration_sec: u32,
    /// 是否在 mini 显示摘要
    pub mini_render: bool,
    /// mini 摘要文本（mini_render=true 时有意义）
    pub mini_text: String,
    pub created_at: Instant,
    pub expanded_started_at: Option<Instant>,
    pub collapsed_at: Option<Instant>,
    /// 30 秒超时计时起点（collapsed 时设置）
    pub mini_timeout_start: Option<Instant>,
}
