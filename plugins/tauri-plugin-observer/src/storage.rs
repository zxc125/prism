//! 会话存储层入口。
//!
//! 纯落盘原语（[`append_lifecycle`] / [`append_events_file`] / [`create_session`] /
//! [`finalize_session`] / [`now_ms`]）已抽到 [`observer_storage`] crate，与
//! [`observer_server`](../../crates/observer-server) 共用。本模块只保留 tauri 相关的
//! [`recordings_root`]，并 re-export 纯函数供插件内部（commands.rs）沿用 `crate::storage::` 路径。
//!
//! 格式见 docs/架构/分析侧（平台布局结构、功能）.md：
//! `recordings/<sessionId>/{session.json, windows.jsonl, segments/<label>#<n>.jsonl}`

pub use observer_storage::storage::{
    append_events_file, append_lifecycle, create_session, finalize_session, now_ms,
};

use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};

/// recordings 根目录：`appDataDir/recordings`。
pub fn recordings_root<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("app data dir")
        .join("recordings")
}
