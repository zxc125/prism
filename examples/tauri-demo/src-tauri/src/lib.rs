//! P5/P16 验证样例：一个独立 Tauri 2 应用安装 tauri-plugin-observer，开多窗口录制。
//!
//! 双模式开关（D10）：环境变量 `VITE_OBSERVER_MODE=local` 时插件用 Local 模式——Rust
//! 直接落盘到本应用 `appDataDir/recordings/`（source=tauri + appId 随 ObserverConfig
//! 写入 session.json），前端 `initTauri({ mode: "local" })`，可 `exportSession` 导出
//! bundle 回 console 导入；缺省 = Remote 模式——经 HttpSink 上报 console 本地 server，
//! sessionId 由前端取得后经插件 bind_session 广播共享。
//! Rust 与前端共用同一环境变量（`pnpm tauri dev` 的 beforeDevCommand 继承 shell env，
//! Vite 只把 `VITE_` 前缀变量暴露给客户端，Rust 读进程 env），单一来源防错配。

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_observer::{Mode, ObserverConfig};

/// 按环境变量解析插件配置：`VITE_OBSERVER_MODE=local` -> Local（本地落盘），否则 Remote。
fn observer_config() -> ObserverConfig {
    if std::env::var("VITE_OBSERVER_MODE").as_deref() == Ok("local") {
        ObserverConfig {
            mode: Mode::Local,
            source: "tauri".into(),
            app_id: Some("tauri-demo".into()),
            ..Default::default()
        }
    } else {
        ObserverConfig {
            mode: Mode::Remote,
            ..Default::default()
        }
    }
}

/// 由路由推导窗口 label：/ -> main，/child/123 -> child-123。
fn window_label(route: &str) -> String {
    let label = route.trim_start_matches('/').replace('/', "-");
    if label.is_empty() {
        "main".to_string()
    } else {
        label
    }
}

#[tauri::command]
fn open_window(app: AppHandle, route: String) -> Result<String, String> {
    let label = window_label(&route);
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.show();
        let _ = window.set_focus();
        // 复用已隐藏窗口：若录制中，插件 emit segment:start 开新段
        tauri_plugin_observer::emit_segment_start_if_active(&app, &label);
        return Ok(label);
    }
    let init_script = format!(
        "if (!window.location.hash) window.location.replace('#{route}');"
    );
    WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("index.html".into()))
        .title(format!("Demo · {}", label))
        .inner_size(640.0, 480.0)
        .initialization_script(&init_script)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(label)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_observer::init_with(observer_config()))
        .invoke_handler(tauri::generate_handler![open_window])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
