//! 会话存储原语：落盘函数，全部吃 `&Path`，零 tauri 依赖。
//!
//! 供 [`tauri_plugin_observer`]（Local 模式 self-obs 落盘）与
//! [`observer_server`](../../observer-server)（HTTP 落盘）共用。
//!
//! 格式：`recordings/<sessionId>/{session.json, windows.jsonl, segments/<label>#<n>.jsonl[.gz]}`。
//! P9 起 segments 可选 gzip 落盘（`.jsonl.gz`），读路径按扩展名透明解压。

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use flate2::read::MultiGzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde_json::Value;

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// segment 落盘选项。`gzip=true` 写 `.jsonl.gz`（每批一个 gzip member，可追加）。
#[derive(Clone, Copy, Default)]
pub struct WriteOpts {
    pub gzip: bool,
}

impl WriteOpts {
    pub fn plain() -> Self {
        Self { gzip: false }
    }
    pub fn gzip() -> Self {
        Self { gzip: true }
    }
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

/// 追加事件到 segment 文件（plain JSONL，向后兼容）。
pub fn append_events_file(dir: &Path, segment_id: &str, events: &[Value]) -> Result<(), String> {
    append_events_file_with(dir, segment_id, events, &WriteOpts::plain())
}

/// 追加事件到 segment 文件，按 [`WriteOpts`] 决定是否 gzip。
///
/// gzip 模式下每次调用写一个独立 gzip member（concatenate 到同一文件），
/// 读侧用 [`read_segment_events`] 经 [`MultiGzDecoder`] 透明解压全部 member。
pub fn append_events_file_with(
    dir: &Path,
    segment_id: &str,
    events: &[Value],
    opts: &WriteOpts,
) -> Result<(), String> {
    let seg_dir = dir.join("segments");
    fs::create_dir_all(&seg_dir).map_err(|e| e.to_string())?;
    let fname = if opts.gzip {
        format!("{}.jsonl.gz", segment_id)
    } else {
        format!("{}.jsonl", segment_id)
    };
    let path = seg_dir.join(&fname);
    let f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    if opts.gzip {
        let mut enc = GzEncoder::new(f, Compression::default());
        for e in events {
            writeln!(enc, "{}", e).map_err(|e| e.to_string())?;
        }
        enc.finish().map_err(|e| e.to_string())?;
    } else {
        let mut f = f;
        for e in events {
            writeln!(f, "{}", e).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// 读 segment 文件事件（gzip 感知：`.jsonl.gz` 用 [`MultiGzDecoder`]，`.jsonl` 直读）。
/// 文件不存在或单行解析失败被跳过。
pub fn read_segment_events(path: &Path) -> Vec<Value> {
    let Some(raw) = read_segment_text(path) else {
        return Vec::new();
    };
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

/// 由文件名还原 segmentId：`web#1.jsonl.gz` / `web#1.jsonl` -> `web#1`。
pub fn segment_id_from_filename(name: &str) -> &str {
    name.strip_suffix(".jsonl.gz")
        .or_else(|| name.strip_suffix(".jsonl"))
        .unwrap_or(name)
}

fn read_segment_text(path: &Path) -> Option<String> {
    let is_gz = path.extension().and_then(|e| e.to_str()) == Some("gz");
    if is_gz {
        let f = fs::File::open(path).ok()?;
        let mut dec = MultiGzDecoder::new(f);
        let mut s = String::new();
        dec.read_to_string(&mut s).ok()?;
        Some(s)
    } else {
        fs::read_to_string(path).ok()
    }
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

    /// gzip 追加：两次调用 = 两个 concatenate gzip member，读侧 MultiGzDecoder 还原全部。
    #[test]
    fn append_events_gzip_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let seg = "web#1";
        append_events_file_with(
            dir.path(),
            seg,
            &[json!({ "type": 2, "timestamp": 1 })],
            &WriteOpts::gzip(),
        )
        .unwrap();
        append_events_file_with(
            dir.path(),
            seg,
            &[json!({ "type": 6, "timestamp": 2 })],
            &WriteOpts::gzip(),
        )
        .unwrap();

        let path = dir.path().join("segments/web#1.jsonl.gz");
        assert!(path.exists(), ".gz 文件应存在");
        assert!(
            !dir.path().join("segments/web#1.jsonl").exists(),
            "不应同时存在 plain 文件"
        );

        let events = read_segment_events(&path);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["type"], 2);
        assert_eq!(events[1]["type"], 6);
    }

    /// 读路径同时支持 plain 与 gz，按扩展名分派。
    #[test]
    fn read_segment_events_plain_and_gz() {
        let dir = tempfile::tempdir().unwrap();
        append_events_file(dir.path(), "a#0", &[json!({ "type": 2, "timestamp": 1 })]).unwrap();
        let plain = read_segment_events(&dir.path().join("segments/a#0.jsonl"));
        assert_eq!(plain.len(), 1);

        append_events_file_with(
            dir.path(),
            "b#0",
            &[json!({ "type": 2, "timestamp": 2 })],
            &WriteOpts::gzip(),
        )
        .unwrap();
        let gz = read_segment_events(&dir.path().join("segments/b#0.jsonl.gz"));
        assert_eq!(gz.len(), 1);
        assert_eq!(gz[0]["timestamp"], 2);
    }

    #[test]
    fn segment_id_from_filename_strips_extensions() {
        assert_eq!(segment_id_from_filename("web#1.jsonl.gz"), "web#1");
        assert_eq!(segment_id_from_filename("web#1.jsonl"), "web#1");
        assert_eq!(segment_id_from_filename("web#1"), "web#1");
    }
}
