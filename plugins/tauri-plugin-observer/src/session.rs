//! 录制会话状态。active 期间所有窗口的 rrweb 事件按 segment 落盘（Local）
//! 或仅协调（Remote，前端走 HttpSink 上报）。

use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::ObserverConfig;

pub struct Session {
    pub config: ObserverConfig,
    pub id: Option<String>,
    pub started_at: i64,
    /// Local 模式：会话目录 `appDataDir/recordings/<id>/`。
    pub dir: Option<PathBuf>,
    pub active: bool,
    /// label -> 下一段序号（Local 模式生成 segmentId `<label>#<n>`）。
    pub segment_seq: HashMap<String, u64>,
    /// label -> 当前活跃 segmentId（Local 模式 hide 时据此记 hidden）。
    pub current: HashMap<String, String>,
    /// Remote 模式：前端从 console server 拿到并绑定的 sessionId（跨窗口共享）。
    pub remote_session_id: Option<String>,
}

impl Session {
    pub fn new(config: ObserverConfig) -> Self {
        Self {
            config,
            id: None,
            started_at: 0,
            dir: None,
            active: false,
            segment_seq: HashMap::new(),
            current: HashMap::new(),
            remote_session_id: None,
        }
    }
}
