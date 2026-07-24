//! 自托管 HTTP server：ingest（/ingest/*）+ 读 API（/sessions/*）+ bundle 上传（/sessions/import）。
//!
//! 同一份代码服务两种部署（见 docs/阶段路径/P8-云端server抽取.md）：
//! - **console 内嵌**：绑 `127.0.0.1`，数据目录指向 `appDataDir/recordings`（[`ObserverServer`]）。
//! - **独立二进制**：绑 `0.0.0.0`，用户自托管（`src/bin/observer_server.rs`）。
//!
//! 鉴权：单一 Bearer token（P8 单租户；空 token = 不鉴权，本机回环 dev 友好）。
//! 多租户 / API key 映射留 P9。

mod routes;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tiny_http::{Header, Method, Request, Response, Server};

pub use routes::{handle_read_route, handle_route};

/// server 配置：bind 地址、数据目录（recordings 根）、鉴权 token、是否启用。
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    pub bind: String,
    pub data_dir: std::path::PathBuf,
    pub auth_token: String,
    pub enabled: bool,
}

impl ServerConfig {
    /// console 内嵌用：绑 127.0.0.1:port，数据目录指向 appDataDir/recordings。
    pub fn local(port: u16, data_dir: std::path::PathBuf, token: String, enabled: bool) -> Self {
        Self {
            bind: format!("127.0.0.1:{}", port),
            data_dir,
            auth_token: token,
            enabled,
        }
    }
}

/// 运行时状态快照（给设置页 / 监控用）。
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub enabled: bool,
    pub bind: String,
    pub auth_token: String,
    pub listening: bool,
    pub addr: Option<String>,
}

struct Inner {
    config: ServerConfig,
    open_segments: HashMap<String, Vec<String>>,
    listening: bool,
    addr: Option<String>,
}

/// server 句柄：clone 共享同一份状态，可在多线程更新配置 / 查状态。
#[derive(Clone)]
pub struct ObserverServer {
    inner: Arc<Mutex<Inner>>,
}

impl ObserverServer {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                config,
                open_segments: HashMap::new(),
                listening: false,
                addr: None,
            })),
        }
    }

    /// 启动 HTTP server 线程。绑定失败（端口占用）时 listening=false，不致命。
    pub fn start(&self) {
        let config = self.config_snapshot();
        let inner = self.inner.clone();
        std::thread::spawn(move || match Server::http(&config.bind) {
            Ok(server) => {
                {
                    let mut s = inner.lock().expect("server state poisoned");
                    s.listening = true;
                    s.addr = Some(config.bind.clone());
                }
                for req in server.incoming_requests() {
                    if let Err(e) = handle_request(&inner, req) {
                        eprintln!("[observer-server] {e}");
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "[observer-server] bind {} 失败: {}（端口占用？）",
                    config.bind, e
                );
            }
        });
    }

    pub fn config_snapshot(&self) -> ServerConfig {
        self.inner.lock().expect("server state poisoned").config.clone()
    }

    /// 热更新配置（token/enabled 即时生效；bind 变更需 restart）。
    pub fn update_config(&self, config: ServerConfig) {
        let mut s = self.inner.lock().expect("server state poisoned");
        s.config = config;
    }

    pub fn status(&self) -> ServerStatus {
        let s = self.inner.lock().expect("server state poisoned");
        ServerStatus {
            enabled: s.config.enabled,
            bind: s.config.bind.clone(),
            auth_token: s.config.auth_token.clone(),
            listening: s.listening,
            addr: s.addr.clone(),
        }
    }
}

fn err_json(msg: &str) -> String {
    serde_json::json!({ "error": msg }).to_string()
}

fn respond(req: Request, status: u16, body: Option<String>) -> std::io::Result<()> {
    let ct = Header::from_bytes("Content-Type", "application/json; charset=utf-8")
        .expect("static header");
    let origin =
        Header::from_bytes("Access-Control-Allow-Origin", "*").expect("static header");
    let resp = match body {
        Some(s) => Response::from_string(s),
        None => Response::from_string(String::new()),
    }
    .with_status_code(status)
    .with_header(ct)
    .with_header(origin);
    req.respond(resp)
}

fn json_body(req: &mut Request) -> Result<serde_json::Value, (u16, String)> {
    let mut body = String::new();
    req.as_reader()
        .read_to_string(&mut body)
        .map_err(|e| (400, format!("read body: {e}")))?;
    if body.is_empty() {
        return Ok(serde_json::Value::Null);
    }
    serde_json::from_str(&body).map_err(|e| (400, format!("invalid json: {e}")))
}

fn auth_ok(req: &Request, token: &str) -> bool {
    if token.is_empty() {
        return true; // 未设 token = 不鉴权（本机回环，dev 友好）
    }
    let expected = format!("Bearer {}", token);
    req.headers().iter().any(|h| {
        h.field.as_str().as_str().eq_ignore_ascii_case("authorization")
            && h.value.as_str() == expected.as_str()
    })
}

/// 处理单个请求（串行）。返回错误仅用于日志，响应已在内部发出。
fn handle_request(state: &Arc<Mutex<Inner>>, mut req: Request) -> Result<(), String> {
    // CORS 预检：web demo / console / 云端客户端跨端口，application/json 触发 preflight
    if req.method() == &Method::Options {
        let resp = Response::empty(204)
            .with_header(
                Header::from_bytes("Access-Control-Allow-Origin", "*").expect("static header"),
            )
            .with_header(
                Header::from_bytes("Access-Control-Allow-Methods", "GET, POST, PATCH, DELETE, OPTIONS")
                    .expect("static header"),
            )
            .with_header(
                Header::from_bytes("Access-Control-Allow-Headers", "Content-Type, Authorization")
                    .expect("static header"),
            );
        return req.respond(resp).map_err(|e| e.to_string());
    }

    let url = req.url().to_string();
    let method = req.method().as_str().to_string();

    // 读 config 快照做鉴权 / enabled 检查（clone 后即放锁，不阻塞 ingest 路由的 open_segments 锁）
    let config = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.config.clone()
    };
    if !config.enabled {
        respond(req, 503, Some(err_json("server disabled"))).map_err(|e| e.to_string())?;
        return Ok(());
    }
    if !auth_ok(&req, &config.auth_token) {
        respond(req, 401, Some(err_json("unauthorized"))).map_err(|e| e.to_string())?;
        return Ok(());
    }

    let body = json_body(&mut req).unwrap_or(serde_json::Value::Null);

    let (status, out) = if url.starts_with("/ingest/") {
        // ingest 路由需要 open_segments（session/end 据此补 hidden）
        let mut s = state.lock().map_err(|e| e.to_string())?;
        match handle_route(&config.data_dir, &mut s.open_segments, &url, body) {
            Ok(r) => r,
            Err((st, msg)) => (st, Some(err_json(&msg))),
        }
    } else if url.starts_with("/sessions") {
        // 读/管理路由：纯存储，无需 open_segments
        match handle_read_route(&config.data_dir, &method, &url, body) {
            Ok(r) => r,
            Err((st, msg)) => (st, Some(err_json(&msg))),
        }
    } else {
        (404, Some(err_json("unknown route")))
    };
    respond(req, status, out).map_err(|e| e.to_string())
}
