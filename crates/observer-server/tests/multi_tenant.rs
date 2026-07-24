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
