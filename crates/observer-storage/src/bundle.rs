//! bundle 契约 + 会话读/列举/导入导出。
//!
//! bundle 是平台「会话跨进程/跨机迁移的唯一契约」（见 docs/架构/bundle-规范.md）：
//! 本地文件分享 / 本地 server 实时流 / 云端上传三路共用。本模块提供纯逻辑，
//! 供 console Tauri command 与 [`observer_server`](../../observer-server) HTTP 路由共用。

use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::annotations::read_annotations;
use crate::storage::{now_ms, read_segment_events, segment_id_from_filename};

/// bundle 契约标识与版本（与 TS 侧 buildBundle/parseBundle 对齐，见 docs/架构/bundle-规范.md）。
pub const BUNDLE_FORMAT: &str = "prism-session";
pub const BUNDLE_VERSION: i64 = 1;

/// segmentId 合法性：`<label>#<n>`，label 仅 [A-Za-z0-9_-]。
/// segment key 会成为文件名（segments/<key>.jsonl），此校验是路径穿越防护的核心。
pub fn validate_segment_id(id: &str) -> bool {
    let Some((label, n)) = id.split_once('#') else {
        return false;
    };
    if label.is_empty() || n.is_empty() {
        return false;
    }
    if !label
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return false;
    }
    n.bytes().all(|b| b.is_ascii_digit())
}

/// session ID 合法性：纯数字（与 [`unique_id`] 生成器一致）。
/// read API 的 `/sessions/:id` 直接 `root.join(id)`，未校验可被 `../` 跨租户逃逸。
/// 多租户（P9）下这是必校验项；单租户也应校验。
pub fn validate_session_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())
}

/// 分配不与本地已有会话冲突的新 id（毫秒时间戳，冲突则自增）。
pub fn unique_id(root: &Path) -> String {
    let mut id = now_ms();
    loop {
        let s = id.to_string();
        if !root.join(&s).exists() {
            return s;
        }
        id += 1;
    }
}

/// 读会话目录组装成回放数据（session + windows + segments + annotations）。
/// 供 console `read_session` 命令与 server `GET /sessions/:id` 共用。
/// segments 读路径 gzip 感知（`.jsonl` / `.jsonl.gz` 均可），见 [`read_segment_events`]。
pub fn read_session(dir: &Path) -> Result<Value, String> {
    let session = serde_json::from_str::<Value>(
        &fs::read_to_string(dir.join("session.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let windows: Vec<Value> = fs::read_to_string(dir.join("windows.jsonl"))
        .ok()
        .and_then(|s| s.lines().map(|l| serde_json::from_str(l).ok()).collect())
        .unwrap_or_default();

    let segments = read_segments_dir(dir);
    let annotations = read_annotations(dir);
    Ok(json!({ "session": session, "windows": windows, "segments": segments, "annotations": annotations }))
}

/// 列出所有会话 meta（按 startedAt 倒序）。
pub fn list_sessions(root: &Path) -> Vec<Value> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for e in entries.flatten() {
            if let Ok(meta) = fs::read_to_string(e.path().join("session.json")) {
                if let Ok(v) = serde_json::from_str::<Value>(&meta) {
                    out.push(v);
                }
            }
        }
    }
    out.sort_by(|a, b| b["startedAt"].as_i64().cmp(&a["startedAt"].as_i64()));
    out
}

/// 读 segments 目录所有段（gzip 感知），返回 segmentId -> events 映射。
fn read_segments_dir(dir: &Path) -> Value {
    let mut segments = serde_json::Map::new();
    let seg_dir = dir.join("segments");
    if let Ok(entries) = fs::read_dir(&seg_dir) {
        for e in entries.flatten() {
            let path = e.path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let seg_id = segment_id_from_filename(&name).to_string();
            let events = read_segment_events(&path);
            segments.insert(seg_id, Value::Array(events));
        }
    }
    Value::Object(segments)
}

/// 读会话目录组装成 export bundle（不依赖 AppHandle，便于测试）。
pub fn build_export_bundle(dir: &Path) -> Result<Value, String> {
    let session = serde_json::from_str::<Value>(
        &fs::read_to_string(dir.join("session.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let windows: Vec<Value> = fs::read_to_string(dir.join("windows.jsonl"))
        .ok()
        .and_then(|s| s.lines().map(|l| serde_json::from_str(l).ok()).collect())
        .unwrap_or_default();
    let segments = read_segments_dir(dir);
    let annotations = read_annotations(dir);
    Ok(json!({
        "format": BUNDLE_FORMAT,
        "version": BUNDLE_VERSION,
        "exportedAt": now_ms(),
        "session": session,
        "windows": windows,
        "segments": segments,
        "annotations": annotations,
    }))
}

/// 合并 meta 到 session.json（空串/null 删除字段，否则覆盖），返回写回后的 session。
pub fn merge_session_meta(path: &Path, meta: &Value) -> Result<Value, String> {
    let mut v = serde_json::from_str::<Value>(
        &fs::read_to_string(path).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if let (Some(obj), Some(m)) = (v.as_object_mut(), meta.as_object()) {
        for (k, val) in m {
            // 空字符串 / null = 删除该字段（清理），否则覆盖
            if (val.is_string() && val.as_str() == Some("")) || val.is_null() {
                obj.remove(k);
            } else {
                obj.insert(k.clone(), val.clone());
            }
        }
    }
    fs::write(path, v.to_string()).map_err(|e| e.to_string())?;
    Ok(v)
}

/// 把 bundle 重建为目标目录的文件结构（session.id 替换为 new_id，标记 importedAt）。
pub fn write_import_bundle(dir: &Path, bundle: &Value, new_id: &str) -> Result<(), String> {
    fs::create_dir_all(dir.join("segments")).map_err(|e| e.to_string())?;
    // session.json：保留原始 meta，替换 id 并标记导入时间
    let mut session = bundle["session"].clone();
    if let Some(obj) = session.as_object_mut() {
        obj.insert("id".into(), json!(new_id));
        obj.insert("importedAt".into(), json!(now_ms()));
    }
    fs::write(dir.join("session.json"), session.to_string()).map_err(|e| e.to_string())?;
    // windows.jsonl
    if let Some(wins) = bundle["windows"].as_array() {
        let mut body = String::new();
        for w in wins {
            body.push_str(&w.to_string());
            body.push('\n');
        }
        fs::write(dir.join("windows.jsonl"), body).map_err(|e| e.to_string())?;
    }
    // segments/<name>.jsonl -- name 来自 bundle，必须校验防路径穿越（B1）
    if let Some(segs) = bundle["segments"].as_object() {
        for (name, events) in segs {
            if !validate_segment_id(name) {
                return Err(format!("非法 segmentId（拒绝以防路径穿越）：{name}"));
            }
            if let Some(arr) = events.as_array() {
                let mut body = String::new();
                for e in arr {
                    body.push_str(&e.to_string());
                    body.push('\n');
                }
                fs::write(dir.join("segments").join(format!("{name}.jsonl")), body)
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    // annotations.jsonl
    if let Some(annos) = bundle["annotations"].as_array() {
        crate::annotations::write_annotations(dir, annos)?;
    }
    Ok(())
}

/// 校验 bundle 格式与版本，返回解析后的 Value。
pub fn parse_bundle(content: &str) -> Result<Value, String> {
    let v = serde_json::from_str::<Value>(content).map_err(|e| e.to_string())?;
    if v["format"].as_str() != Some(BUNDLE_FORMAT) {
        return Err(format!("不是有效的会话文件（缺少 format: {BUNDLE_FORMAT}）"));
    }
    let version = v["version"].as_i64().unwrap_or(1);
    if version > BUNDLE_VERSION {
        return Err(format!(
            "不支持的 bundle 版本：{version}（当前支持 ≤{BUNDLE_VERSION}）"
        ));
    }
    Ok(v)
}

/// 导入已校验的 bundle 到 root 下：分配新 id、原子写（.tmp -> rename）。返回新 id。
pub fn import_bundle(root: &Path, bundle: &Value) -> Result<String, String> {
    let new_id = unique_id(root);
    let dir = root.join(&new_id);
    let tmp = root.join(format!("{new_id}.tmp"));
    // 原子写：先写临时目录，成功后 rename；任一步失败清理 tmp，不污染 recordings/
    if tmp.exists() {
        let _ = fs::remove_dir_all(&tmp);
    }
    match write_import_bundle(&tmp, bundle, &new_id) {
        Ok(()) => fs::rename(&tmp, &dir).map_err(|e| {
            let _ = fs::remove_dir_all(&tmp);
            e.to_string()
        })?,
        Err(e) => {
            let _ = fs::remove_dir_all(&tmp);
            return Err(e);
        }
    }
    Ok(new_id)
}

/// 导入 bundle（从 JSON 字符串）：校验 + 原子写。返回新 id。
/// 供 console `import_session(content)` 与 server `POST /sessions/import` 共用。
pub fn import_bundle_content(root: &Path, content: &str) -> Result<String, String> {
    let v = parse_bundle(content)?;
    import_bundle(root, &v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_jsonl(path: &Path, rows: &[Value]) {
        let mut body = String::new();
        for r in rows {
            body.push_str(&r.to_string());
            body.push('\n');
        }
        fs::write(path, body).unwrap();
    }

    /// 导出 -> 导入闭环：建一个完整会话目录，导出为 bundle，再导入到新目录，
    /// 验证 session id 替换、原字段保留、windows/segments/annotations 均还原。
    #[test]
    fn export_import_roundtrip() {
        let root = tempdir().unwrap();
        let src = root.path().join("session-001");
        fs::create_dir_all(src.join("segments")).unwrap();
        fs::write(
            src.join("session.json"),
            json!({"id":"session-001","source":"self","startedAt":1000,"endedAt":2000})
                .to_string(),
        )
        .unwrap();
        write_jsonl(
            &src.join("windows.jsonl"),
            &[
                json!({"type":"shown","label":"main","segmentId":"main#0","t":1000}),
                json!({"type":"hidden","label":"main","segmentId":"main#0","t":2000}),
            ],
        );
        write_jsonl(
            &src.join("segments/main#0.jsonl"),
            &[
                json!({"type":2,"timestamp":1000}),
                json!({"type":6,"timestamp":1100,"data":{"plugin":"console"}}),
            ],
        );
        crate::annotations::write_annotations(
            &src,
            &[json!({"id":"a1","t":500,"text":"note here","author":"local","createdAt":1200})],
        )
        .unwrap();

        // 导出
        let bundle = build_export_bundle(&src).unwrap();
        assert_eq!(bundle["format"], "prism-session");
        assert_eq!(bundle["session"]["id"], "session-001");
        assert_eq!(bundle["session"]["source"], "self");
        assert_eq!(bundle["windows"].as_array().unwrap().len(), 2);
        assert_eq!(bundle["segments"]["main#0"].as_array().unwrap().len(), 2);
        assert_eq!(bundle["annotations"].as_array().unwrap().len(), 1);
        assert_eq!(bundle["annotations"][0]["text"], "note here");

        // 导入到新目录
        let dst = root.path().join("session-002");
        write_import_bundle(&dst, &bundle, "session-002").unwrap();

        // session id 替换、原字段保留、importedAt 标记
        let session: Value =
            serde_json::from_str(&fs::read_to_string(dst.join("session.json")).unwrap()).unwrap();
        assert_eq!(session["id"], "session-002");
        assert_eq!(session["source"], "self");
        assert_eq!(session["startedAt"], 1000);
        assert_eq!(session["endedAt"], 2000);
        assert!(session["importedAt"].as_i64().unwrap() > 0);

        // windows / segments / annotations 还原
        assert_eq!(
            fs::read_to_string(dst.join("windows.jsonl"))
                .unwrap()
                .lines()
                .count(),
            2
        );
        assert_eq!(
            fs::read_to_string(dst.join("segments/main#0.jsonl"))
                .unwrap()
                .lines()
                .count(),
            2
        );
        let annos = read_annotations(&dst);
        assert_eq!(annos.len(), 1);
        assert_eq!(annos[0]["text"], "note here");
        assert_eq!(annos[0]["author"], "local");
    }

    /// 元信息合并：覆盖已有字段、新增字段、空串删除字段。
    #[test]
    fn merge_session_meta_overrides_adds_and_clears() {
        let root = tempdir().unwrap();
        let path = root.path().join("session.json");
        fs::write(
            &path,
            json!({"id":"s1","startedAt":100,"name":"old"}).to_string(),
        )
        .unwrap();

        // 覆盖 name + 新增 note/tags
        let v = merge_session_meta(
            &path,
            &json!({"name":"new name","note":"a note","tags":["bug","login"]}),
        )
        .unwrap();
        assert_eq!(v["name"], "new name");
        assert_eq!(v["note"], "a note");
        assert_eq!(v["tags"].as_array().unwrap().len(), 2);
        assert_eq!(v["startedAt"], 100); // 原字段不受影响

        // 空串 = 删除字段
        let v2 = merge_session_meta(&path, &json!({"name":""})).unwrap();
        assert!(v2.get("name").is_none());
        assert!(v2.get("note").is_some()); // 其他字段保留
    }

    /// segmentId 校验：合法的通过，含路径分隔符/`..` 的拒绝（路径穿越防护）。
    #[test]
    fn segment_id_validation() {
        assert!(validate_segment_id("main#0"));
        assert!(validate_segment_id("web#1"));
        assert!(validate_segment_id("settings#12"));
        assert!(validate_segment_id("a-b_c#9"));
        // 非法：路径穿越企图
        assert!(!validate_segment_id("../etc#0"));
        assert!(!validate_segment_id("a/..#1"));
        assert!(!validate_segment_id("a\\b#1"));
        // 非法：其他形态
        assert!(!validate_segment_id("a#b")); // n 非数字
        assert!(!validate_segment_id("#1")); // label 空
        assert!(!validate_segment_id("a#")); // n 空
        assert!(!validate_segment_id("a")); // 无 #
    }

    /// session ID 校验：纯数字通过，含 `..`/`/`/字母 被拒（read API 路径穿越防护）。
    #[test]
    fn session_id_validation() {
        assert!(validate_session_id("1750000000000"));
        assert!(validate_session_id("0"));
        assert!(validate_session_id("123456789"));
        // 非法：路径穿越企图
        assert!(!validate_session_id("../1"));
        assert!(!validate_session_id("1/2"));
        assert!(!validate_session_id(".."));
        // 非法：非纯数字
        assert!(!validate_session_id("abc"));
        assert!(!validate_session_id("1a"));
        assert!(!validate_session_id(""));
        assert!(!validate_session_id("1-2"));
    }

    /// 含恶意 segment key 的 bundle 必须被拒绝，恶意文件不得落盘（路径穿越防护）。
    #[test]
    fn import_rejects_path_traversal() {
        let root = tempdir().unwrap();
        let dir = root.path().join("dst");
        let bundle = json!({
            "format": BUNDLE_FORMAT,
            "version": BUNDLE_VERSION,
            "session": {"id":"x","startedAt":1},
            "windows": [],
            "segments": { "../evil#0": [{"type":2,"timestamp":1}] },
            "annotations": [],
        });
        let err = write_import_bundle(&dir, &bundle, "dst").unwrap_err();
        assert!(err.contains("路径穿越"), "got: {err}");
        // 未写穿：traversal 目标文件不应存在（无校验时会落到 dst/evil#0.jsonl）
        assert!(
            !dir.join("evil#0.jsonl").exists(),
            "恶意 segment 不得落盘到段目录之外"
        );
    }

    /// parse_bundle：合法 bundle 通过，缺 format / 版本过高被拒。
    #[test]
    fn parse_bundle_validates() {
        let ok = json!({
            "format": BUNDLE_FORMAT, "version": 1,
            "session": {"id":"x","startedAt":1}, "windows": [], "segments": {}, "annotations": [],
        })
        .to_string();
        assert!(parse_bundle(&ok).is_ok());

        let no_format = json!({"version": 1, "session": {}}).to_string();
        assert!(parse_bundle(&no_format).is_err());

        let too_new = json!({"format": BUNDLE_FORMAT, "version": 99, "session": {}}).to_string();
        let err = parse_bundle(&too_new).unwrap_err();
        assert!(err.contains("版本"));
    }

    /// import_bundle_content：完整闭环，分配新 id 并原子落盘。
    #[test]
    fn import_bundle_content_roundtrip() {
        let root = tempdir().unwrap();
        let bundle = json!({
            "format": BUNDLE_FORMAT, "version": 1,
            "session": {"id":"orig","source":"web","startedAt":100},
            "windows": [json!({"type":"shown","label":"web","segmentId":"web#1","t":100})],
            "segments": {"web#1": [json!({"type":2,"timestamp":100})]},
            "annotations": [],
        })
        .to_string();
        let new_id = import_bundle_content(root.path(), &bundle).unwrap();
        let dir = root.path().join(&new_id);
        assert!(dir.join("session.json").exists());
        let session: Value =
            serde_json::from_str(&fs::read_to_string(dir.join("session.json")).unwrap()).unwrap();
        assert_eq!(session["id"], new_id);
        assert_eq!(session["source"], "web");
    }

    /// read_session 支持 gzip segment 文件（混合 plain + gz 目录）。
    #[test]
    fn read_session_with_gzip_segments() {
        let root = tempdir().unwrap();
        let dir = root.path().join("s1");
        fs::create_dir_all(dir.join("segments")).unwrap();
        fs::write(
            &dir.join("session.json"),
            json!({"id":"s1","startedAt":1}).to_string(),
        )
        .unwrap();
        // 一个 plain segment + 一个 gzip segment
        crate::storage::append_events_file(
            &dir,
            "plain#0",
            &[json!({"type":2,"timestamp":1})],
        )
        .unwrap();
        crate::storage::append_events_file_with(
            &dir,
            "gz#0",
            &[json!({"type":2,"timestamp":2}), json!({"type":6,"timestamp":3})],
            &crate::storage::WriteOpts::gzip(),
        )
        .unwrap();

        let data = read_session(&dir).unwrap();
        let segs = data["segments"].as_object().unwrap();
        assert_eq!(segs.len(), 2);
        assert_eq!(segs["plain#0"].as_array().unwrap().len(), 1);
        assert_eq!(segs["gz#0"].as_array().unwrap().len(), 2);
    }
}
