//! HTTP 路由：ingest（/ingest/*）+ 读/管理 API（/sessions/*）。
//!
//! [`handle_route`] 从 console 旧 ingest.rs 抽出（纯存储路由，吃 `&Path`），
//! 供 console 内嵌 server 与独立二进制共用。[`handle_read_route`] 是 P8 新增的
//! 读/管理 API，对齐 console 的 Tauri command（list/read/annotations/meta/export/import/delete）。

use std::collections::HashMap;
use std::path::Path;

use serde_json::{json, Value};

use observer_storage::{
    append_events_file, append_lifecycle, build_export_bundle, create_session, finalize_session,
    import_bundle, list_sessions, merge_session_meta, now_ms, read_annotations, read_session,
    write_annotations, BUNDLE_FORMAT, BUNDLE_VERSION,
};

fn bad(msg: &str) -> (u16, String) {
    (400, msg.to_string())
}

fn unique_session_id(root: &Path) -> String {
    let mut id = now_ms();
    loop {
        let s = id.to_string();
        if !root.join(&s).exists() {
            return s;
        }
        id += 1;
    }
}

/// ingest 路由 /ingest/* -> 落盘。返回 (status, 可选 JSON 体)。
/// 纯存储逻辑（不依赖 server 状态机）：root 即 recordings 目录，
/// open_segments 由调用方持有（session/end 据此补 hidden）。
pub fn handle_route(
    root: &Path,
    open_segments: &mut HashMap<String, Vec<String>>,
    url: &str,
    body: Value,
) -> Result<(u16, Option<String>), (u16, String)> {
    match url {
        "/ingest/session" => {
            let id = unique_session_id(root);
            let dir = root.join(&id);
            let mut meta = if body.is_object() {
                body
            } else {
                json!({})
            };
            if let Some(obj) = meta.as_object_mut() {
                obj.insert("id".into(), json!(id));
                obj.insert("startedAt".into(), json!(now_ms()));
            }
            create_session(&dir, meta).map_err(|e| (500, e))?;
            Ok((200, Some(json!({ "sessionId": id }).to_string())))
        }
        "/ingest/segment" => {
            let session_id = body["sessionId"]
                .as_str()
                .ok_or_else(|| bad("missing sessionId"))?
                .to_string();
            let label = body["label"].as_str().unwrap_or("web").to_string();
            let segment_id = body["segmentId"]
                .as_str()
                .ok_or_else(|| bad("missing segmentId"))?
                .to_string();
            let started_at = body["startedAt"].as_i64().unwrap_or_else(now_ms);
            let dir = root.join(&session_id);
            append_lifecycle(
                &dir,
                json!({ "type": "shown", "label": label, "segmentId": segment_id, "t": started_at }),
            )
            .map_err(|e| (500, e))?;
            open_segments.entry(session_id).or_default().push(segment_id);
            Ok((204, None))
        }
        "/ingest/events" => {
            let session_id = body["sessionId"]
                .as_str()
                .ok_or_else(|| bad("missing sessionId"))?;
            let segment_id = body["segmentId"]
                .as_str()
                .ok_or_else(|| bad("missing segmentId"))?;
            let events = body["events"]
                .as_array()
                .ok_or_else(|| bad("missing events array"))?;
            let dir = root.join(session_id);
            append_events_file(&dir, segment_id, events).map_err(|e| (500, e))?;
            Ok((204, None))
        }
        "/ingest/lifecycle" => {
            let session_id = body["sessionId"]
                .as_str()
                .ok_or_else(|| bad("missing sessionId"))?;
            let dir = root.join(session_id);
            let mut evt = json!({
                "type": body["type"],
                "label": body["label"],
                "t": body["t"],
            });
            if let Some(sid) = body.get("segmentId") {
                evt["segmentId"] = sid.clone();
            }
            append_lifecycle(&dir, evt).map_err(|e| (500, e))?;
            Ok((204, None))
        }
        "/ingest/session/end" => {
            let session_id = body["sessionId"]
                .as_str()
                .ok_or_else(|| bad("missing sessionId"))?
                .to_string();
            let ended_at = body["endedAt"].as_i64().unwrap_or_else(now_ms);
            let dir = root.join(&session_id);
            let segs = open_segments.remove(&session_id).unwrap_or_default();
            for seg in segs {
                let label = seg.split('#').next().unwrap_or("web").to_string();
                let _ = append_lifecycle(
                    &dir,
                    json!({ "type": "hidden", "label": label, "segmentId": seg, "t": ended_at }),
                );
            }
            let _ = finalize_session(&dir, ended_at); // best-effort：会话可能无 /end（页面卸载）
            Ok((204, None))
        }
        _ => Err((404, format!("unknown route: {url}"))),
    }
}

/// 读/管理路由 /sessions/*。对齐 console 的 Tauri command，使 HttpBackend 与
/// TauriBackend 行为一致。method 为大写 HTTP 动词，url 为路径（不含 query）。
pub fn handle_read_route(
    root: &Path,
    method: &str,
    url: &str,
    body: Value,
) -> Result<(u16, Option<String>), (u16, String)> {
    let path = url.split('?').next().unwrap_or(url);
    let segs: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    match (method, segs.as_slice()) {
        ("GET", ["sessions"]) => {
            let list = list_sessions(root);
            Ok((200, Some(Value::Array(list).to_string())))
        }
        ("GET", ["sessions", id]) => {
            let dir = root.join(id);
            let data = read_session(&dir).map_err(|e| (404, e))?;
            Ok((200, Some(data.to_string())))
        }
        ("GET", ["sessions", id, "annotations"]) => {
            let dir = root.join(id);
            if !dir.join("session.json").exists() {
                return Err((404, "session not found".into()));
            }
            let annos = read_annotations(&dir);
            Ok((200, Some(Value::Array(annos).to_string())))
        }
        ("POST", ["sessions", id, "annotations"]) => {
            let dir = root.join(id);
            if !dir.join("session.json").exists() {
                return Err((404, "session not found".into()));
            }
            let annos = body
                .as_array()
                .ok_or_else(|| bad("expected annotations array"))?;
            write_annotations(&dir, annos).map_err(|e| (500, e))?;
            Ok((204, None))
        }
        ("PATCH", ["sessions", id]) => {
            let dir = root.join(id);
            let path = dir.join("session.json");
            if !path.exists() {
                return Err((404, "session not found".into()));
            }
            let v = merge_session_meta(&path, &body).map_err(|e| (500, e))?;
            Ok((200, Some(v.to_string())))
        }
        ("GET", ["sessions", id, "export"]) => {
            let dir = root.join(id);
            let bundle = build_export_bundle(&dir).map_err(|e| (404, e))?;
            Ok((200, Some(bundle.to_string())))
        }
        ("POST", ["sessions", "import"]) => {
            // body 即 bundle JSON（server 已 parse）；校验 format/version
            if body["format"].as_str() != Some(BUNDLE_FORMAT) {
                return Err((400, format!("不是有效的会话文件（缺少 format: {BUNDLE_FORMAT}）")));
            }
            let version = body["version"].as_i64().unwrap_or(1);
            if version > BUNDLE_VERSION {
                return Err((400, format!("不支持的 bundle 版本：{version}")));
            }
            let new_id = import_bundle(root, &body).map_err(|e| (400, e))?;
            Ok((200, Some(json!({ "sessionId": new_id }).to_string())))
        }
        ("DELETE", ["sessions", id]) => {
            let dir = root.join(id);
            if !dir.exists() {
                return Err((404, "session not found".into()));
            }
            std::fs::remove_dir_all(&dir).map_err(|e| (500, e.to_string()))?;
            Ok((204, None))
        }
        _ => Err((404, format!("unknown route: {method} {url}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn read_jsonl(path: &std::path::Path) -> Vec<Value> {
        std::fs::read_to_string(path)
            .unwrap()
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    /// 外部 web SDK 一次完整会话：session -> segment -> events(含 type:6 信号) -> end，
    /// 落盘后应能被 read_session 正确解析（windows.jsonl 有 shown/hidden、segment 文件含交错事件）。
    #[test]
    fn web_session_full_lifecycle() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let mut open = HashMap::new();

        let (_st, body) = handle_route(
            root,
            &mut open,
            "/ingest/session",
            json!({ "source": "web", "appId": "demo", "env": "dev" }),
        )
        .unwrap();
        let sid = serde_json::from_str::<Value>(&body.unwrap()).unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(open.is_empty());

        handle_route(
            root,
            &mut open,
            "/ingest/segment",
            json!({ "sessionId": &sid, "label": "web", "segmentId": "web#1", "startedAt": 100 }),
        )
        .unwrap();
        assert_eq!(open.get(&sid).map(|v| v.len()), Some(1));

        handle_route(
            root,
            &mut open,
            "/ingest/events",
            json!({
                "sessionId": &sid,
                "segmentId": "web#1",
                "events": [
                    { "type": 2, "timestamp": 100 },
                    { "type": 6, "timestamp": 101, "data": { "plugin": "console", "payload": { "level": "error" } } }
                ]
            }),
        )
        .unwrap();

        let (st, _) = handle_route(
            root,
            &mut open,
            "/ingest/session/end",
            json!({ "sessionId": &sid, "endedAt": 200 }),
        )
        .unwrap();
        assert_eq!(st, 204);
        assert!(open.is_empty());

        let sdir = root.join(&sid);
        let session: Value =
            serde_json::from_str(&std::fs::read_to_string(sdir.join("session.json")).unwrap())
                .unwrap();
        assert_eq!(session["source"], "web");
        assert_eq!(session["appId"], "demo");
        assert!(session["startedAt"].as_i64().unwrap() > 0);
        assert_eq!(session["endedAt"].as_i64(), Some(200));

        let win = read_jsonl(&sdir.join("windows.jsonl"));
        assert_eq!(win.len(), 2);
        assert_eq!(win[0]["type"], "shown");
        assert_eq!(win[0]["segmentId"], "web#1");
        assert_eq!(win[1]["type"], "hidden");
        assert_eq!(win[1]["segmentId"], "web#1");

        let seg = read_jsonl(&sdir.join("segments/web#1.jsonl"));
        assert_eq!(seg.len(), 2);
        assert_eq!(seg[0]["type"], 2);
        assert_eq!(seg[1]["type"], 6);
        assert_eq!(seg[1]["data"]["plugin"], "console");
    }

    /// 缺字段应返回 400，不落盘。
    #[test]
    fn events_missing_field_is_400() {
        let dir = tempdir().unwrap();
        let mut open = HashMap::new();
        let err = handle_route(
            dir.path(),
            &mut open,
            "/ingest/events",
            json!({ "sessionId": "x" }),
        )
        .unwrap_err();
        assert_eq!(err.0, 400);
    }

    /// 读 API：ingest 落盘后，list / read / annotations / export / delete 全链路。
    #[test]
    fn read_api_lifecycle() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let mut open = HashMap::new();

        // ingest 一个会话
        let (_, body) = handle_route(
            root,
            &mut open,
            "/ingest/session",
            json!({ "source": "web" }),
        )
        .unwrap();
        let sid = serde_json::from_str::<Value>(&body.unwrap()).unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();

        // GET /sessions
        let (st, list_body) =
            handle_read_route(root, "GET", "/sessions", Value::Null).unwrap();
        assert_eq!(st, 200);
        let list: Vec<Value> = serde_json::from_str(&list_body.unwrap()).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["id"], sid);

        // GET /sessions/:id
        let (st, read_body) =
            handle_read_route(root, "GET", &format!("/sessions/{sid}"), Value::Null).unwrap();
        assert_eq!(st, 200);
        let data: Value = serde_json::from_str(&read_body.unwrap()).unwrap();
        assert_eq!(data["session"]["id"], sid);

        // POST /sessions/:id/annotations
        let (st, _) = handle_read_route(
            root,
            "POST",
            &format!("/sessions/{sid}/annotations"),
            json!([{ "id": "a1", "t": 0, "text": "note", "author": "x", "createdAt": 1 }]),
        )
        .unwrap();
        assert_eq!(st, 204);

        // GET /sessions/:id/annotations
        let (st, annos_body) = handle_read_route(
            root,
            "GET",
            &format!("/sessions/{sid}/annotations"),
            Value::Null,
        )
        .unwrap();
        assert_eq!(st, 200);
        let annos: Vec<Value> = serde_json::from_str(&annos_body.unwrap()).unwrap();
        assert_eq!(annos.len(), 1);
        assert_eq!(annos[0]["text"], "note");

        // GET /sessions/:id/export
        let (st, export_body) =
            handle_read_route(root, "GET", &format!("/sessions/{sid}/export"), Value::Null).unwrap();
        assert_eq!(st, 200);
        let bundle: Value = serde_json::from_str(&export_body.unwrap()).unwrap();
        assert_eq!(bundle["format"], "rrweb-demo-session");
        assert_eq!(bundle["annotations"].as_array().unwrap().len(), 1);

        // POST /sessions/import (上传 bundle 到新 id)
        let (st, import_body) =
            handle_read_route(root, "POST", "/sessions/import", bundle.clone()).unwrap();
        assert_eq!(st, 200);
        let new_id = serde_json::from_str::<Value>(&import_body.unwrap()).unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();
        assert_ne!(new_id, sid);

        // PATCH /sessions/:id
        let (st, _) =
            handle_read_route(root, "PATCH", &format!("/sessions/{new_id}"), json!({"name":"renamed"}))
                .unwrap();
        assert_eq!(st, 200);

        // DELETE /sessions/:id
        let (st, _) =
            handle_read_route(root, "DELETE", &format!("/sessions/{sid}"), Value::Null).unwrap();
        assert_eq!(st, 204);
        assert!(!root.join(&sid).exists());

        // 404 不存在的会话
        let (st, _) =
            handle_read_route(root, "GET", "/sessions/nope", Value::Null).unwrap_err();
        assert_eq!(st, 404);
    }
}
