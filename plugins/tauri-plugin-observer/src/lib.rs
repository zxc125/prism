//! 本地优先前端观测平台 · Tauri 多窗口录制协调插件。
//!
//! 把多窗口对齐录制逻辑（Session 状态、segment 分配、窗口 show/hide/focus 生命周期拦截）
//! 抽成可复用插件，别的 Tauri 2 应用安装即得。两种部署模式：
//!
//! - [`Mode::Local`]：Rust 侧直接落盘到 `<dir_base>/<dir_name>`（默认
//!   `appDataDir/recordings/`，P20 可配）。console self-obs 与外部应用 opt-in 本地落盘
//!   （P16）均用此模式；`list_sessions`/`export_session`/
//!   `export_session_to_file`（P18）只读命令 Local 限定，导出 `prism-session` bundle
//!   闭环回 console 导入回放。支持 `retention` 保留策略自动清理（P23）：声明
//!   [`ObserverConfig::retention`] 即启用后台清扫（首跑延迟 30s、每 1h 一轮），
//!   活跃会话豁免；缺省空策略零行为变化。
//! - [`Mode::Remote`]：Rust 侧只管窗口协调 + 状态 + 事件驱动，不落盘；
//!   前端用 `HttpSink` 上报到 console 本地 server（外部 Tauri 应用）。
//!
//! 协议见 docs/架构/被观测侧（采集）.md；阶段定位见 docs/阶段路径/P5-Tauri-Plugin.md。

pub mod commands;
pub mod config;
pub mod lifecycle;
pub mod session;
pub mod storage;

pub use config::{DirBase, Mode, ObserverConfig};
/// P23：保留策略类型 re-export，宿主声明 `retention` 无需直接依赖 observer-storage。
pub use observer_storage::RetentionPolicy;

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

/// 保留清扫首跑延迟：避让启动 IO 高峰。
const RETENTION_FIRST_RUN_DELAY_SECS: u64 = 30;
/// 保留清扫周期：本地数据无 10min 级时效敏感，24h 又会漏长挂机（P23 D4）。
const RETENTION_SWEEP_INTERVAL_SECS: u64 = 3600;

/// P23：Local + 非空策略时起后台清扫线程。循环体 catch_unwind——panic 记日志后
/// continue，防「数据无界增长」静默回归（现成 panic 源：`recordings_root_with`
/// 的目录解析 expect）。Remote（无落盘）/ 空策略（缺省宿主零行为变化）不启动。
fn spawn_retention_sweeper<R: Runtime>(app: AppHandle<R>, config: ObserverConfig) {
    if config.mode != Mode::Local || config.retention.is_empty() {
        return;
    }
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(RETENTION_FIRST_RUN_DELAY_SECS));
        loop {
            let swept =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sweep_once(&app, &config)));
            if let Err(panic) = swept {
                let msg = panic
                    .downcast_ref::<&str>()
                    .map_or(String::new(), |s| (*s).to_string());
                eprintln!("[observer] retention 清扫线程 panic，下轮重试: {msg}");
            }
            std::thread::sleep(std::time::Duration::from_secs(RETENTION_SWEEP_INTERVAL_SECS));
        }
    });
}

/// 单轮清扫。锁纪律：`SessionState` 锁只包 id 提取、绝不跨 enforce 调用（全 crate
/// 对毒化锁按静默 no-op 处理，持锁跨 IO/panic 会让录制整体哑火）；protect 必须
/// 看 `active`——`stop_session` 不清 `s.id`，不看 active 会无限期豁免已收口会话
/// （P22 D5 长活会话合法性由 protect 保证）。
fn sweep_once<R: Runtime>(app: &AppHandle<R>, config: &ObserverConfig) {
    let protect: Vec<String> = app
        .try_state::<SessionState>()
        .and_then(|state| {
            // 锁守卫不跨闭包边界；active 判定 + id 提取都在锁内完成
            let s = state.lock().ok()?;
            if s.active { s.id.clone() } else { None }
        })
        .into_iter()
        .collect();
    let protect_refs: Vec<&str> = protect.iter().map(String::as_str).collect();
    let root = storage::recordings_root_with(app, config);
    // 被删 id 观测：清扫前快照、清扫后差集（1h 级扫描，开销可忽略）。
    let before: Vec<String> = storage::list_sessions(&root)
        .iter()
        .filter_map(|s| s["id"].as_str().map(str::to_string))
        .collect();
    match observer_storage::enforce_retention_protect(&root, &config.retention, &protect_refs) {
        Ok(0) => {}
        Ok(n) => {
            let after: std::collections::HashSet<String> = storage::list_sessions(&root)
                .iter()
                .filter_map(|s| s["id"].as_str().map(str::to_string))
                .collect();
            let deleted: Vec<&str> = before
                .iter()
                .map(String::as_str)
                .filter(|id| !after.contains(*id))
                .collect();
            eprintln!("[observer] retention 清理 {n} 个会话: {deleted:?}");
        }
        Err(e) => eprintln!("[observer] retention 失败: {e}"),
    }
}

/// 带配置初始化：外部 Tauri 应用用 `.mode(Mode::Remote)`（HTTP 上报）或
/// `.mode(Mode::Local)`（本地落盘，P16）装插件。
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
            // P16：只读命令，Local 模式限定（外部应用本地落盘 -> 导出 bundle 闭环）
            commands::list_sessions,
            commands::export_session,
            // P18：to-file 变体（宿主传 path，插件代执行序列化+落盘；Local 限定）
            commands::export_session_to_file,
            // Remote 模式专用：前端绑定 server 分配的 sessionId 并广播给各窗口
            commands::bind_session,
            commands::session_id,
            // 窗口复用时由宿主 open_window 调用，emit segment:start 驱动前端开新段
            commands::notify_segment_start,
        ])
        .setup(move |app, _api| {
            app.manage(std::sync::Mutex::new(session::Session::new(config.clone())));
            // P23：保留策略清扫线程（Local + 非空策略才启动，缺省零线程开销）
            spawn_retention_sweeper(app.clone(), config);
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
