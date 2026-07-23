//! 本地优先前端观测平台 · Tauri 多窗口录制协调插件。
//!
//! 把多窗口对齐录制逻辑（Session 状态、segment 分配、窗口 show/hide/focus 生命周期拦截）
//! 抽成可复用插件，别的 Tauri 2 应用安装即得。两种部署模式：
//!
//! - [`Mode::Local`]：Rust 侧直接落盘到 `appDataDir/recordings/`（console 自录 self-obs）。
//! - [`Mode::Remote`]：Rust 侧只管窗口协调 + 状态 + 事件驱动，不落盘；
//!   前端用 `HttpSink` 上报到 console 本地 server（外部 Tauri 应用）。
//!
//! 协议见 docs/架构/被观测侧（采集）.md；阶段定位见 docs/阶段路径/P5-Tauri-Plugin.md。

pub mod commands;
pub mod config;
pub mod lifecycle;
pub mod session;
pub mod storage;

pub use config::{Mode, ObserverConfig};

use serde_json::json;
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{AppHandle, Emitter, EventTarget, Manager, Runtime};

type SessionState = std::sync::Mutex<session::Session>;

/// 若会话活跃，定向 emit `segment{start}` 驱动目标窗口开新段。
/// 供宿主 `open_window` 复用已隐藏窗口时调用，避免重复判断 active + emit_to。
pub fn emit_segment_start_if_active<R: Runtime>(app: &AppHandle<R>, label: &str) -> bool {
    let Some(state) = app.try_state::<SessionState>() else {
        return false;
    };
    let active = state.lock().map(|s| s.active).unwrap_or(false);
    if active {
        let _ = app.emit_to(
            EventTarget::labeled(label),
            "segment",
            json!({ "action": "start" }),
        );
    }
    active
}

/// 默认 Local 模式初始化（console 自录用）。
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    init_with(ObserverConfig::default())
}

/// 带配置初始化：外部 Tauri 应用用 `.mode(Mode::Remote)` 装插件。
///
/// ```ignore
/// tauri::Builder::default()
///     .plugin(tauri_plugin_observer::init_with(
///         tauri_plugin_observer::ObserverConfig {
///             mode: tauri_plugin_observer::Mode::Remote,
///             ..Default::default()
///         },
///     ))
///     .run(tauri::generate_context!())
///     .expect("error while running tauri application");
/// ```
pub fn init_with<R: Runtime>(config: ObserverConfig) -> TauriPlugin<R> {
    Builder::<R>::new("observer")
        .invoke_handler(tauri::generate_handler![
            commands::start_session,
            commands::stop_session,
            commands::is_recording_active,
            commands::begin_segment,
            commands::append_events,
            // Remote 模式专用：前端绑定 server 分配的 sessionId 并广播给各窗口
            commands::bind_session,
            commands::session_id,
            // 窗口复用时由宿主 open_window 调用，emit segment:start 驱动前端开新段
            commands::notify_segment_start,
        ])
        .setup(move |app, _api| {
            app.manage(std::sync::Mutex::new(session::Session::new(config.clone())));
            Ok(())
        })
        // Tauri 2 plugin Builder 无 on_window_event；在窗口就绪时给每个窗口挂
        // Window::on_window_event，拦截 CloseRequested/Focused。
        .on_window_ready(|window| {
            let app = window.app_handle().clone();
            let label = window.label().to_string();
            window.on_window_event(move |event| {
                lifecycle::handle_window_event(&app, &label, event);
            });
        })
        .build()
}
