// Learn more about Tauri commands at https://tauri.app/develop/calling-rust-from-js/
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde_json::{json, Value};
use tauri::{Emitter, EventTarget, Manager, WebviewUrl, WebviewWindowBuilder, Window, WindowEvent};

mod ingest;
mod storage;

use storage::{append_events_file, append_lifecycle, now_ms, recordings_root};

/// 录制会话状态。active 期间所有窗口的 rrweb 事件按 segment 落盘。
#[derive(Default)]
struct Session {
    id: Option<String>,
    started_at: i64,
    dir: Option<PathBuf>,
    active: bool,
    /// label -> 下一段序号，用于生成 segmentId `<label>#<n>`
    segment_seq: HashMap<String, u64>,
    /// label -> 当前活跃 segmentId（hide 时据此记 hidden）
    current: HashMap<String, String>,
}

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
fn open_window(
    app: tauri::AppHandle,
    state: tauri::State<Mutex<Session>>,
    route: String,
) -> Result<String, String> {
    let label = window_label(&route);

    // 已存在则显示+聚焦，避免重复创建
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.show();
        let _ = window.set_focus();
        // 复用已隐藏窗口：若录制中，通知该窗口开新段
        let active = state.lock().map(|s| s.active).unwrap_or(false);
        if active {
            let _ = app.emit_to(
                EventTarget::labeled(&label),
                "segment",
                json!({ "action": "start" }),
            );
        }
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
fn start_session(app: tauri::AppHandle, state: tauri::State<Mutex<Session>>) -> Result<String, String> {
    let id = format!("{}", now_ms());
    let dir = recordings_root(&app).join(&id);
    fs::create_dir_all(dir.join("segments")).map_err(|e| e.to_string())?;
    let started_at = now_ms();
    {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.id = Some(id.clone());
        s.started_at = started_at;
        s.dir = Some(dir.clone());
        s.active = true;
        s.segment_seq.clear();
        s.current.clear();
    }
    fs::write(
        dir.join("session.json"),
        json!({ "id": id, "source": "self", "startedAt": started_at }).to_string(),
    )
    .map_err(|e| e.to_string())?;
    // 补记初始 focus：会话开始时当前聚焦窗口，避免 focus 时间线在 t=0 为空
    for (label, w) in app.webview_windows() {
        if !label.starts_with("player-") && w.is_focused().unwrap_or(false) {
            let _ = append_lifecycle(
                &dir,
                json!({ "type": "focus", "label": label, "t": now_ms() }),
            );
            break;
        }
    }
    // 广播：已挂载的各窗口收到后各自 begin_segment
    app.emit("recording-session", json!({ "active": true, "id": id }))
        .map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn stop_session(app: tauri::AppHandle, state: tauri::State<Mutex<Session>>) -> Result<(), String> {
    let (dir, id, started_at) = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.active = false;
        // 关闭所有仍活跃的段（先 drain 出来，避免与 dir 借用冲突）
        let open_segs: Vec<(String, String)> = s.current.drain().collect();
        if let Some(dir) = &s.dir {
            let now = now_ms();
            for (label, seg) in open_segs {
                let _ = append_lifecycle(
                    dir,
                    json!({ "type": "hidden", "label": label, "segmentId": seg, "t": now }),
                );
            }
        }
        (s.dir.clone(), s.id.clone(), s.started_at)
    };
    if let Some(dir) = dir {
        let ended_at = now_ms();
        fs::write(
            dir.join("session.json"),
            json!({ "id": id, "source": "self", "startedAt": started_at, "endedAt": ended_at })
                .to_string(),
        )
        .map_err(|e| e.to_string())?;
    }
    app.emit("recording-session", json!({ "active": false }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn is_recording_active(state: tauri::State<Mutex<Session>>) -> bool {
    state.lock().map(|s| s.active).unwrap_or(false)
}

/// 由窗口挂载时调用（首次创建）或收到 segment:start 事件后调用（复用显示）。
/// 分配 segmentId、记 shown 生命周期、返回 segmentId 给前端。
#[tauri::command]
fn begin_segment(
    app: tauri::AppHandle,
    state: tauri::State<Mutex<Session>>,
    window: Window,
) -> Result<String, String> {
    let label = window.label().to_string();
    let (seg, dir) = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        if !s.active {
            return Err("session not active".into());
        }
        let n = s.segment_seq.entry(label.clone()).or_insert(0);
        let seg = format!("{}#{}", label, n);
        *n += 1;
        s.current.insert(label.clone(), seg.clone());
        (seg, s.dir.clone())
    };
    if let Some(dir) = dir {
        append_lifecycle(
            &dir,
            json!({ "type": "shown", "label": label, "segmentId": seg, "t": now_ms() }),
        )?;
    }
    let _ = app; // 保留以备扩展
    Ok(seg)
}

#[tauri::command]
fn append_events(
    state: tauri::State<Mutex<Session>>,
    segment_id: String,
    events: Vec<Value>,
) -> Result<(), String> {
    let dir = state.lock().map_err(|e| e.to_string())?.dir.clone();
    if let Some(dir) = dir {
        append_events_file(&dir, &segment_id, &events)?;
    }
    Ok(())
}

#[tauri::command]
fn list_sessions(app: tauri::AppHandle) -> Result<Vec<Value>, String> {
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
fn read_session(app: tauri::AppHandle, id: String) -> Result<Value, String> {
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
fn delete_session(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let dir = recordings_root(&app).join(&id);
    fs::remove_dir_all(&dir).map_err(|e| e.to_string())
}

/// 窗口生命周期：子窗口关闭=隐藏（停段），主窗口关闭=退出。
fn on_window_event(window: &Window, event: &WindowEvent) {
    let app = window.app_handle();
    let label = window.label().to_string();

    match event {
        WindowEvent::CloseRequested { api, .. } => {
            if label == "main" {
                return; // 主窗口：默认关闭 -> 退出进程
            }
            let Some(state) = app.try_state::<Mutex<Session>>() else {
                return;
            };
            // 仅对正在录制的子窗口（有活跃段）拦截关闭为隐藏；
            // 未录制（如 player 窗口）或非录制期 -> 默认关闭销毁
            let (seg, dir, t) = {
                let Ok(mut s) = state.lock() else {
                    return;
                };
                if !s.active {
                    return;
                }
                (s.current.remove(&label), s.dir.clone(), now_ms())
            };
            let Some(seg) = seg else {
                return; // 该窗口无活跃段，默认关闭
            };
            api.prevent_close();
            let _ = window.hide();
            if let Some(dir) = dir {
                let _ = append_lifecycle(
                    &dir,
                    json!({ "type": "hidden", "label": label, "segmentId": seg, "t": t }),
                );
            }
            let _ = app.emit_to(
                EventTarget::labeled(&label),
                "segment",
                json!({ "action": "stop" }),
            );
        }
        WindowEvent::Focused(focused) => {
            // 回放窗不参与录制，跳过其 focus 事件，避免 windows.jsonl 出现无 segment 的孤儿记录
            if *focused && !label.starts_with("player-") {
                let Some(state) = app.try_state::<Mutex<Session>>() else {
                    return;
                };
                let (active, dir) = {
                    let Ok(s) = state.lock() else {
                        return;
                    };
                    (s.active, s.dir.clone())
                };
                if active {
                    if let Some(dir) = dir {
                        let _ = append_lifecycle(
                            &dir,
                            json!({ "type": "focus", "label": label, "t": now_ms() }),
                        );
                    }
                }
            }
        }
        _ => {}
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(Session::default()))
        .manage(Mutex::new(ingest::IngestState::default()))
        .on_window_event(on_window_event)
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
            start_session,
            stop_session,
            is_recording_active,
            begin_segment,
            append_events,
            list_sessions,
            read_session,
            delete_session,
            ingest::get_ingest_config,
            ingest::set_ingest_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
