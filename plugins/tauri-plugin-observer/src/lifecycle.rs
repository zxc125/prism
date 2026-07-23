//! 窗口生命周期拦截：子窗口关闭 = 隐藏（停段），主窗口关闭 = 退出。
//!
//! Local 模式直接落 `windows.jsonl`；Remote 模式 emit 事件交前端经 `HttpSink` 上报。
//!
//! Tauri 2 plugin Builder 无 `on_window_event` 钩子，故在 `on_window_ready` 时给每个
//! 窗口挂 `Window::on_window_event`，回调里只持 `AppHandle` + `label`（事件闭包不传 window），
//! 需要 hide 时从 app 重新取窗口。
//!
//! 关闭拦截判断：
//! - Local：仅拦截有活跃段（`current`）的子窗口（精确，与 console 原行为一致）。
//! - Remote：会话活跃时的所有非主、非 skip 子窗口（观测插件语义：所有窗口皆被观测）。

use std::sync::Mutex;

use serde_json::json;
use tauri::{AppHandle, Emitter, EventTarget, Manager, Runtime, WindowEvent};

use crate::config::Mode;
use crate::session::Session;
use crate::storage::{append_lifecycle, now_ms};

type SessionState = Mutex<Session>;

pub fn handle_window_event<R: Runtime>(
    app: &AppHandle<R>,
    label: &str,
    event: &WindowEvent,
) {
    match event {
        WindowEvent::CloseRequested { api, .. } => on_close_requested(app, label, api),
        WindowEvent::Focused(focused) => {
            if *focused {
                on_focused(app, label);
            }
        }
        _ => {}
    }
}

fn on_close_requested<R: Runtime>(app: &AppHandle<R>, label: &str, api: &tauri::CloseRequestApi) {
    let Some(state) = app.try_state::<SessionState>() else {
        return;
    };
    let (mode, seg, dir, should_intercept) = {
        let Ok(mut s) = state.lock() else {
            return;
        };
        if !s.active {
            return;
        }
        let is_main = label == s.config.main_label;
        let skip = !s.config.skip_focus_prefix.is_empty()
            && label.starts_with(&s.config.skip_focus_prefix);
        if is_main || skip {
            return; // 主窗口关闭 = 退出；skip 窗口 = 默认关闭销毁
        }
        match s.config.mode {
            Mode::Local => {
                // 仅拦截有活跃段的窗口
                let seg = s.current.remove(label);
                let should = seg.is_some();
                (Mode::Local, seg, s.dir.clone(), should)
            }
            Mode::Remote => {
                // 观测插件：会话活跃时所有子窗口关闭 = 隐藏
                (Mode::Remote, None, None, true)
            }
        }
    };
    if !should_intercept {
        return;
    }
    api.prevent_close();
    // 事件闭包不持 window 引用，从 app 重新取窗口 hide
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.hide();
    }
    let t = now_ms();
    if mode == Mode::Local {
        if let (Some(seg), Some(dir)) = (seg, dir) {
            let _ = append_lifecycle(
                &dir,
                json!({ "type": "hidden", "label": label, "segmentId": seg, "t": t }),
            );
        }
    }
    // Remote：hidden 由前端收到 segment:stop 后自带 segmentId 上报
    let _ = app.emit_to(
        EventTarget::labeled(label),
        "segment",
        json!({ "action": "stop" }),
    );
}

fn on_focused<R: Runtime>(app: &AppHandle<R>, label: &str) {
    let Some(state) = app.try_state::<SessionState>() else {
        return;
    };
    let (active, mode, skip_prefix, dir) = {
        let Ok(s) = state.lock() else {
            return;
        };
        (
            s.active,
            s.config.mode,
            s.config.skip_focus_prefix.clone(),
            s.dir.clone(),
        )
    };
    if !active {
        return;
    }
    if !skip_prefix.is_empty() && label.starts_with(&skip_prefix) {
        return; // 回放窗等不参与录制，跳过其 focus
    }
    let t = now_ms();
    match mode {
        Mode::Local => {
            if let Some(dir) = dir {
                let _ = append_lifecycle(
                    &dir,
                    json!({ "type": "focus", "label": label, "t": t }),
                );
            }
        }
        Mode::Remote => {
            // 通知前端报 focus lifecycle（前端经 HttpSink 上报）
            let _ = app.emit_to(
                EventTarget::labeled(label),
                "observer-lifecycle",
                json!({ "type": "focus", "label": label, "t": t }),
            );
        }
    }
}
