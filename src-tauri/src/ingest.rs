//! 本地 HTTP 接收 server：监听 127.0.0.1，接收外部 Web SDK / Tauri Plugin 上报，
//! 复用 storage 层落盘到同一 recordings/ 结构。仅本机回环，不对外。
//! 协议见 docs/架构/被观测侧（采集）.md。

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager, State};
use tiny_http::{Header, Method, Request, Response, Server};

use tauri_plugin_observer::storage::{
    append_events_file, append_lifecycle, create_session, finalize_session, now_ms, recordings_root,
};

const CONFIG_FILE: &str = "ingest-config.json";

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IngestConfig {
    pub enabled: bool,
    pub port: u16,
    pub token: String,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 1421,
            token: String::new(),
        }
    }
}

/// 运行时状态：配置 + server 绑定状态 + 各外部会话的活跃段（用于 session/end 补 hidden）。
pub struct IngestState {
    pub config: IngestConfig,
    pub listening: bool,
    pub addr: Option<String>,
    pub open_segments: HashMap<String, Vec<String>>,
}

impl Default for IngestState {
    fn default() -> Self {
        Self {
            config: IngestConfig::default(),
            listening: false,
            addr: None,
            open_segments: HashMap::new(),
        }
    }
}

/// 给设置页 / MainView 的状态快照（含运行时 listening/addr）。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestStatus {
    pub enabled: bool,
    pub port: u16,
    pub token: String,
    pub listening: bool,
    pub addr: Option<String>,
}

fn config_path(app: &AppHandle) -> std::path::PathBuf {
    app.path()
        .app_data_dir()
        .expect("app data dir")
        .join(CONFIG_FILE)
}

pub fn load_config(app: &AppHandle) -> IngestConfig {
    match std::fs::read_to_string(config_path(app)) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => IngestConfig::default(),
    }
}

pub fn save_config(app: &AppHandle, cfg: &IngestConfig) -> Result<(), String> {
    let s = serde_json::to_string(cfg).map_err(|e| e.to_string())?;
    std::fs::write(config_path(app), s).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_ingest_config(state: State<'_, Mutex<IngestState>>) -> IngestStatus {
    let s = state.lock().expect("ingest state poisoned");
    IngestStatus {
        enabled: s.config.enabled,
        port: s.config.port,
        token: s.config.token.clone(),
        listening: s.listening,
        addr: s.addr.clone(),
    }
}

#[tauri::command]
pub fn set_ingest_config(
    app: AppHandle,
    state: State<'_, Mutex<IngestState>>,
    config: IngestConfig,
) -> Result<IngestStatus, String> {
    {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.config = config.clone();
        // 端口修改需重启生效；token / enabled 即时生效
    }
    save_config(&app, &config)?;
    let s = state.lock().map_err(|e| e.to_string())?;
    Ok(IngestStatus {
        enabled: s.config.enabled,
        port: s.config.port,
        token: s.config.token.clone(),
        listening: s.listening,
        addr: s.addr.clone(),
    })
}

/// 启动 HTTP server 线程。绑定失败（端口占用）时 listening=false，不致命。
pub fn start_server(app: AppHandle) {
    let port = {
        let state = app.state::<Mutex<IngestState>>();
        let s = state.lock().expect("ingest state poisoned");
        s.config.port
    };
    let addr = format!("127.0.0.1:{}", port);
    let app2 = app.clone();
    std::thread::spawn(move || match Server::http(&addr) {
        Ok(server) => {
            {
                let state = app2.state::<Mutex<IngestState>>();
                let mut s = state.lock().expect("ingest state poisoned");
                s.listening = true;
                s.addr = Some(addr.clone());
            }
            for req in server.incoming_requests() {
                if let Err(e) = handle(&app2, req) {
                    eprintln!("[ingest] {e}");
                }
            }
        }
        Err(e) => {
            eprintln!(
                "[ingest] bind 127.0.0.1:{} 失败: {}（端口占用？改设置后重启）",
                port, e
            );
        }
    });
}

fn err_json(msg: &str) -> String {
    json!({ "error": msg }).to_string()
}

/// 400 错误构造助手（route 的错误类型为 (u16, String)）。
fn bad(msg: &str) -> (u16, String) {
    (400, msg.to_string())
}

fn respond(req: Request, status: u16, body: Option<String>) -> std::io::Result<()> {
    let ct = Header::from_bytes("Content-Type", "application/json; charset=utf-8")
        .expect("static header");
    let origin =
        Header::from_bytes("Access-Control-Allow-Origin", "*").expect("static header");
    // 两个分支统一成 Response<Box<dyn Read + Send>>：empty 的 reader 类型不同，不能并列
    let resp = match body {
        Some(s) => Response::from_string(s),
        None => Response::from_string(String::new()),
    }
    .with_status_code(status)
    .with_header(ct)
    .with_header(origin);
    req.respond(resp)
}

fn json_body(req: &mut Request) -> Result<Value, (u16, String)> {
    let mut body = String::new();
    req.as_reader()
        .read_to_string(&mut body)
        .map_err(|e| (400, format!("read body: {e}")))?;
    if body.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&body).map_err(|e| (400, format!("invalid json: {e}")))
}

fn auth_ok(req: &Request, token: &str) -> bool {
    if token.is_empty() {
        return true; // 未设 token = 不鉴权（本机回环，dev 友好）
    }
    let expected = format!("Bearer {}", token);
    req.headers().iter().any(|h| {
        // HeaderField.as_str() -> &AsciiStr，再 as_str() -> &str
        h.field.as_str().as_str().eq_ignore_ascii_case("authorization")
            && h.value.as_str() == expected.as_str()
    })
}

/// 处理单个请求（串行）。返回错误仅用于日志，响应已在内部发出。
fn handle(app: &AppHandle, mut req: Request) -> Result<(), String> {
    // CORS 预检：web demo 与 console 跨端口，application/json 触发 preflight
    if req.method() == &Method::Options {
        let resp = Response::empty(204)
            .with_header(
                Header::from_bytes("Access-Control-Allow-Origin", "*").expect("static header"),
            )
            .with_header(
                Header::from_bytes("Access-Control-Allow-Methods", "POST, OPTIONS")
                    .expect("static header"),
            )
            .with_header(
                Header::from_bytes("Access-Control-Allow-Headers", "Content-Type, Authorization")
                    .expect("static header"),
            );
        return req.respond(resp).map_err(|e| e.to_string());
    }
    if req.method() != &Method::Post {
        respond(req, 405, Some(err_json("method not allowed"))).map_err(|e| e.to_string())?;
        return Ok(());
    }

    let (enabled, token) = {
        let state = app.state::<Mutex<IngestState>>();
        let s = state.lock().map_err(|e| e.to_string())?;
        (s.config.enabled, s.config.token.clone())
    };
    if !enabled {
        respond(req, 503, Some(err_json("ingest disabled"))).map_err(|e| e.to_string())?;
        return Ok(());
    }
    if !auth_ok(&req, &token) {
        respond(req, 401, Some(err_json("unauthorized"))).map_err(|e| e.to_string())?;
        return Ok(());
    }

    let url = req.url().to_string();
    let body = match json_body(&mut req) {
        Ok(v) => v,
        Err((st, msg)) => {
            respond(req, st, Some(err_json(&msg))).map_err(|e| e.to_string())?;
            return Ok(());
        }
    };

    let (status, out) = match route(app, &url, body) {
        Ok(r) => r,
        Err((st, msg)) => (st, Some(err_json(&msg))),
    };
    respond(req, status, out).map_err(|e| e.to_string())
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

/// 路由 /ingest/* -> 落盘。返回 (status, 可选 JSON 体)。
fn route(
    app: &AppHandle,
    url: &str,
    body: Value,
) -> Result<(u16, Option<String>), (u16, String)> {
    let root = recordings_root(app);
    let state = app.state::<Mutex<IngestState>>();
    let mut s = state.lock().expect("ingest state poisoned");
    handle_route(&root, &mut s.open_segments, url, body)
}

/// 纯存储路由（不依赖 AppHandle，便于测试）：root 即 recordings 目录，
/// open_segments 由调用方持有（session/end 据此补 hidden）。
fn handle_route(
    root: &Path,
    open_segments: &mut HashMap<String, Vec<String>>,
    url: &str,
    body: Value,
) -> Result<(u16, Option<String>), (u16, String)> {
    match url {
        "/ingest/session" => {
            let id = unique_session_id(&root);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn read_jsonl(path: &std::path::Path) -> Vec<Value> {
        fs::read_to_string(path)
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
            serde_json::from_str(&fs::read_to_string(sdir.join("session.json")).unwrap()).unwrap();
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
}

