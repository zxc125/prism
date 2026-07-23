// Learn more about Tauri commands at https://tauri.app/develop/calling-rust-from-js/
use std::fs;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

mod ingest;

use tauri_plugin_observer::storage::recordings_root;

/// 由路由推导窗口 label：/settings -> settings，/player/abc -> player-abc，/ -> main。
/// 同路由 = 同 label = 单实例（聚焦已有）；路由带不同 :id = 多实例。
fn window_label(route: &str) -> String {
    let label = route.trim_start_matches('/').replace('/', "-");
    if label.is_empty() {
        "main".to_string()
    } else {
        label
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn open_window(app: AppHandle, route: String) -> Result<String, String> {
    let label = window_label(&route);

    // 已存在则显示+聚焦，避免重复创建
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.show();
        let _ = window.set_focus();
        // 复用已隐藏窗口：若录制中，插件 emit segment:start 驱动该窗口开新段
        tauri_plugin_observer::emit_segment_start_if_active(&app, &label);
        return Ok(label);
    }

    // 按路由首段决定标题与尺寸
    let segment = route.split('/').nth(1).unwrap_or("");
    let (title, width, height) = match segment {
        "settings" => ("设置", 480.0, 640.0),
        "player" => ("播放器", 960.0, 600.0),
        _ => ("RRWeb Demo", 800.0, 600.0),
    };
    // 不在 URL 里带 hash：WebviewUrl::App 接收的是 PathBuf，Windows WebView2 上
    // # 不会被识别为 URL fragment（macOS WKWebView 会），子窗口会因路径解析失败
    // 而白屏。改为加载纯 index.html，用 initialization_script 在 Vue router
    // 初始化前设置 hash -- 此脚本在页面任何脚本之前注入，跨平台一致。
    let init_script = format!(
        "if (!window.location.hash) window.location.replace('#{route}');"
    );
    WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("index.html".into()))
        .title(title)
        .inner_size(width, height)
        .initialization_script(&init_script)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(label)
}

#[tauri::command]
fn list_sessions(app: AppHandle) -> Result<Vec<Value>, String> {
    let root = recordings_root(&app);
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(&root) {
        for e in entries.flatten() {
            if let Ok(meta) = fs::read_to_string(e.path().join("session.json")) {
                if let Ok(v) = serde_json::from_str::<Value>(&meta) {
                    out.push(v);
                }
            }
        }
    }
    out.sort_by(|a, b| b["startedAt"].as_i64().cmp(&a["startedAt"].as_i64()));
    Ok(out)
}

/// 回放用：一次性返回会话元信息、窗口生命周期、各 segment 事件。
#[tauri::command]
fn read_session(app: AppHandle, id: String) -> Result<Value, String> {
    let dir = recordings_root(&app).join(&id);
    let session = serde_json::from_str::<Value>(
        &fs::read_to_string(dir.join("session.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let windows: Vec<Value> = fs::read_to_string(dir.join("windows.jsonl"))
        .ok()
        .and_then(|s| {
            s.lines()
                .map(|l| serde_json::from_str(l).ok())
                .collect()
        })
        .unwrap_or_default();

    let mut segments = serde_json::Map::new();
    let seg_dir = dir.join("segments");
    if let Ok(entries) = fs::read_dir(&seg_dir) {
        for e in entries.flatten() {
            let path = e.path();
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            let events: Vec<Value> = fs::read_to_string(&path)
                .ok()
                .and_then(|s| {
                    s.lines()
                        .map(|l| serde_json::from_str(l).ok())
                        .collect()
                })
                .unwrap_or_default();
            segments.insert(name, Value::Array(events));
        }
    }
    Ok(json!({ "session": session, "windows": windows, "segments": segments }))
}

#[tauri::command]
fn delete_session(app: AppHandle, id: String) -> Result<(), String> {
    let dir = recordings_root(&app).join(&id);
    fs::remove_dir_all(&dir).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // 录制协调逻辑已抽成 tauri-plugin-observer（Local 模式 = self-obs 落盘）。
        // skip_focus_prefix "player-" 与 console 原行为一致：回放窗不参与录制。
        .plugin(tauri_plugin_observer::init_with(
            tauri_plugin_observer::ObserverConfig {
                mode: tauri_plugin_observer::Mode::Local,
                skip_focus_prefix: "player-".into(),
                ..Default::default()
            },
        ))
        .manage(Mutex::new(ingest::IngestState::default()))
        .setup(|app| {
            // 载入持久化的接收配置，再启动 HTTP server（端口取自配置）
            let cfg = ingest::load_config(app.handle());
            {
                let state = app.state::<Mutex<ingest::IngestState>>();
                let mut s = state.lock().expect("ingest state poisoned");
                s.config = cfg;
            }
            ingest::start_server(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            open_window,
            list_sessions,
            read_session,
            delete_session,
            ingest::get_ingest_config,
            ingest::set_ingest_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use std::sync::Mutex;
