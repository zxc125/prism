// Learn more about Tauri commands at https://tauri.app/develop/calling-rust-from-js/
use std::fs;

use serde_json::Value;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

mod ingest;

use observer_storage::{
    build_export_bundle, import_bundle_content, merge_session_meta, read_annotations,
    write_annotations,
};
use tauri_plugin_observer::storage::recordings_root;

/// 由路由推导窗口 label：/settings -> settings，/s/abc -> s-abc，/ -> main。
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
    // P10：player 走 /s/:id（in-app 路由为主，open_window 仅用于「新窗口打开」）
    let segment = route.split('/').nth(1).unwrap_or("");
    let (title, width, height) = match segment {
        "settings" => ("设置", 480.0, 640.0),
        "s" => ("播放器", 960.0, 600.0),
        "player" => ("播放器", 960.0, 600.0),
        "live" => ("实时观测", 720.0, 640.0),
        "tenants" => ("租户", 640.0, 600.0),
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
    Ok(observer_storage::list_sessions(&recordings_root(&app)))
}

/// 回放用：一次性返回会话元信息、窗口生命周期、各 segment 事件。
#[tauri::command]
fn read_session(app: AppHandle, id: String) -> Result<Value, String> {
    observer_storage::read_session(&recordings_root(&app).join(&id))
}

#[tauri::command]
fn list_annotations(app: AppHandle, id: String) -> Result<Vec<Value>, String> {
    Ok(read_annotations(&recordings_root(&app).join(&id)))
}

/// 整体覆写标注文件（前端持有完整列表，增删改后整体保存）。
#[tauri::command]
fn save_annotations(app: AppHandle, id: String, annotations: Vec<Value>) -> Result<(), String> {
    write_annotations(&recordings_root(&app).join(&id), &annotations)
}

/// 合并更新 session.json 的元信息字段（name/note/tags 等），返回写回后的完整 session。
#[tauri::command]
fn update_session_meta(app: AppHandle, id: String, meta: Value) -> Result<Value, String> {
    let path = recordings_root(&app).join(&id).join("session.json");
    merge_session_meta(&path, &meta)
}

/// 导出会话为单文件 JSON bundle。前端拿到后用 Blob 下载，零云依赖。
#[tauri::command]
fn export_session(app: AppHandle, id: String) -> Result<Value, String> {
    build_export_bundle(&recordings_root(&app).join(&id))
}

/// 导入会话 JSON bundle（内容直传；小文件 / 云端 HttpBackend 上传路径）。
#[tauri::command]
fn import_session(app: AppHandle, content: String) -> Result<String, String> {
    import_bundle_content(&recordings_root(&app), &content)
}

/// 从文件路径导入会话 bundle（Rust 侧读文件，避免大 JSON 过 IPC）。
#[tauri::command]
fn import_session_path(app: AppHandle, path: String) -> Result<String, String> {
    let content = fs::read_to_string(&path).map_err(|e| format!("读取文件失败：{e}"))?;
    import_bundle_content(&recordings_root(&app), &content)
}

/// 读取文本文件内容（供 HttpBackend 模式：文件选择器拿 path 后读内容上传云端）。
/// 仅读取用户经 dialog 主动选择的文件。
#[tauri::command]
fn read_text_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("读取文件失败：{e}"))
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
        .plugin(tauri_plugin_dialog::init())
        // 录制协调逻辑已抽成 tauri-plugin-observer（Local 模式 = self-obs 落盘）。
        // skip_focus_prefix "player-" 与 console 原行为一致：回放窗不参与录制。
        .plugin(tauri_plugin_observer::init_with(
            tauri_plugin_observer::ObserverConfig {
                mode: tauri_plugin_observer::Mode::Local,
                skip_focus_prefix: "player-".into(),
                ..Default::default()
            },
        ))
        .setup(|app| {
            // 载入持久化的接收配置，建 IngestState（含 observer-server 句柄）再启动
            let cfg = ingest::load_config(app.handle());
            let data_dir = recordings_root(app.handle());
            std::fs::create_dir_all(&data_dir).ok();
            app.manage(std::sync::Mutex::new(ingest::IngestState::new(cfg, data_dir)));
            ingest::start_server(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            open_window,
            list_sessions,
            read_session,
            delete_session,
            list_annotations,
            save_annotations,
            update_session_meta,
            export_session,
            import_session,
            import_session_path,
            read_text_file,
            ingest::get_ingest_config,
            ingest::set_ingest_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
