//! observer-server 独立二进制：自托管私有云 server。
//!
//! 用法：
//! ```sh
//! # 单租户（P8 兼容）
//! observer-server --bind 0.0.0.0:8080 --data-dir ./recordings --token <api-key>
//!
//! # 多租户（P9）
//! observer-server --bind 0.0.0.0:8080 --data-dir ./recordings --tenants-file ./tenants.json
//! ```
//!
//! 也可用环境变量：`OBSERVER_BIND` / `OBSERVER_DATA_DIR` / `OBSERVER_TOKEN` /
//! `OBSERVER_TENANTS_FILE`。TLS 建议由反代（nginx/caddy）终止，server 本身只跑 HTTP。

use std::path::PathBuf;

use observer_server::{ObserverServer, ServerConfig};

fn main() {
    let bind = arg_or_env("bind", "OBSERVER_BIND", "127.0.0.1:8080");
    let data_dir = PathBuf::from(arg_or_env("data-dir", "OBSERVER_DATA_DIR", "recordings"));
    let token = arg_or_env("token", "OBSERVER_TOKEN", "");
    let tenants_file = arg_or_env_opt("tenants-file", "OBSERVER_TENANTS_FILE");

    if std::env::args().any(|a| a == "--help" || a == "-h") {
        eprintln!("observer-server - 自托管前端观测 server");
        eprintln!();
        eprintln!("用法: observer-server [OPTIONS]");
        eprintln!();
        eprintln!("选项:");
        eprintln!("  --bind <addr>           绑定地址（默认 127.0.0.1:8080，对外用 0.0.0.0:8080）");
        eprintln!("  --data-dir <path>       会话存储目录（默认 ./recordings）");
        eprintln!("  --token <key>           单租户 API key（Authorization: Bearer <key>，留空 = 不鉴权）");
        eprintln!("  --tenants-file <path>   多租户配置文件（启用多租户模式，覆盖 --token）");
        eprintln!("  -h, --help              显示帮助");
        eprintln!();
        eprintln!("环境变量: OBSERVER_BIND / OBSERVER_DATA_DIR / OBSERVER_TOKEN / OBSERVER_TENANTS_FILE");
        eprintln!("TLS 建议由反代（nginx/caddy）终止，server 本身只跑 HTTP。");
        return;
    }

    // 确保数据目录存在
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        eprintln!("[observer-server] 创建数据目录失败: {e}");
        std::process::exit(1);
    }

    let config = ServerConfig {
        bind: bind.clone(),
        data_dir,
        auth_token: token.clone(),
        enabled: true,
        tenants_file: tenants_file.map(PathBuf::from),
        retention: None,
    };

    eprintln!("[observer-server] 绑定 {bind}");
    if config.tenants_file.is_some() {
        eprintln!("[observer-server] 模式: 多租户（tenants.json）");
    } else if config.auth_token.is_empty() {
        eprintln!("[observer-server] 模式: 单租户，鉴权关闭（无 token）");
    } else {
        eprintln!("[observer-server] 模式: 单租户，鉴权开启（Bearer token）");
    }

    let server = ObserverServer::new(config);
    server.start();

    // 阻塞主线程：tiny_http 在独立线程里跑 incoming_requests
    // 接收 Ctrl-C 退出（loop 休眠避免忙等）
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

/// 解析 `--key value`，缺失则取环境变量，再缺失用默认值。
fn arg_or_env(key: &str, env: &str, default: &str) -> String {
    let flag = format!("--{key}");
    let mut args = std::env::args().collect::<Vec<_>>().into_iter();
    while let Some(a) = args.next() {
        if a == flag {
            if let Some(v) = args.next() {
                return v;
            }
        }
    }
    std::env::var(env).unwrap_or_else(|_| default.to_string())
}

/// 同 [`arg_or_env`] 但无默认值，返回 Option。
fn arg_or_env_opt(key: &str, env: &str) -> Option<String> {
    let flag = format!("--{key}");
    let mut args = std::env::args().collect::<Vec<_>>().into_iter();
    while let Some(a) = args.next() {
        if a == flag {
            if let Some(v) = args.next() {
                return Some(v);
            }
        }
    }
    std::env::var(env).ok()
}
