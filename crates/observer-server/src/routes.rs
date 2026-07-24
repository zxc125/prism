//! HTTP 路由：ingest（/ingest/*）+ 读/管理 API（/sessions/*）。
//!
//! [`handle_route`] 从 console 旧 ingest.rs 抽出（纯存储路由，吃 `&Path`），
//! 供 console 内嵌 server 与独立二进制共用。[`handle_read_route`] 是 P8 新增的
//! 读/管理 API，对齐 console 的 Tauri command（list/read/annotations/meta/export/import/delete）。
//!
//! P9 起 两路由都吃 `Option<&TenantConfig>`：多租户模式下做 appId 越权校验 + import 前
//! redact + tenantId 写入 session meta；单租户（None）行为与 P8 一致。session ID 校验
//!（[`validate_session_id`]）无条件执行，堵 read API 路径穿越。

use std::collections::HashMap;
use std::path::Path;

use serde_json::{json, Value};

use observer_storage::{
    append_events_file, append_lifecycle, build_export_bundle, create_session, finalize_session,
    import_bundle, list_sessions, merge_session_meta, now_ms, read_annotations, read_session,
    redact_bundle, validate_session_id, write_annotations, BUNDLE_FORMAT, BUNDLE_VERSION,
};

use crate::tenant::TenantConfig;

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

/// open_segments 的 key：多租户用 `<tenantId>/<sessionId>` 防跨租户同 id 冲突。
fn seg_key(tenant: Option<&TenantConfig>, session_id: &str) -> String {
    match tenant {
        Some(t) => format!("{}/{}", t.tenant_id, session_id),
        None => session_id.to_string(),
    }
}

/// ingest 路由 /ingest/* -> 落盘。返回 (status, 可选 JSON 体)。
///
/// `tenant`：多租户模式下用于 appId 越权校验 + 写入 tenantId；None = 单租户透传。
/// `open_segments` 由调用方持有（session/end 据此补 hidden），key 已含 tenantId 防冲突。
pub fn handle_route(
    root: &Path,
    open_segments: &mut HashMap<String, Vec<String>>,
    url: &str,
    body: Value,
    tenant: Option<&TenantConfig>,
) -> Result<(u16, Option<String>), (u16, String)> {
    match url {
        "/ingest/session" => {
            let id = unique_session_id(root);
            let dir = root.join(&id);
            let mut meta = if body.is_object() { body } else { json!({}) };
            if let Some(obj) = meta.as_object_mut() {
                obj.insert("id".into(), json!(id));
                obj.insert("startedAt".into(), json!(now_ms()));
            }
            // 多租户：appId 越权校验 + 写入 tenantId（不信客户端）
            if let Some(t) = tenant {
                if !t.app_ids.is_empty() {
                    let app = meta
                        .get("appId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if !t.app_ids.iter().any(|a| *a == app) {
                        return Err((403, format!("appId {app} 不在授权集合")));
                    }
                }
                if let Some(obj) = meta.as_object_mut() {
                    obj.insert("tenantId".into(), json!(t.tenant_id));
                }
            }
            create_session(&dir, meta).map_err(|e| (500, e))?;
            Ok((200, Some(json!({ "sessionId": id }).to_string())))
        }
        "/ingest/segment" => {
            let session_id = body["sessionId"]
                .as_str()
                .ok_or_else(|| bad("missing sessionId"))?
                .to_string();
            if !validate_session_id(&session_id) {
                return Err(bad("invalid sessionId"));
            }
            let dir = root.join(&session_id);
            if !dir.join("session.json").exists() {
                return Err((404, format!("session {session_id} not found")));
            }
            let label = body["label"].as_str().unwrap_or("web").to_string();
            let segment_id = body["segmentId"]
                .as_str()
                .ok_or_else(|| bad("missing segmentId"))?
                .to_string();
            let started_at = body["startedAt"].as_i64().unwrap_or_else(now_ms);
            append_lifecycle(
                &dir,
                json!({ "type": "shown", "label": label, "segmentId": segment_id, "t": started_at }),
            )
            .map_err(|e| (500, e))?;
            open_segments
                .entry(seg_key(tenant, &session_id))
                .or_default()
                .push(segment_id);
            Ok((204, None))
        }
        "/ingest/events" => {
            let session_id = body["sessionId"]
                .as_str()
                .ok_or_else(|| bad("missing sessionId"))?;
            if !validate_session_id(session_id) {
                return Err(bad("invalid sessionId"));
            }
            let dir = root.join(session_id);
            if !dir.join("session.json").exists() {
                return Err((404, format!("session {session_id} not found")));
            }
            let segment_id = body["segmentId"]
                .as_str()
                .ok_or_else(|| bad("missing segmentId"))?;
            let events = body["events"]
                .as_array()
                .ok_or_else(|| bad("missing events array"))?;
            append_events_file(&dir, segment_id, events).map_err(|e| (500, e))?;
            Ok((204, None))
        }
        "/ingest/lifecycle" => {
            let session_id = body["sessionId"]
                .as_str()
                .ok_or_else(|| bad("missing sessionId"))?;
            if !validate_session_id(session_id) {
                return Err(bad("invalid sessionId"));
            }
            let dir = root.join(session_id);
            if !dir.join("session.json").exists() {
                return Err((404, format!("session {session_id} not found")));
            }
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
            if !validate_session_id(&session_id) {
                return Err(bad("invalid sessionId"));
            }
            let dir = root.join(&session_id);
            let ended_at = body["endedAt"].as_i64().unwrap_or_else(now_ms);
            let segs = open_segments
                .remove(&seg_key(tenant, &session_id))
                .unwrap_or_default();
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
///
/// `tenant`：多租户模式下 `POST /sessions/import` 入库前过 redact；None = 单租户透传。
/// 所有 `:id` 路由先过 [`validate_session_id`]，堵路径穿越。
pub fn handle_read_route(
    root: &Path,
    method: &str,
    url: &str,
    body: Value,
    tenant: Option<&TenantConfig>,
) -> Result<(u16, Option<String>), (u16, String)> {
    let path = url.split('?').next().unwrap_or(url);
    let segs: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    match (method, segs.as_slice()) {
        ("GET", ["sessions"]) => {
            let list = list_sessions(root);
            Ok((200, Some(Value::Array(list).to_string())))
        }
        ("GET", ["sessions", id]) => {
            if !validate_session_id(id) {
                return Err(bad("invalid session id"));
            }
            let dir = root.join(id);
            let data = read_session(&dir).map_err(|e| (404, e))?;
            Ok((200, Some(data.to_string())))
        }
        ("GET", ["sessions", id, "annotations"]) => {
            if !validate_session_id(id) {
                return Err(bad("invalid session id"));
            }
            let dir = root.join(id);
            if !dir.join("session.json").exists() {
                return Err((404, "session not found".into()));
            }
            let annos = read_annotations(&dir);
            Ok((200, Some(Value::Array(annos).to_string())))
        }
        ("POST", ["sessions", id, "annotations"]) => {
            if !validate_session_id(id) {
                return Err(bad("invalid session id"));
            }
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
            if !validate_session_id(id) {
                return Err(bad("invalid session id"));
            }
            let dir = root.join(id);
            let path = dir.join("session.json");
            if !path.exists() {
                return Err((404, "session not found".into()));
            }
            let v = merge_session_meta(&path, &body).map_err(|e| (500, e))?;
            Ok((200, Some(v.to_string())))
        }
        ("GET", ["sessions", id, "export"]) => {
            if !validate_session_id(id) {
                return Err(bad("invalid session id"));
            }
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
            // 多租户：入库前服务端 redact（不可逆，per-tenant scrubbers）
            let mut bundle = body;
            if let Some(t) = tenant {
                if let Ok(opts) = t.redact.to_opts() {
                    redact_bundle(&mut bundle, &opts);
                }
            }
            let new_id = import_bundle(root, &bundle).map_err(|e| (400, e))?;
            Ok((200, Some(json!({ "sessionId": new_id }).to_string())))
        }
        ("DELETE", ["sessions", id]) => {
            if !validate_session_id(id) {
                return Err(bad("invalid session id"));
            }
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
    use crate::tenant::RateLimitConfig;
    use observer_storage::{RedactConfig, RetentionPolicy};
    use tempfile::tempdir;

    fn read_jsonl(path: &std::path::Path) -> Vec<Value> {
        std::fs::read_to_string(path)
            .unwrap()
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    /// 构造一个多租户 TenantConfig（appId 校验 + redact）。
    fn tenant_acme() -> TenantConfig {
        TenantConfig {
            key: "sk_acme".into(),
            tenant_id: "acme".into(),
            app_ids: vec!["shop-web".into()],
            quota_bytes: None,
            retention: RetentionPolicy::default(),
            redact: RedactConfig {
                strip_network_body: true,
                strip_network_headers: true,
                scrubbers: vec![],
                ..Default::default()
            },
            rate_limit: RateLimitConfig::default(),
        }
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
            None,
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
            None,
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
            None,
        )
        .unwrap();

        let (st, _) = handle_route(
            root,
            &mut open,
            "/ingest/session/end",
            json!({ "sessionId": &sid, "endedAt": 200 }),
            None,
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

    /// 缺 segmentId（session 存在）应返回 400，不落盘。
    #[test]
    fn events_missing_field_is_400() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let mut open = HashMap::new();
        // 先建会话
        let (_, body) = handle_route(root, &mut open, "/ingest/session", json!({}), None).unwrap();
        let sid = serde_json::from_str::<Value>(&body.unwrap()).unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();

        let err = handle_route(
            root,
            &mut open,
            "/ingest/events",
            json!({ "sessionId": &sid }), // 缺 segmentId/events
            None,
        )
        .unwrap_err();
        assert_eq!(err.0, 400);
    }

    /// 非法 session id（路径穿越企图）在 ingest 与 read API 均被拒。
    #[test]
    fn invalid_session_id_rejected() {
        let dir = tempdir().unwrap();
        let mut open = HashMap::new();

        // ingest/events with "../1"
        let err = handle_route(
            dir.path(),
            &mut open,
            "/ingest/events",
            json!({ "sessionId": "../1", "segmentId": "web#1", "events": [] }),
            None,
        )
        .unwrap_err();
        assert_eq!(err.0, 400);

        // read API：`..` 作为 id 段（2 段路径）能到 validate_session_id，被拒 400
        let err = handle_read_route(dir.path(), "GET", "/sessions/..", Value::Null, None).unwrap_err();
        assert_eq!(err.0, 400);
        // 非数字 id 也拒
        let err = handle_read_route(dir.path(), "GET", "/sessions/abc", Value::Null, None).unwrap_err();
        assert_eq!(err.0, 400);
    }

    /// ingest 到不存在的 session 返回 404。
    #[test]
    fn ingest_events_unknown_session_404() {
        let dir = tempdir().unwrap();
        let mut open = HashMap::new();
        let err = handle_route(
            dir.path(),
            &mut open,
            "/ingest/events",
            json!({ "sessionId": "99999999999", "segmentId": "web#1", "events": [] }),
            None,
        )
        .unwrap_err();
        assert_eq!(err.0, 404);
    }

    /// 读 API：ingest 落盘后，list / read / annotations / export / delete 全链路。
    #[test]
    fn read_api_lifecycle() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let mut open = HashMap::new();

        // ingest 一个会话
        let (_, body) = handle_route(root, &mut open, "/ingest/session", json!({ "source": "web" }), None).unwrap();
        let sid = serde_json::from_str::<Value>(&body.unwrap()).unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();

        // GET /sessions
        let (st, list_body) = handle_read_route(root, "GET", "/sessions", Value::Null, None).unwrap();
        assert_eq!(st, 200);
        let list: Vec<Value> = serde_json::from_str(&list_body.unwrap()).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["id"], sid);

        // GET /sessions/:id
        let (st, read_body) =
            handle_read_route(root, "GET", &format!("/sessions/{sid}"), Value::Null, None).unwrap();
        assert_eq!(st, 200);
        let data: Value = serde_json::from_str(&read_body.unwrap()).unwrap();
        assert_eq!(data["session"]["id"], sid);

        // POST /sessions/:id/annotations
        let (st, _) = handle_read_route(
            root,
            "POST",
            &format!("/sessions/{sid}/annotations"),
            json!([{ "id": "a1", "t": 0, "text": "note", "author": "x", "createdAt": 1 }]),
            None,
        )
        .unwrap();
        assert_eq!(st, 204);

        // GET /sessions/:id/annotations
        let (st, annos_body) =
            handle_read_route(root, "GET", &format!("/sessions/{sid}/annotations"), Value::Null, None).unwrap();
        assert_eq!(st, 200);
        let annos: Vec<Value> = serde_json::from_str(&annos_body.unwrap()).unwrap();
        assert_eq!(annos.len(), 1);
        assert_eq!(annos[0]["text"], "note");

        // GET /sessions/:id/export
        let (st, export_body) =
            handle_read_route(root, "GET", &format!("/sessions/{sid}/export"), Value::Null, None).unwrap();
        assert_eq!(st, 200);
        let bundle: Value = serde_json::from_str(&export_body.unwrap()).unwrap();
        assert_eq!(bundle["format"], "rrweb-demo-session");
        assert_eq!(bundle["annotations"].as_array().unwrap().len(), 1);

        // POST /sessions/import (上传 bundle 到新 id)
        let (st, import_body) =
            handle_read_route(root, "POST", "/sessions/import", bundle.clone(), None).unwrap();
        assert_eq!(st, 200);
        let new_id = serde_json::from_str::<Value>(&import_body.unwrap()).unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();
        assert_ne!(new_id, sid);

        // PATCH /sessions/:id
        let (st, _) =
            handle_read_route(root, "PATCH", &format!("/sessions/{new_id}"), json!({"name":"renamed"}), None)
                .unwrap();
        assert_eq!(st, 200);

        // DELETE /sessions/:id
        let (st, _) =
            handle_read_route(root, "DELETE", &format!("/sessions/{sid}"), Value::Null, None).unwrap();
        assert_eq!(st, 204);
        assert!(!root.join(&sid).exists());

        // 404 不存在的会话（合法数字 id 但不存在）
        let (st, _) =
            handle_read_route(root, "GET", "/sessions/99999999999", Value::Null, None).unwrap_err();
        assert_eq!(st, 404);
    }

    /// 多租户：appId 越权被拒（403），合法 appId 通过且 session.json 写入 tenantId。
    #[test]
    fn app_id_authorization() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let t = tenant_acme();
        let mut open = HashMap::new();

        // 越权 appId -> 403
        let err = handle_route(
            root,
            &mut open,
            "/ingest/session",
            json!({ "appId": "other-app" }),
            Some(&t),
        )
        .unwrap_err();
        assert_eq!(err.0, 403);

        // 合法 appId -> 200，session.json 含 tenantId
        let (_, body) = handle_route(
            root,
            &mut open,
            "/ingest/session",
            json!({ "appId": "shop-web" }),
            Some(&t),
        )
        .unwrap();
        let sid = serde_json::from_str::<Value>(&body.unwrap()).unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();
        let session: Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(&sid).join("session.json")).unwrap())
                .unwrap();
        assert_eq!(session["tenantId"], "acme");
        assert_eq!(session["appId"], "shop-web");

        // app_ids 为空 = 不校验（向后兼容老 key）
        let t_open = TenantConfig {
            app_ids: vec![],
            ..tenant_acme()
        };
        let (st, _) = handle_route(
            root,
            &mut open,
            "/ingest/session",
            json!({ "appId": "anything" }),
            Some(&t_open),
        )
        .unwrap();
        assert_eq!(st, 200);
    }

    /// 多租户：import 时服务端 redact，network body/headers 被剥离。
    #[test]
    fn import_redacts_with_tenant() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let t = tenant_acme();

        let bundle = json!({
            "format": BUNDLE_FORMAT, "version": 1,
            "session": { "id": "orig", "startedAt": 1 },
            "windows": [],
            "segments": { "web#1": [
                json!({ "type": 6, "timestamp": 1, "data": { "plugin": "network", "payload": {
                    "reqBody": "secret", "resBody": "data", "method": "GET"
                }}})
            ]},
            "annotations": [],
        });

        let (_, body) =
            handle_read_route(root, "POST", "/sessions/import", bundle, Some(&t)).unwrap();
        let new_id = serde_json::from_str::<Value>(&body.unwrap()).unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();

        // 落盘的 segment 文件中 reqBody/resBody 应已被剥离
        let seg_path = root.join(&new_id).join("segments/web#1.jsonl");
        let events = read_jsonl(&seg_path);
        assert_eq!(events.len(), 1);
        let payload = &events[0]["data"]["payload"];
        assert!(payload.get("reqBody").is_none(), "reqBody 应被服务端 redact 剥离");
        assert!(payload.get("resBody").is_none(), "resBody 应被服务端 redact 剥离");
        assert_eq!(payload["method"], "GET"); // 非敏感字段保留
    }

    /// 多租户：open_segments 跨租户隔离（同 sessionId 不同 tenant 不冲突）。
    #[test]
    fn open_segments_tenant_isolation() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let t_acme = tenant_acme();
        let t_beta = TenantConfig {
            tenant_id: "beta".into(),
            app_ids: vec![],
            ..tenant_acme()
        };
        let mut open = HashMap::new();

        // 两租户各建一个 session（root 不同：acme/ vs beta/）
        let acme_root = root.join("acme");
        let beta_root = root.join("beta");
        std::fs::create_dir_all(&acme_root).unwrap();
        std::fs::create_dir_all(&beta_root).unwrap();

        let (_, a_body) = handle_route(&acme_root, &mut open, "/ingest/session", json!({ "appId": "shop-web" }), Some(&t_acme)).unwrap();
        let a_sid = serde_json::from_str::<Value>(&a_body.unwrap()).unwrap()["sessionId"].as_str().unwrap().to_string();
        let (_, b_body) = handle_route(&beta_root, &mut open, "/ingest/session", json!({}), Some(&t_beta)).unwrap();
        let b_sid = serde_json::from_str::<Value>(&b_body.unwrap()).unwrap()["sessionId"].as_str().unwrap().to_string();

        // 两租户各开一个 segment（即使 sessionId 相同也不会混）
        // 故意用同一段 id 串测隔离
        handle_route(&acme_root, &mut open, "/ingest/segment",
            json!({ "sessionId": &a_sid, "label": "web", "segmentId": "web#1", "startedAt": 1 }), Some(&t_acme)).unwrap();
        handle_route(&beta_root, &mut open, "/ingest/segment",
            json!({ "sessionId": &b_sid, "label": "web", "segmentId": "web#1", "startedAt": 1 }), Some(&t_beta)).unwrap();

        // end acme 的 session：只应关 acme 的 segment，beta 的不动
        handle_route(&acme_root, &mut open, "/ingest/session/end",
            json!({ "sessionId": &a_sid, "endedAt": 2 }), Some(&t_acme)).unwrap();

        // beta 的 open_segments 仍应有 1 个（acme 的 end 不影响 beta）
        let beta_key = seg_key(Some(&t_beta), &b_sid);
        assert_eq!(open.get(&beta_key).map(|v| v.len()), Some(1));
    }
}
