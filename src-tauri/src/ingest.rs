//! console 内嵌 HTTP 接收 server：[`observer_server`] 绑 127.0.0.1 的薄封装。
//!
//! P8 起，HTTP server 逻辑（ingest + 读 API + import）已抽到 [`observer_server`] crate，
//! 独立二进制（绑 0.0.0.0）与 console 内嵌（绑 127.0.0.1）共用同一份代码。本文件只保留：
//! - [`IngestConfig`] / [`IngestStatus`]：设置页用的配置结构 + 持久化
//! - [`IngestState`]：持有 [`ObserverServer`] 句柄
//! - `start_server` / `get_ingest_config` / `set_ingest_config`：启动 + 热更新
//!
//! 协议见 docs/架构/被观测侧（采集）.md。

use std::path::PathBuf;
use std::sync::Mutex;

use observer_server::{ObserverServer, ServerConfig};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

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

/// console 内嵌 server 状态：配置 + observer-server 句柄 + 数据目录（不变）。
pub struct IngestState {
    pub config: IngestConfig,
    pub server: ObserverServer,
    pub data_dir: PathBuf,
}

impl IngestState {
    pub fn new(config: IngestConfig, data_dir: PathBuf) -> Self {
        let server = ObserverServer::new(ServerConfig::local(
            config.port,
            data_dir.clone(),
            config.token.clone(),
            config.enabled,
        ));
        Self {
            config,
            server,
            data_dir,
        }
    }

    /// 把当前 config 同步到 observer-server（token/enabled 即时生效）。
    fn sync_server(&self) {
        self.server.update_config(ServerConfig::local(
            self.config.port,
            self.data_dir.clone(),
            self.config.token.clone(),
            self.config.enabled,
        ));
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

fn config_path(app: &AppHandle) -> PathBuf {
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

/// 启动 HTTP server 线程。绑定失败（端口占用）时 listening=false，不致命。
pub fn start_server(app: AppHandle) {
    let state = app.state::<Mutex<IngestState>>();
    let s = state.lock().expect("ingest state poisoned");
    s.server.clone().start();
}

#[tauri::command]
pub fn get_ingest_config(state: State<'_, Mutex<IngestState>>) -> IngestStatus {
    let s = state.lock().expect("ingest state poisoned");
    let status = s.server.status();
    IngestStatus {
        enabled: s.config.enabled,
        port: s.config.port,
        token: s.config.token.clone(),
        listening: status.listening,
        addr: status.addr,
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
        s.sync_server(); // token/enabled 即时生效；端口修改需重启生效
    }
    save_config(&app, &config)?;
    let s = state.lock().map_err(|e| e.to_string())?;
    let status = s.server.status();
    Ok(IngestStatus {
        enabled: s.config.enabled,
        port: s.config.port,
        token: s.config.token.clone(),
        listening: status.listening,
        addr: status.addr,
    })
}
