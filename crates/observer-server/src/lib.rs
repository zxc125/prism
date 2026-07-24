//! 自托管 HTTP server：ingest（/ingest/*）+ 读 API（/sessions/*）+ bundle 上传（/sessions/import）。
//!
//! 同一份代码服务两种部署（见 docs/阶段路径/P8-云端server抽取.md / P9-多租户与运营加固.md）：
//! - **console 内嵌**：绑 `127.0.0.1`，数据目录指向 `appDataDir/recordings`（隐式单租户）。
//! - **独立二进制**：绑 `0.0.0.0`，用户自托管；传 `--tenants-file` 启用多租户。
//!
//! 鉴权：单租户用 `auth_token`（空 = 不鉴权，本机回环 dev 友好）；多租户用 `tenants.json`
//! 里的 per-tenant API key（`Authorization: Bearer <key>`），映射 tenantId + appId 集合。
//!
//! P9 运营加固：session ID 校验（堵 read API 路径穿越）、gzip 响应、per-tenant 配额 + 限流、
//! 服务端 redact（import 入库前）、后台保留清扫线程。详见 docs/架构/P9-多租户运营加固（方案）.md。

mod quota;
mod rate_limit;
mod routes;
mod tenant;

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use tiny_http::{Header, Method, Request, Response, Server};

pub use quota::QuotaTracker;
pub use rate_limit::RateLimiter;
pub use routes::{handle_read_route, handle_route};
pub use tenant::{RateLimitConfig, TenantConfig, TenantRegistry};

use observer_storage::{enforce_retention, RetentionPolicy};

/// server 配置：bind 地址、数据目录、鉴权 token、是否启用、多租户配置文件、单租户保留策略。
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    pub bind: String,
    pub data_dir: PathBuf,
    pub auth_token: String,
    pub enabled: bool,
    /// 多租户配置文件路径。None = 隐式单租户（console 内嵌默认）。
    #[serde(default)]
    pub tenants_file: Option<PathBuf>,
    /// 单租户模式的保留策略（多租户时 per-tenant 覆盖）。
    #[serde(default)]
    pub retention: Option<RetentionPolicy>,
}

impl ServerConfig {
    /// console 内嵌用：绑 127.0.0.1:port，数据目录指向 appDataDir/recordings，单租户。
    pub fn local(port: u16, data_dir: PathBuf, token: String, enabled: bool) -> Self {
        Self {
            bind: format!("127.0.0.1:{}", port),
            data_dir,
            auth_token: token,
            enabled,
            tenants_file: None,
            retention: None,
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
    pub multi_tenant: bool,
}

struct Inner {
    config: ServerConfig,
    registry: Option<TenantRegistry>,
    quota: QuotaTracker,
    rate_limiter: RateLimiter,
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
        // 多租户：启动时加载 tenants.json，失败不致命（退化为单租户 + 日志）
        let registry = config.tenants_file.as_ref().and_then(|p| {
            match TenantRegistry::load(p) {
                Ok(r) => {
                    eprintln!(
                        "[observer-server] 多租户已启用：{}",
                        p.display()
                    );
                    Some(r)
                }
                Err(e) => {
                    eprintln!("[observer-server] 加载 tenants.json 失败（退化为单租户）: {e}");
                    None
                }
            }
        });
        Self {
            inner: Arc::new(Mutex::new(Inner {
                config,
                registry,
                quota: QuotaTracker::new(),
                rate_limiter: RateLimiter::new(60),
                open_segments: HashMap::new(),
                listening: false,
                addr: None,
            })),
        }
    }

    /// 启动 HTTP server 线程 + 保留清扫后台线程。绑定失败（端口占用）时 listening=false，不致命。
    pub fn start(&self) {
        let config = self.config_snapshot();
        let inner = self.inner.clone();
        // HTTP server 线程
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

        // 保留清扫线程：每 10 分钟跑一次（多租户 per-tenant / 单租户用 config.retention）
        let inner_sweep = self.inner.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(600));
            run_retention_sweep(&inner_sweep);
        });
    }

    pub fn config_snapshot(&self) -> ServerConfig {
        self.inner.lock().expect("server state poisoned").config.clone()
    }

    /// 热更新配置（token/enabled 即时生效；bind/tenants_file 变更需 restart）。
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
            multi_tenant: s.registry.is_some(),
        }
    }
}

/// 后台保留清扫：多租户遍历各 tenant root，单租户用 config.retention。
fn run_retention_sweep(state: &Arc<Mutex<Inner>>) {
    let (registry, data_dir, retention) = {
        let Ok(s) = state.lock() else {
            return;
        };
        (s.registry.clone(), s.config.data_dir.clone(), s.config.retention.clone())
    };
    if let Some(reg) = &registry {
        for t in reg.tenants() {
            if t.retention.is_empty() {
                continue;
            }
            let root = data_dir.join(&t.tenant_id);
            if root.exists() {
                if let Err(e) = enforce_retention(&root, &t.retention) {
                    eprintln!("[observer-server] 保留清扫 tenant={} 失败: {e}", t.tenant_id);
                }
            }
        }
    } else if let Some(policy) = retention {
        if !policy.is_empty() && data_dir.exists() {
            if let Err(e) = enforce_retention(&data_dir, &policy) {
                eprintln!("[observer-server] 保留清扫失败: {e}");
            }
        }
    }
}

fn err_json(msg: &str) -> String {
    serde_json::json!({ "error": msg }).to_string()
}

/// 响应：大 body（>1KB）且客户端 Accept-Encoding: gzip 时压缩，附 Content-Encoding 头。
fn respond(req: Request, status: u16, body: Option<String>, accept_gzip: bool) -> std::io::Result<()> {
    let ct = Header::from_bytes("Content-Type", "application/json; charset=utf-8").expect("static header");
    let origin = Header::from_bytes("Access-Control-Allow-Origin", "*").expect("static header");

    let (data, encoding): (Vec<u8>, Option<&'static str>) = match &body {
        Some(s) if accept_gzip && s.len() > 1024 => {
            let mut enc = GzEncoder::new(Vec::new(), Compression::default());
            enc.write_all(s.as_bytes())?;
            let gz = enc.finish()?;
            (gz, Some("gzip"))
        }
        Some(s) => (s.as_bytes().to_vec(), None),
        None => (Vec::new(), None),
    };

    let mut resp = Response::from_data(data)
        .with_status_code(status)
        .with_header(ct)
        .with_header(origin);
    if let Some(enc) = encoding {
        if let Ok(h) = Header::from_bytes("Content-Encoding", enc) {
            resp = resp.with_header(h);
        }
    }
    req.respond(resp)
}

/// 读请求体为 JSON Value，同时返回原始字节长度（供配额估算）。
fn read_body(req: &mut Request) -> (serde_json::Value, usize) {
    let mut body = String::new();
    if req.as_reader().read_to_string(&mut body).is_err() {
        return (serde_json::Value::Null, 0);
    }
    let len = body.len();
    if body.is_empty() {
        return (serde_json::Value::Null, len);
    }
    let v = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
    (v, len)
}

/// 单租户鉴权：token 为空 = 不鉴权；否则要求 `Authorization: Bearer <token>`。
fn auth_ok(req: &Request, token: &str) -> bool {
    if token.is_empty() {
        return true;
    }
    let expected = format!("Bearer {}", token);
    req.headers().iter().any(|h| {
        h.field.as_str().as_str().eq_ignore_ascii_case("authorization")
            && h.value.as_str() == expected.as_str()
    })
}

/// 从 Authorization 头提取 bearer key。
fn bearer_key(req: &Request) -> Option<String> {
    req.headers().iter().find_map(|h| {
        if h.field.as_str().as_str().eq_ignore_ascii_case("authorization") {
            h.value.as_str().strip_prefix("Bearer ").map(|s| s.to_string())
        } else {
            None
        }
    })
}

/// 解析租户：多租户走 registry.lookup(key)；单租户走 auth_token。返回 (tenant, Err(code,msg))。
fn resolve_tenant(
    config: &ServerConfig,
    registry: &Option<TenantRegistry>,
    req: &Request,
) -> Result<Option<TenantConfig>, (u16, String)> {
    match registry {
        Some(reg) => {
            let key = bearer_key(req).ok_or((401, "unauthorized: missing bearer key".into()))?;
            reg.lookup(&key)
                .map(Some)
                .ok_or((401, "unauthorized: invalid api key".into()))
        }
        None => {
            if !auth_ok(req, &config.auth_token) {
                return Err((401, "unauthorized".into()));
            }
            Ok(None)
        }
    }
}

/// tenant root：多租户 = data_dir/tenant_id；单租户 = data_dir。
fn tenant_root(data_dir: &Path, tenant: Option<&TenantConfig>) -> PathBuf {
    match tenant {
        Some(t) => data_dir.join(&t.tenant_id),
        None => data_dir.to_path_buf(),
    }
}

/// 处理单个请求（串行）。返回错误仅用于日志，响应已在内部发出。
fn handle_request(state: &Arc<Mutex<Inner>>, mut req: Request) -> Result<(), String> {
    // CORS 预检：web demo / console / 云端客户端跨端口，application/json 触发 preflight
    if req.method() == &Method::Options {
        let resp = Response::empty(204)
            .with_header(Header::from_bytes("Access-Control-Allow-Origin", "*").expect("static header"))
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
    let accept_gzip = req
        .headers()
        .iter()
        .any(|h| {
            h.field.as_str().as_str().eq_ignore_ascii_case("accept-encoding")
                && h.value.as_str().contains("gzip")
        });

    // 取配置 + 租户注册表 + quota/rate_limiter 句柄（clone 后即放锁）
    let (config, registry, quota, rate_limiter) = {
        let s = state.lock().map_err(|e| e.to_string())?;
        (
            s.config.clone(),
            s.registry.clone(),
            s.quota.clone(),
            s.rate_limiter.clone(),
        )
    };

    if !config.enabled {
        respond(req, 503, Some(err_json("server disabled")), false).map_err(|e| e.to_string())?;
        return Ok(());
    }

    // 解析租户
    let tenant = match resolve_tenant(&config, &registry, &req) {
        Ok(t) => t,
        Err((code, msg)) => {
            respond(req, code, Some(err_json(&msg)), false).map_err(|e| e.to_string())?;
            return Ok(());
        }
    };

    let root = tenant_root(&config.data_dir, tenant.as_ref());

    // 限流（多租户 per-tenant）
    if let Some(t) = &tenant {
        if let Some(rpm) = t.rate_limit.max_rpm {
            if !rate_limiter.check(&t.tenant_id, rpm) {
                respond(req, 429, Some(err_json("rate limit exceeded")), false).map_err(|e| e.to_string())?;
                return Ok(());
            }
        }
    }

    let (body, body_len) = read_body(&mut req);

    // 配额预检（/ingest/events）：按请求体字节估算（偏保守，安全）
    let is_ingest_events = url.starts_with("/ingest/events");
    if is_ingest_events {
        if let Some(t) = &tenant {
            if let Some(quota_bytes) = t.quota_bytes {
                if !quota.check(&t.tenant_id, &root, quota_bytes, body_len as u64) {
                    respond(req, 429, Some(err_json("quota exceeded")), false).map_err(|e| e.to_string())?;
                    return Ok(());
                }
            }
        }
    }

    let (status, out) = if url.starts_with("/ingest/") {
        // ingest 路由需要 open_segments（session/end 据此补 hidden）
        let mut s = state.lock().map_err(|e| e.to_string())?;
        match handle_route(&root, &mut s.open_segments, &url, body, tenant.as_ref()) {
            Ok(r) => r,
            Err((st, msg)) => (st, Some(err_json(&msg))),
        }
    } else if url.starts_with("/sessions") {
        match handle_read_route(&root, &method, &url, body, tenant.as_ref()) {
            Ok(r) => r,
            Err((st, msg)) => (st, Some(err_json(&msg))),
        }
    } else {
        (404, Some(err_json("unknown route")))
    };

    // 配额记账（仅 /ingest/events 写入成功后）
    if is_ingest_events && status == 204 {
        if let Some(t) = &tenant {
            if t.quota_bytes.is_some() {
                quota.add(&t.tenant_id, &root, body_len as u64);
            }
        }
    }

    respond(req, status, out, accept_gzip).map_err(|e| e.to_string())
}
