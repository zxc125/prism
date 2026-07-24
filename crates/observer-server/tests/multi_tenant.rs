//! 多租户端到端冒烟：启动真实 ObserverServer，走 HTTP 验证 P9 接线。
//!
//! 覆盖 handle_request 的胶水层（bearer 解析、租户隔离、appId 越权、session_id 校验、
//! 限流、gzip 响应），这些在 routes.rs 单测里无法覆盖（那里直接调 handle_route，绕过 HTTP 层）。

use observer_server::{ObserverServer, ServerConfig};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use tempfile::tempdir;

/// 找一个空闲端口（bind :0 取端口后立即关闭，有轻微竞态但测试够用）。
fn free_port() -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    l.local_addr().unwrap().port()
}

/// 发一个原始 HTTP 请求，返回 (status, body)。不送 Accept-Encoding -> 响应不压缩。
fn http(port: u16, method: &str, path: &str, body: Option<&str>, auth: Option<&str>) -> (u16, String) {
    // 重试几次：server 线程可能还没 ready
    for attempt in 0..20 {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(mut stream) => {
                let body = body.unwrap_or("");
                let mut req = format!(
                    "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
                    body.len()
                );
                if let Some(a) = auth {
                    req.push_str(&format!("Authorization: Bearer {a}\r\n"));
                }
                req.push_str("Connection: close\r\n\r\n");
                req.push_str(body);
                stream.write_all(req.as_bytes()).unwrap();
                let mut resp = String::new();
                stream.read_to_string(&mut resp).unwrap();
                let status: u16 = resp
                    .lines()
                    .next()
                    .unwrap()
                    .split(' ')
                    .nth(1)
                    .unwrap()
                    .parse()
                    .unwrap();
                let body = resp.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
                return (status, body);
            }
            Err(_) => std::thread::sleep(std::time::Duration::from_millis(50 * (attempt + 1))),
        }
    }
    panic!("connect to 127.0.0.1:{port} failed after retries");
}

fn start_server(data_dir: &PathBuf, tenants_file: Option<PathBuf>) -> u16 {
    let port = free_port();
    let config = ServerConfig {
        bind: format!("127.0.0.1:{port}"),
        data_dir: data_dir.clone(),
        auth_token: String::new(),
        enabled: true,
        tenants_file,
        retention: None,
        web_dir: None,
    };
    let server = ObserverServer::new(config);
    server.start();
    port
}

fn write_tenants(path: &std::path::Path, body: &str) {
    std::fs::write(path, body).unwrap();
}

#[test]
fn multi_tenant_isolation_and_authz() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("recordings");
    std::fs::create_dir_all(&data_dir).unwrap();
    let tenants_path = dir.path().join("tenants.json");
    write_tenants(
        &tenants_path,
        r#"[
          { "key": "sk_acme", "tenantId": "acme", "appIds": ["shop-web"] },
          { "key": "sk_beta", "tenantId": "beta", "appIds": [] }
        ]"#,
    );
    let port = start_server(&data_dir, Some(tenants_path));

    // 1. 无 key -> 401
    let (st, _) = http(port, "GET", "/sessions", None, None);
    assert_eq!(st, 401);

    // 2. 错 key -> 401
    let (st, _) = http(port, "GET", "/sessions", None, Some("sk_unknown"));
    assert_eq!(st, 401);

    // 3. acme 越权 appId -> 403
    let (st, _) = http(
        port,
        "POST",
        "/ingest/session",
        Some(r#"{"appId":"other-app"}"#),
        Some("sk_acme"),
    );
    assert_eq!(st, 403);

    // 4. acme 合法 appId 建会话 -> 200
    let (st, body) = http(
        port,
        "POST",
        "/ingest/session",
        Some(r#"{"appId":"shop-web","source":"web"}"#),
        Some("sk_acme"),
    );
    assert_eq!(st, 200);
    let acme_sid: serde_json::Value = serde_json::from_str(&body).unwrap();
    let acme_sid = acme_sid["sessionId"].as_str().unwrap().to_string();

    // 5. beta list -> 空（看不到 acme 的会话）
    let (st, body) = http(port, "GET", "/sessions", None, Some("sk_beta"));
    assert_eq!(st, 200);
    let list: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 0);

    // 6. acme list -> 1 条
    let (st, body) = http(port, "GET", "/sessions", None, Some("sk_acme"));
    assert_eq!(st, 200);
    let list: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["id"], acme_sid);
    assert_eq!(list[0]["tenantId"], "acme");

    // 7. beta 不能读 acme 的会话（不同 tenant root，找不到 -> 404）
    let (st, _) = http(port, "GET", &format!("/sessions/{acme_sid}"), None, Some("sk_beta"));
    assert_eq!(st, 404);

    // 8. 路径穿越 -> 400
    let (st, _) = http(port, "GET", "/sessions/..", None, Some("sk_acme"));
    assert_eq!(st, 400);
    let (st, _) = http(port, "GET", "/sessions/abc", None, Some("sk_acme"));
    assert_eq!(st, 400);
}

#[test]
fn gzip_response_when_accept_encoding() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("recordings");
    std::fs::create_dir_all(&data_dir).unwrap();
    let port = start_server(&data_dir, None);

    // 建足够多会话让 list body > 1KB（触发 gzip 阈值）
    for _ in 0..200 {
        let _ = http(port, "POST", "/ingest/session", Some(r#"{}"#), None);
    }
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .write_all(b"GET /sessions HTTP/1.1\r\nHost: 127.0.0.1\r\nAccept-Encoding: gzip\r\nConnection: close\r\n\r\n")
        .unwrap();
    let mut resp = Vec::new();
    stream.read_to_end(&mut resp).unwrap();
    let resp_str = String::from_utf8_lossy(&resp);
    let status: u16 = resp_str
        .lines()
        .next()
        .unwrap()
        .split(' ')
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(status, 200);
    assert!(
        resp_str.contains("Content-Encoding: gzip"),
        "大 body + Accept-Encoding 应 gzip: {}",
        &resp_str[..resp_str.len().min(200)]
    );
}

#[test]
fn rate_limit_returns_429() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("recordings");
    std::fs::create_dir_all(&data_dir).unwrap();
    let tenants_path = dir.path().join("tenants.json");
    write_tenants(
        &tenants_path,
        r#"[
          { "key": "sk_limited", "tenantId": "limited", "appIds": [], "rateLimit": { "maxRpm": 3 } }
        ]"#,
    );
    let port = start_server(&data_dir, Some(tenants_path));

    // 前 3 次通过
    for _ in 0..3 {
        let (st, _) = http(port, "GET", "/sessions", None, Some("sk_limited"));
        assert_eq!(st, 200);
    }
    // 第 4 次超限
    let (st, body) = http(port, "GET", "/sessions", None, Some("sk_limited"));
    assert_eq!(st, 429);
    assert!(body.contains("rate limit"));
}

#[test]
fn quota_returns_429_on_exceed() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("recordings");
    std::fs::create_dir_all(&data_dir).unwrap();
    let tenants_path = dir.path().join("tenants.json");
    // 配额 1 字节：任何写入都超
    write_tenants(
        &tenants_path,
        r#"[
          { "key": "sk_small", "tenantId": "small", "appIds": [], "quotaBytes": 1 }
        ]"#,
    );
    let port = start_server(&data_dir, Some(tenants_path));

    // 先建会话（建会话本身不走 /ingest/events 配额检查）
    let (_, body) = http(port, "POST", "/ingest/session", Some(r#"{}"#), Some("sk_small"));
    let sid: serde_json::Value = serde_json::from_str(&body).unwrap();
    let sid = sid["sessionId"].as_str().unwrap().to_string();

    // ingest events 必超配额 -> 429
    let (st, resp) = http(
        port,
        "POST",
        "/ingest/events",
        Some(&format!(
            r#"{{"sessionId":"{}","segmentId":"web#1","events":[{{"type":2,"timestamp":1}}]}}"#,
            sid
        )),
        Some("sk_small"),
    );
    assert_eq!(st, 429);
    assert!(resp.contains("quota"));
}

#[test]
fn single_tenant_backward_compat() {
    // 无 tenants_file：单租户 + auth_token，P8 行为不变。
    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("recordings");
    std::fs::create_dir_all(&data_dir).unwrap();
    let port = free_port();
    let config = ServerConfig {
        bind: format!("127.0.0.1:{port}"),
        data_dir: data_dir.clone(),
        auth_token: "secret".into(),
        enabled: true,
        tenants_file: None,
        retention: None,
        web_dir: None,
    };
    ObserverServer::new(config).start();

    // 无 token -> 401
    let (st, _) = http(port, "GET", "/sessions", None, None);
    assert_eq!(st, 401);
    // 错 token -> 401
    let (st, _) = http(port, "GET", "/sessions", None, Some("wrong"));
    assert_eq!(st, 401);
    // 对 token -> 200
    let (st, _) = http(port, "GET", "/sessions", None, Some("secret"));
    assert_eq!(st, 200);
}

// ---- P10：/whoami + 静态文件服务 ----

/// `GET /whoami`：多租户返回 tenant 信息 + usageBytes；单租户返回 { multiTenant: false }。
#[test]
fn whoami_returns_tenant_context() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("recordings");
    std::fs::create_dir_all(&data_dir).unwrap();
    let tenants_path = dir.path().join("tenants.json");
    write_tenants(
        &tenants_path,
        r#"[
          { "key": "sk_acme", "tenantId": "acme", "appIds": ["shop-web"], "quotaBytes": 1000000 }
        ]"#,
    );
    let port = start_server(&data_dir, Some(tenants_path));

    // 无 key -> 401
    let (st, _) = http(port, "GET", "/whoami", None, None);
    assert_eq!(st, 401);

    // acme -> multiTenant:true + tenantId + appIds + usageBytes
    let (st, body) = http(port, "GET", "/whoami", None, Some("sk_acme"));
    assert_eq!(st, 200);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["multiTenant"], true);
    assert_eq!(v["tenantId"], "acme");
    assert_eq!(v["appIds"][0], "shop-web");
    assert_eq!(v["quotaBytes"], 1000000);
    assert!(v["usageBytes"].is_number(), "usageBytes 应为数字");
}

#[test]
fn whoami_single_tenant_returns_false() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("recordings");
    std::fs::create_dir_all(&data_dir).unwrap();
    let port = free_port();
    let config = ServerConfig {
        bind: format!("127.0.0.1:{port}"),
        data_dir: data_dir.clone(),
        auth_token: "secret".into(),
        enabled: true,
        tenants_file: None,
        retention: None,
        web_dir: None,
    };
    ObserverServer::new(config).start();

    // 无 token -> 401
    let (st, _) = http(port, "GET", "/whoami", None, None);
    assert_eq!(st, 401);

    // 对 token -> multiTenant:false
    let (st, body) = http(port, "GET", "/whoami", None, Some("secret"));
    assert_eq!(st, 200);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["multiTenant"], false);
}

/// 静态文件服务：启用 web_dir 后 `GET /` 返回 index.html，`/assets/*` 返回文件 + 正确 MIME；
/// API 路由（/ingest /sessions /whoami）不被静态拦截；`..` 路径穿越被拒。
#[test]
fn static_file_serving_and_path_traversal_protection() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("recordings");
    std::fs::create_dir_all(&data_dir).unwrap();
    let web_dir = dir.path().join("web");
    let assets_dir = web_dir.join("assets");
    std::fs::create_dir_all(&assets_dir).unwrap();
    // index.html + assets/app.js + assets/style.css
    std::fs::write(web_dir.join("index.html"), "<!doctype html><html><body>console</body></html>").unwrap();
    std::fs::write(assets_dir.join("app.js"), "console.log('hi')").unwrap();
    std::fs::write(assets_dir.join("style.css"), "body{color:red}").unwrap();

    let port = free_port();
    let config = ServerConfig {
        bind: format!("127.0.0.1:{port}"),
        data_dir: data_dir.clone(),
        auth_token: String::new(),
        enabled: true,
        tenants_file: None,
        retention: None,
        web_dir: Some(web_dir),
    };
    ObserverServer::new(config).start();

    // GET / -> index.html + text/html
    let resp = http_raw(port, "GET", "/", None, None);
    assert_eq!(resp.status, 200);
    assert!(resp.body.contains("console"));
    assert!(resp.content_type.contains("text/html"), "got: {}", resp.content_type);

    // GET /assets/app.js -> JS MIME
    let resp = http_raw(port, "GET", "/assets/app.js", None, None);
    assert_eq!(resp.status, 200);
    assert!(resp.body.contains("console.log"));
    assert!(resp.content_type.contains("javascript"), "got: {}", resp.content_type);

    // GET /assets/style.css -> CSS MIME
    let resp = http_raw(port, "GET", "/assets/style.css", None, None);
    assert_eq!(resp.status, 200);
    assert!(resp.content_type.contains("text/css"), "got: {}", resp.content_type);

    // SPA fallback：未知路径（非 API 路由）回 index.html（hash 路由客户端解析）
    // 注意：/sessions/* 会命中 read API，浏览器实际用 hash 路由（path 永远是 /），
    // 所以这里测一个不与任何 API 冲突的路径。
    let resp = http_raw(port, "GET", "/some-unknown-page", None, None);
    assert_eq!(resp.status, 200);
    assert!(resp.body.contains("console"));

    // 路径穿越 -> 403
    let resp = http_raw(port, "GET", "/../recordings", None, None);
    // tiny_http 可能在客户端层就拒绝 ../（规范化），但 server 侧也应拒绝
    assert!(
        resp.status == 403 || resp.status == 404,
        "穿越应被拒，got: {}",
        resp.status
    );

    // API 路由不被静态拦截：/sessions 仍返回 JSON list（这里无 token、空 list）
    let (st, body) = http(port, "GET", "/sessions", None, None);
    assert_eq!(st, 200);
    assert_eq!(body, "[]");
}

/// P10 关键场景：多租户模式下静态文件服务必须在鉴权之前--浏览器加载登录页 + JS/CSS
/// 资产时尚无 bearer key。若静态 fallback 走在 tenant 解析之后，会返回 401 拦截登录页。
#[test]
fn static_serving_bypasses_auth_in_multi_tenant_mode() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("recordings");
    std::fs::create_dir_all(&data_dir).unwrap();
    let tenants_path = dir.path().join("tenants.json");
    write_tenants(
        &tenants_path,
        r#"[
          { "key": "sk_acme", "tenantId": "acme", "appIds": ["shop-web"] }
        ]"#,
    );
    let web_dir = dir.path().join("web");
    let assets_dir = web_dir.join("assets");
    std::fs::create_dir_all(&assets_dir).unwrap();
    std::fs::write(web_dir.join("index.html"), "<!doctype html><html><body>login</body></html>").unwrap();
    std::fs::write(assets_dir.join("app.js"), "console.log(1)").unwrap();

    let port = free_port();
    let config = ServerConfig {
        bind: format!("127.0.0.1:{port}"),
        data_dir: data_dir.clone(),
        auth_token: String::new(),
        enabled: true,
        tenants_file: Some(tenants_path),
        retention: None,
        web_dir: Some(web_dir),
    };
    ObserverServer::new(config).start();

    // 无 key 访问 / -> 200 + index.html（不是 401 unauthorized）
    let resp = http_raw(port, "GET", "/", None, None);
    assert_eq!(resp.status, 200, "多租户模式下无 key 访问 / 应返回静态页，不是 401");
    assert!(resp.body.contains("login"), "应返回 index.html 内容");
    assert!(resp.content_type.contains("text/html"), "got: {}", resp.content_type);

    // 无 key 访问 /assets/app.js -> 200 + JS MIME
    let resp = http_raw(port, "GET", "/assets/app.js", None, None);
    assert_eq!(resp.status, 200, "静态资产不应被鉴权拦截");
    assert!(resp.content_type.contains("javascript"), "got: {}", resp.content_type);

    // 但 API 路由仍需鉴权：/sessions 无 key -> 401
    let (st, body) = http(port, "GET", "/sessions", None, None);
    assert_eq!(st, 401, "API 路由仍需鉴权");
    assert!(body.contains("unauthorized"), "应返回 401 unauthorized");

    // /whoami 无 key -> 401
    let (st, _) = http(port, "GET", "/whoami", None, None);
    assert_eq!(st, 401, "/whoami 仍需鉴权");
}

/// 含完整响应头解析的 HTTP 请求，便于断言 Content-Type。
struct RawResp {
    status: u16,
    body: String,
    content_type: String,
}

fn http_raw(port: u16, method: &str, path: &str, body: Option<&str>, auth: Option<&str>) -> RawResp {
    for attempt in 0..20 {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(mut stream) => {
                let body = body.unwrap_or("");
                let mut req = format!(
                    "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
                    body.len()
                );
                if let Some(a) = auth {
                    req.push_str(&format!("Authorization: Bearer {a}\r\n"));
                }
                req.push_str("Connection: close\r\n\r\n");
                req.push_str(body);
                stream.write_all(req.as_bytes()).unwrap();
                let mut resp = Vec::new();
                use std::io::Read;
                stream.read_to_end(&mut resp).unwrap();
                let resp_str = String::from_utf8_lossy(&resp).to_string();
                let status: u16 = resp_str
                    .lines()
                    .next()
                    .unwrap()
                    .split(' ')
                    .nth(1)
                    .unwrap()
                    .parse()
                    .unwrap();
                let ct = resp_str
                    .lines()
                    .find(|l| l.to_ascii_lowercase().starts_with("content-type:"))
                    .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
                    .unwrap_or_default();
                let body = resp_str.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
                return RawResp { status, body, content_type: ct };
            }
            Err(_) => std::thread::sleep(std::time::Duration::from_millis(50 * (attempt + 1))),
        }
    }
    panic!("connect to 127.0.0.1:{port} failed after retries");
}
