// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
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
fn open_window(app: tauri::AppHandle, route: String) -> Result<String, String> {
    let label = window_label(&route);

    // 已存在则聚焦，避免重复创建
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(label);
    }

    // 按路由首段决定标题与尺寸
    let segment = route.split('/').nth(1).unwrap_or("");
    let (title, width, height) = match segment {
        "settings" => ("设置", 480.0, 640.0),
        "player" => ("播放器", 960.0, 600.0),
        _ => ("RRWeb Demo", 800.0, 600.0),
    };
    let url = format!("index.html#{route}");

    WebviewWindowBuilder::new(&app, &label, WebviewUrl::App(url.into()))
        .title(title)
        .inner_size(width, height)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(label)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, open_window])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
