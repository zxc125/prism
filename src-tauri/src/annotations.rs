//! 会话级标注存储：annotations.jsonl，每行一条标注 JSON。
//! 标注是回放侧「人对会话的批注」，与 segment 事件流分离，保持原始录制数据纯净。
//! 整体覆盖写入（save_annotations）：前端持有完整列表，增删改后整体覆写。

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// 会话目录下的标注文件路径。
pub fn annotations_path(dir: &Path) -> PathBuf {
    dir.join("annotations.jsonl")
}

/// 读全部标注（按文件行序）。无文件返回空数组。
pub fn read_annotations(dir: &Path) -> Vec<Value> {
    fs::read_to_string(annotations_path(dir))
        .ok()
        .and_then(|s| {
            s.lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| serde_json::from_str(l).ok())
                .collect()
        })
        .unwrap_or_default()
}

/// 整体覆写标注文件（每行一条 JSON）。
pub fn write_annotations(dir: &Path, annotations: &[Value]) -> Result<(), String> {
    let path = annotations_path(dir);
    let mut body = String::new();
    for a in annotations {
        body.push_str(&a.to_string());
        body.push('\n');
    }
    fs::write(&path, body).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn write_then_read_roundtrip() {
        let dir = tempdir().unwrap();
        let annos = vec![
            json!({ "id": "a1", "t": 100, "text": "first", "author": "local" }),
            json!({ "id": "a2", "t": 500, "text": "second", "author": "local" }),
        ];
        write_annotations(dir.path(), &annos).unwrap();
        let got = read_annotations(dir.path());
        assert_eq!(got.len(), 2);
        assert_eq!(got[0]["id"], "a1");
        assert_eq!(got[1]["text"], "second");
    }

    #[test]
    fn read_missing_file_is_empty() {
        let dir = tempdir().unwrap();
        assert!(read_annotations(dir.path()).is_empty());
    }

    #[test]
    fn overwrite_replaces_all() {
        let dir = tempdir().unwrap();
        write_annotations(dir.path(), &[json!({ "id": "a1", "t": 0 })]).unwrap();
        write_annotations(dir.path(), &[json!({ "id": "b1", "t": 9 })]).unwrap();
        let got = read_annotations(dir.path());
        assert_eq!(got.len(), 1);
        assert_eq!(got[0]["id"], "b1");
    }
}
