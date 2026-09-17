//! 会话存储层入口。
//!
//! 纯落盘原语（[`append_lifecycle`] / [`append_events_file`] / [`create_session`] /
//! [`finalize_session`] / [`now_ms`]）已抽到 [`observer_storage`] crate，与
//! [`observer_server`](../../crates/observer-server) 共用。本模块只保留 tauri 相关的
//! [`recordings_root_with`]/[`recordings_root`]，并 re-export 纯函数供插件内部（commands.rs）沿用 `crate::storage::` 路径。
//!
//! 格式见 docs/架构/分析侧（平台布局结构、功能）.md：
//! `recordings/<sessionId>/{session.json, windows.jsonl, segments/<label>#<n>.jsonl}`

pub use observer_storage::storage::{
    append_events_file, append_lifecycle, create_session, finalize_session, now_ms,
};
// P16：Local 模式只读命令（list/export）复用的 bundle 契约函数。
pub use observer_storage::bundle::{build_export_bundle, list_sessions, validate_session_id};

use std::path::PathBuf;

use crate::config::{DirBase, ObserverConfig};
use tauri::{AppHandle, Manager, Runtime};

/// recordings 根目录（按配置解析）：`<dir_base>/<dir_name>`（P20）。
/// 基目录缺失 panic 与原实现语义一致（环境异常时无合理缺省）。
pub fn recordings_root_with<R: Runtime>(app: &AppHandle<R>, cfg: &ObserverConfig) -> PathBuf {
    let base = match cfg.dir_base {
        DirBase::AppData => app.path().app_data_dir().expect("app data dir"),
        DirBase::ResourceDir => app.path().resource_dir().expect("resource dir"),
    };
    base.join(&cfg.dir_name)
}

/// recordings 根目录：`appDataDir/recordings`（默认配置）。
/// 兼容保留：console 宿主 13 处直接调用，签名不可变更（P20）。
pub fn recordings_root<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    recordings_root_with(app, &ObserverConfig::default())
}
