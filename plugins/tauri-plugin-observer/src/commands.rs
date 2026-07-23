//! 录制协调命令。Local 模式落盘（self-obs），Remote 模式仅管状态 + 事件驱动。
//!
//! 命令泛型 `R: Runtime` 以适配任意宿主 runtime（与 tauri-plugin-opener 一致）。

use std::sync::Mutex;

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, Runtime, State, Window};

use crate::config::Mode;
use crate::session::Session;
use crate::storage::{append_events_file, append_lifecycle, finalize_session, now_ms, recordings_root};

type SessionState = Mutex<Session>;

/// Local：建目录、置 active、补记初始 focus、广播 `recording-session{active:true,id}`。
/// Remote：仅置 active（会话目录由前端 HttpSink 在 console server 侧建），等 [`bind_session`] 绑定 sessionId 后广播。
#[tauri::command]
pub fn start_session<R: Runtime>(app: AppHandle<R>, state: State<'_, SessionState>) -> Result<String, String> {
    let id = format!("{}", now_ms());
    let started_at = now_ms();
    let mode = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.id = Some(id.clone());
        s.started_at = started_at;
        s.active = true;
        s.segment_seq.clear();
        s.current.clear();
        s.remote_session_id = None;
        s.dir = None;
        s.config.mode
    };

    let dir = if mode == Mode::Local {
        let dir = recordings_root(&app).join(&id);
        std::fs::create_dir_all(dir.join("segments")).map_err(|e| e.to_string())?;
        std::fs::write(
            dir.join("session.json"),
            json!({ "id": id, "source": "self", "startedAt": started_at }).to_string(),
        )
        .map_err(|e| e.to_string())?;
        state.lock().map_err(|e| e.to_string())?.dir = Some(dir.clone());
        Some(dir)
    } else {
        None
    };

    // 补记初始 focus：会话开始时当前聚焦窗口，避免 focus 时间线在 t=0 为空
    if let Some(dir) = dir {
        let skip_prefix = state
            .lock()
            .map_err(|e| e.to_string())?
            .config
            .skip_focus_prefix
            .clone();
        for (label, w) in app.webview_windows() {
            if !skip_prefix.is_empty() && label.starts_with(&skip_prefix) {
                continue;
            }
            if w.is_focused().unwrap_or(false) {
                let _ = append_lifecycle(
                    &dir,
                    json!({ "type": "focus", "label": label, "t": now_ms() }),
                );
                break;
            }
        }
    }

    app.emit("recording-session", json!({ "active": true, "id": id }))
        .map_err(|e| e.to_string())?;
    Ok(id)
}

/// Remote 模式专用：前端从 console `/ingest/session` 拿到 sessionId 后绑定，
/// 广播 `recording-session{active:true,sessionId}` 让各窗口共享同一 sessionId。
#[tauri::command]
pub fn bind_session<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, SessionState>,
    session_id: String,
) -> Result<(), String> {
    {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.active = true;
        s.remote_session_id = Some(session_id.clone());
    }
    app.emit(
        "recording-session",
        json!({ "active": true, "sessionId": session_id }),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Remote 模式：子窗口取已绑定的 sessionId（主窗口尚未绑定时返回 null）。
#[tauri::command]
pub fn session_id(state: State<'_, SessionState>) -> Result<Option<String>, String> {
    Ok(state
        .lock()
        .map_err(|e| e.to_string())?
        .remote_session_id
        .clone())
}

/// 停止会话。Local：关闭活跃段记 hidden、写 endedAt。Remote：清 sessionId。
/// 两者都广播 `recording-session{active:false}`。
#[tauri::command]
pub fn stop_session<R: Runtime>(app: AppHandle<R>, state: State<'_, SessionState>) -> Result<(), String> {
    let (dir, id, started_at, mode) = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.active = false;
        s.remote_session_id = None;
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
        (s.dir.clone(), s.id.clone(), s.started_at, s.config.mode)
    };
    if mode == Mode::Local {
        if let Some(dir) = dir {
            let ended_at = now_ms();
            let _ = finalize_session(&dir, ended_at);
            // 与原 console 行为一致：写回含 endedAt 的 session.json（失败不致命）
            let meta = json!({ "id": id, "source": "self", "startedAt": started_at, "endedAt": ended_at });
            let _ = std::fs::write(dir.join("session.json"), meta.to_string());
        }
    }
    app.emit("recording-session", json!({ "active": false }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn is_recording_active(state: State<'_, SessionState>) -> bool {
    state.lock().map(|s| s.active).unwrap_or(false)
}

/// 由窗口挂载时调用（首次创建）或收到 segment:start 后调用（复用显示）。
/// Local：分配 segmentId `<label>#<n>`、记 shown、返回给前端。
/// Remote：no-op（segmentId 由前端 HttpSink 自分配并上报），仅保留命令以兼容统一 Sink 接口。
#[tauri::command]
pub fn begin_segment<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, SessionState>,
    window: Window<R>,
) -> Result<String, String> {
    let label = window.label().to_string();
    let (seg, dir, mode) = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        if !s.active {
            return Err("session not active".into());
        }
        if s.config.mode == Mode::Local {
            let n = s.segment_seq.entry(label.clone()).or_insert(0);
            let seg = format!("{}#{}", label, n);
            *n += 1;
            s.current.insert(label.clone(), seg.clone());
            (seg, s.dir.clone(), Mode::Local)
        } else {
            (String::new(), None, Mode::Remote)
        }
    };
    if mode == Mode::Local {
        if let Some(dir) = dir {
            append_lifecycle(
                &dir,
                json!({ "type": "shown", "label": label, "segmentId": seg, "t": now_ms() }),
            )?;
        }
    }
    let _ = app; // 保留以备扩展
    Ok(seg)
}

/// Local 模式：追加事件到 segment 文件。Remote 模式 no-op（前端走 HttpSink）。
#[tauri::command]
pub fn append_events(
    state: State<'_, SessionState>,
    segment_id: String,
    events: Vec<Value>,
) -> Result<(), String> {
    let (dir, mode) = {
        let s = state.lock().map_err(|e| e.to_string())?;
        (s.dir.clone(), s.config.mode)
    };
    if mode == Mode::Local {
        if let Some(dir) = dir {
            append_events_file(&dir, &segment_id, &events)?;
        }
    }
    Ok(())
}

/// 窗口复用时由宿主 `open_window` 调用：若录制中，定向 emit `segment{start}` 驱动
/// 该窗口开新段。委托 [`crate::emit_segment_start_if_active`]，命令形式供 JS 调用。
#[tauri::command]
pub fn notify_segment_start<R: Runtime>(app: AppHandle<R>, label: String) -> Result<bool, String> {
    Ok(crate::emit_segment_start_if_active(&app, &label))
}
