//! 会话存储原语：落盘函数，全部吃 `&Path`，零 tauri 依赖。
//!
//! 供 [`tauri_plugin_observer`]（Local 模式 self-obs 落盘）与
//! [`observer_server`](../../observer-server)（HTTP 落盘）共用。
//!
//! 格式：`recordings/<sessionId>/{session.json, windows.jsonl, segments/<label>#<n>.jsonl}`。

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
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

/// 结束会话：读回 session.json 补 endedAt 后覆写。
pub fn finalize_session(dir: &Path, ended_at: i64) -> Result<(), String> {
    let path = dir.join("session.json");
    let mut v = serde_json::from_str::<Value>(
        &fs::read_to_string(&path).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if let Some(obj) = v.as_object_mut() {
        obj.insert("endedAt".into(), serde_json::json!(ended_at));
    }
    fs::write(path, v.to_string()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn append_events_writes_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        append_events_file(
            dir.path(),
            "main#0",
            &[json!({ "type": 2, "timestamp": 1 }), json!({ "type": 6, "timestamp": 2 })],
        )
        .unwrap();
        let lines: Vec<Value> = fs::read_to_string(dir.path().join("segments/main#0.jsonl"))
            .unwrap()
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1]["type"], 6);
    }
}
