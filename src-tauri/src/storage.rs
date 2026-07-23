//! 会话存储层：self-obs 命令与外部 /ingest HTTP handler 共用的落盘函数。
//! 格式见 docs/架构/分析侧（平台布局结构、功能）.md。
//! `recordings/<sessionId>/{session.json, windows.jsonl, segments/<label>#<n>.jsonl}`

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use tauri::Manager;

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn recordings_root(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("app data dir")
        .join("recordings")
}

pub fn append_lifecycle(dir: &Path, evt: Value) -> Result<(), String> {
    let path = dir.join("windows.jsonl");
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    writeln!(f, "{}", evt).map_err(|e| e.to_string())
}

pub fn append_events_file(dir: &Path, segment_id: &str, events: &[Value]) -> Result<(), String> {
    let seg_dir = dir.join("segments");
    fs::create_dir_all(&seg_dir).map_err(|e| e.to_string())?;
    let path = seg_dir.join(format!("{}.jsonl", segment_id));
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    for e in events {
        writeln!(f, "{}", e).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 建会话目录并写 session.json（meta 由调用方组装好）。
pub fn create_session(dir: &Path, meta: Value) -> Result<(), String> {
    fs::create_dir_all(dir.join("segments")).map_err(|e| e.to_string())?;
    fs::write(dir.join("session.json"), meta.to_string()).map_err(|e| e.to_string())
}

/// 结束会话：读回 session.json 补 endedAt 后覆写。返回原 meta（供调用方记录）。
pub fn finalize_session(dir: &Path, ended_at: i64) -> Result<(), String> {
    let path = dir.join("session.json");
    let mut v = serde_json::from_str::<Value>(
        &fs::read_to_string(&path).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if let Some(obj) = v.as_object_mut() {
        obj.insert("endedAt".into(), json!(ended_at));
    }
    fs::write(path, v.to_string()).map_err(|e| e.to_string())
}
