//! 录制协调命令。Local 模式落盘（console self-obs / 外部应用 opt-in 本地落盘，P16），
//! Remote 模式仅管状态 + 事件驱动；`list_sessions`/`export_session` 只读命令 Local 限定。
//!
//! 命令泛型 `R: Runtime` 以适配任意宿主 runtime（与 tauri-plugin-opener 一致）。

use std::sync::Mutex;

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, Runtime, State, Window};

use crate::config::{Mode, ObserverConfig};
use crate::session::Session;
use crate::storage::{
    append_events_file, append_lifecycle, build_export_bundle, finalize_session, now_ms,
    recordings_root, validate_session_id,
};
use crate::storage::list_sessions as storage_list_sessions;

type SessionState = Mutex<Session>;

/// session.json 内容组装（start/stop 共用）。`app_id` 为 None 时省键（P16 D9，与
/// console 现网 session.json 形态一致）；`ended_at` 供 stop 覆写回填。
fn session_meta_payload(
    source: &str,
    app_id: Option<&str>,
    id: &str,
    started_at: i64,
    ended_at: Option<i64>,
) -> Value {
    let mut m = serde_json::Map::new();
    m.insert("id".into(), json!(id));
    m.insert("source".into(), json!(source));
    if let Some(app) = app_id {
        m.insert("appId".into(), json!(app));
    }
    m.insert("startedAt".into(), json!(started_at));
    if let Some(ended) = ended_at {
        m.insert("endedAt".into(), json!(ended));
    }
    Value::Object(m)
}

/// Local：建目录、置 active、补记初始 focus、广播 `recording-session{active:true,id}`。
/// Remote：仅置 active（会话目录由前端 HttpSink 在 console server 侧建），等 [`bind_session`] 绑定 sessionId 后广播。
#[tauri::command]
pub fn start_session<R: Runtime>(app: AppHandle<R>, state: State<'_, SessionState>) -> Result<String, String> {
    let id = format!("{}", now_ms());
    let started_at = now_ms();
    let (mode, source, app_id) = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.id = Some(id.clone());
        s.started_at = started_at;
        s.active = true;
        s.segment_seq.clear();
        s.current.clear();
        s.remote_session_id = None;
        s.dir = None;
        (
            s.config.mode,
            s.config.source.clone(),
            s.config.app_id.clone(),
        )
    };

    let dir = if mode == Mode::Local {
        let dir = recordings_root(&app).join(&id);
        std::fs::create_dir_all(dir.join("segments")).map_err(|e| e.to_string())?;
        std::fs::write(
            dir.join("session.json"),
            session_meta_payload(&source, app_id.as_deref(), &id, started_at, None).to_string(),
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
    let (dir, id, started_at, mode, source, app_id) = {
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
        (
            s.dir.clone(),
            s.id.clone(),
            s.started_at,
            s.config.mode,
            s.config.source.clone(),
            s.config.app_id.clone(),
        )
    };
    if mode == Mode::Local {
        if let Some(dir) = dir {
            let ended_at = now_ms();
            let _ = finalize_session(&dir, ended_at);
            // 与原 console 行为一致：写回含 endedAt 的 session.json（失败不致命）
            let meta = session_meta_payload(
                &source,
                app_id.as_deref(),
                id.as_deref().unwrap_or(""),
                started_at,
                Some(ended_at),
            );
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
///
/// `(async)`（P17 D3）：非 async 命令默认执行在主线程（macOS = NSApplication 事件
/// 循环线程），加 `(async)` 后 serde 反序列化 + 写盘脱离主线程，不再阻塞全进程窗口
/// 事件循环。保序论证：① 同窗口——SDK flush 串行化（observer-sdk SegmentRecorder）
/// 保证同一 recorder 至多一个在途 invoke，下一批发送在前一批响应之后，到达序 = 发送序；
/// ② 跨窗口——segmentId 不同即文件不同，`append(true)` 只保单文件完整性且互不交叉；
/// ③ begin/append 无竞态——`rec.start()` await `begin_segment` 之后才启动 rrweb 与
/// flush 定时器。命令名不变，`observer:default` 权限与 `generate_handler![]` 注册零改动。
#[tauri::command(async)]
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

/// 只读命令的模式闸门：list/export 仅 Local 模式可用（Remote 数据不在本机，P16 D6）。
fn ensure_local(config: &ObserverConfig) -> Result<(), String> {
    if config.mode == Mode::Local {
        Ok(())
    } else {
        Err("observer: list/export commands are Local-mode only".into())
    }
}

/// Local 模式：列出本应用 `appDataDir/recordings/` 下的会话元信息（不含事件流）。
/// 与 console 自有 `list_sessions` 同源（observer-storage），供外部应用本地落盘后
/// 自行选择会话导出（P16）。
#[tauri::command]
pub fn list_sessions<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, SessionState>,
) -> Result<Vec<Value>, String> {
    ensure_local(&state.lock().map_err(|e| e.to_string())?.config)?;
    Ok(storage_list_sessions(&recordings_root(&app)))
}

/// Local 模式：导出会话为 `prism-session` bundle（JSON，契约 version 1 不变，P16 D5）。
/// 前端拿到后自行落盘/传输；session_id 纯数字校验防路径穿越（复用 P7 防护）。
///
/// `(async)`（2026-09-10，bond 0910 提案改动 A）：与 [`append_events`]（P17 D3）同类——
/// 读全部段文件 + 组装 bundle 是秒级操作，非 async 命令默认跑主线程（macOS =
/// NSApplication 事件循环）会冻结全进程窗口。只读命令无保序约束，加 `(async)` 脱离
/// 主线程即可；命令名/返回类型不变，权限与注册零改动。
#[tauri::command(async)]
pub fn export_session<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, SessionState>,
    session_id: String,
) -> Result<Value, String> {
    ensure_local(&state.lock().map_err(|e| e.to_string())?.config)?;
    if !validate_session_id(&session_id) {
        return Err(format!("invalid session id: {session_id}"));
    }
    build_export_bundle(&recordings_root(&app).join(&session_id))
}

/// 窗口复用时由宿主 `open_window` 调用：若录制中，定向 emit `segment{start}` 驱动
/// 该窗口开新段。委托 [`crate::emit_segment_start_if_active`]，命令形式供 JS 调用。
#[tauri::command]
pub fn notify_segment_start<R: Runtime>(app: AppHandle<R>, label: String) -> Result<bool, String> {
    Ok(crate::emit_segment_start_if_active(&app, &label))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_meta_omits_app_id_and_ended_when_none() {
        let v = session_meta_payload("tauri", None, "123", 1000, None);
        assert_eq!(v["id"], "123");
        assert_eq!(v["source"], "tauri");
        assert!(v.get("appId").is_none(), "appId None 时省键（D9）");
        assert!(v.get("endedAt").is_none());
    }

    #[test]
    fn session_meta_includes_app_id_and_ended() {
        let v = session_meta_payload("self", Some("demo"), "123", 1000, Some(2000));
        assert_eq!(v["appId"], "demo");
        assert_eq!(v["endedAt"], 2000);
    }

    #[test]
    fn stop_overwrite_keeps_app_id() {
        // 关键陷阱：stop_session 整文件覆写不得冲掉 appId/source
        let start = session_meta_payload("tauri", Some("demo"), "123", 1000, None);
        let end = session_meta_payload(
            "tauri",
            Some("demo"),
            "123",
            start["startedAt"].as_i64().unwrap(),
            Some(2000),
        );
        assert_eq!(end["appId"], start["appId"]);
        assert_eq!(end["source"], start["source"]);
    }

    #[test]
    fn ensure_local_gates_by_mode() {
        assert!(ensure_local(&ObserverConfig::default()).is_ok()); // 默认 Local
        let remote = ObserverConfig {
            mode: Mode::Remote,
            ..Default::default()
        };
        assert!(ensure_local(&remote).is_err());
    }

    #[test]
    fn export_session_id_guard_rejects_traversal() {
        // export_session 命令以此闸门防路径穿越（P7 防护复用）
        assert!(!validate_session_id("../etc"));
        assert!(!validate_session_id("1/../../x"));
        assert!(!validate_session_id(""));
        assert!(validate_session_id("1730000000000"));
    }
}
