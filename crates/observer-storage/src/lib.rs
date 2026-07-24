//! 纯存储层：会话落盘原语 + 标注 + bundle 契约 + 读/列举/导入导出。
//!
//! 零 tauri 依赖，全部吃 `&Path`，供两方共用：
//! - [`tauri_plugin_observer`]（Local 模式 self-obs 落盘）
//! - [`observer_server`](../../observer-server)（HTTP 服务，console 内嵌 / 独立二进制）
//!
//! 格式见 docs/架构/分析侧（平台布局结构、功能）.md 与 docs/架构/bundle-规范.md：
//! `recordings/<sessionId>/{session.json, windows.jsonl, segments/<label>#<n>.jsonl, annotations.jsonl}`

pub mod annotations;
pub mod bundle;
pub mod storage;

pub use annotations::{annotations_path, read_annotations, write_annotations};
pub use bundle::{
    build_export_bundle, import_bundle, import_bundle_content, list_sessions, merge_session_meta,
    parse_bundle, read_session, unique_id, validate_segment_id, write_import_bundle,
    BUNDLE_FORMAT, BUNDLE_VERSION,
};
pub use storage::{append_events_file, append_lifecycle, create_session, finalize_session, now_ms};
