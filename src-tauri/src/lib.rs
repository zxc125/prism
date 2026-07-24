// Learn more about Tauri commands at https://tauri.app/develop/calling-rust-from-js/
use std::fs;
use std::path::Path;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

mod annotations;
mod ingest;

use annotations::{read_annotations, write_annotations};
use tauri_plugin_observer::storage::{now_ms, recordings_root};

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
    let annotations = read_annotations(&dir);
    Ok(json!({ "session": session, "windows": windows, "segments": segments, "annotations": annotations }))
}

#[tauri::command]
fn list_annotations(app: AppHandle, id: String) -> Result<Vec<Value>, String> {
    let dir = recordings_root(&app).join(&id);
    Ok(read_annotations(&dir))
}

/// 整体覆写标注文件（前端持有完整列表，增删改后整体保存）。
#[tauri::command]
fn save_annotations(app: AppHandle, id: String, annotations: Vec<Value>) -> Result<(), String> {
    let dir = recordings_root(&app).join(&id);
    write_annotations(&dir, &annotations)
}

/// 纯逻辑：合并 meta 到 session.json（空串/null 删除字段，否则覆盖），返回写回后的 session。
fn merge_session_meta(path: &Path, meta: &Value) -> Result<Value, String> {
    let mut v = serde_json::from_str::<Value>(
        &fs::read_to_string(path).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if let (Some(obj), Some(m)) = (v.as_object_mut(), meta.as_object()) {
        for (k, val) in m {
            // 空字符串 / null = 删除该字段（清理），否则覆盖
            if (val.is_string() && val.as_str() == Some("")) || val.is_null() {
                obj.remove(k);
            } else {
                obj.insert(k.clone(), val.clone());
            }
        }
    }
    fs::write(path, v.to_string()).map_err(|e| e.to_string())?;
    Ok(v)
}

/// 合并更新 session.json 的元信息字段（name/note/tags 等），返回写回后的完整 session。
#[tauri::command]
fn update_session_meta(app: AppHandle, id: String, meta: Value) -> Result<Value, String> {
    let path = recordings_root(&app).join(&id).join("session.json");
    merge_session_meta(&path, &meta)
}

/// 纯逻辑：读会话目录组装成 export bundle（不依赖 AppHandle，便于测试）。
fn build_export_bundle(dir: &Path) -> Result<Value, String> {
    let session = serde_json::from_str::<Value>(
        &fs::read_to_string(dir.join("session.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let windows: Vec<Value> = fs::read_to_string(dir.join("windows.jsonl"))
        .ok()
        .and_then(|s| s.lines().map(|l| serde_json::from_str(l).ok()).collect())
        .unwrap_or_default();
    let mut segments = serde_json::Map::new();
    let seg_dir = dir.join("segments");
    if let Ok(entries) = fs::read_dir(&seg_dir) {
        for e in entries.flatten() {
            let path = e.path();
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            let events: Vec<Value> = fs::read_to_string(&path)
                .ok()
                .and_then(|s| s.lines().map(|l| serde_json::from_str(l).ok()).collect())
                .unwrap_or_default();
            segments.insert(name, Value::Array(events));
        }
    }
    let annotations = read_annotations(dir);
    Ok(json!({
        "format": "rrweb-demo-session",
        "version": 1,
        "exportedAt": now_ms(),
        "session": session,
        "windows": windows,
        "segments": segments,
        "annotations": annotations,
    }))
}

/// 导出会话为单文件 JSON bundle。前端拿到后用 Blob 下载，零云依赖。
#[tauri::command]
fn export_session(app: AppHandle, id: String) -> Result<Value, String> {
    let dir = recordings_root(&app).join(&id);
    build_export_bundle(&dir)
}

/// 分配不与本地已有会话冲突的新 id（毫秒时间戳，冲突则自增）。
fn unique_id(root: &Path) -> String {
    let mut id = now_ms();
    loop {
        let s = id.to_string();
        if !root.join(&s).exists() {
            return s;
        }
        id += 1;
    }
}

/// 纯逻辑：把 bundle 重建为目标目录的文件结构（session.id 替换为 new_id，标记 importedAt）。
fn write_import_bundle(dir: &Path, bundle: &Value, new_id: &str) -> Result<(), String> {
    fs::create_dir_all(dir.join("segments")).map_err(|e| e.to_string())?;
    // session.json：保留原始 meta，替换 id 并标记导入时间
    let mut session = bundle["session"].clone();
    if let Some(obj) = session.as_object_mut() {
        obj.insert("id".into(), json!(new_id));
        obj.insert("importedAt".into(), json!(now_ms()));
    }
    fs::write(dir.join("session.json"), session.to_string()).map_err(|e| e.to_string())?;
    // windows.jsonl
    if let Some(wins) = bundle["windows"].as_array() {
        let mut body = String::new();
        for w in wins {
            body.push_str(&w.to_string());
            body.push('\n');
        }
        fs::write(dir.join("windows.jsonl"), body).map_err(|e| e.to_string())?;
    }
    // segments/<name>.jsonl
    if let Some(segs) = bundle["segments"].as_object() {
        for (name, events) in segs {
            if let Some(arr) = events.as_array() {
                let mut body = String::new();
                for e in arr {
                    body.push_str(&e.to_string());
                    body.push('\n');
                }
                fs::write(dir.join("segments").join(format!("{}.jsonl", name)), body)
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    // annotations.jsonl
    if let Some(annos) = bundle["annotations"].as_array() {
        write_annotations(dir, annos)?;
    }
    Ok(())
}

/// 导入会话 JSON bundle：解析后重建目录到 recordings/<newId>/，返回新 id。
/// 新 id 避免与本地会话冲突；session.json 标记 importedAt。
#[tauri::command]
fn import_session(app: AppHandle, content: String) -> Result<String, String> {
    let v = serde_json::from_str::<Value>(&content).map_err(|e| e.to_string())?;
    if v["format"].as_str() != Some("rrweb-demo-session") {
        return Err("不是有效的会话文件（缺少 format: rrweb-demo-session）".into());
    }
    let root = recordings_root(&app);
    let new_id = unique_id(&root);
    let dir = root.join(&new_id);
    write_import_bundle(&dir, &v, &new_id)?;
    Ok(new_id)
}

#[tauri::command]
fn delete_session(app: AppHandle, id: String) -> Result<(), String> {
    let dir = recordings_root(&app).join(&id);
    fs::remove_dir_all(&dir).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_jsonl(path: &Path, rows: &[Value]) {
        let mut body = String::new();
        for r in rows {
            body.push_str(&r.to_string());
            body.push('\n');
        }
        fs::write(path, body).unwrap();
    }

    /// 导出 -> 导入闭环：建一个完整会话目录，导出为 bundle，再导入到新目录，
    /// 验证 session id 替换、原字段保留、windows/segments/annotations 均还原。
    #[test]
    fn export_import_roundtrip() {
        let root = tempdir().unwrap();
        let src = root.path().join("session-001");
        fs::create_dir_all(src.join("segments")).unwrap();
        fs::write(
            src.join("session.json"),
            json!({"id":"session-001","source":"self","startedAt":1000,"endedAt":2000})
                .to_string(),
        )
        .unwrap();
        write_jsonl(
            &src.join("windows.jsonl"),
            &[
                json!({"type":"shown","label":"main","segmentId":"main#0","t":1000}),
                json!({"type":"hidden","label":"main","segmentId":"main#0","t":2000}),
            ],
        );
        write_jsonl(
            &src.join("segments/main#0.jsonl"),
            &[
                json!({"type":2,"timestamp":1000}),
                json!({"type":6,"timestamp":1100,"data":{"plugin":"console"}}),
            ],
        );
        write_annotations(
            &src,
            &[json!({"id":"a1","t":500,"text":"note here","author":"local","createdAt":1200})],
        )
        .unwrap();

        // 导出
        let bundle = build_export_bundle(&src).unwrap();
        assert_eq!(bundle["format"], "rrweb-demo-session");
        assert_eq!(bundle["session"]["id"], "session-001");
        assert_eq!(bundle["session"]["source"], "self");
        assert_eq!(bundle["windows"].as_array().unwrap().len(), 2);
        assert_eq!(bundle["segments"]["main#0"].as_array().unwrap().len(), 2);
        assert_eq!(bundle["annotations"].as_array().unwrap().len(), 1);
        assert_eq!(bundle["annotations"][0]["text"], "note here");

        // 导入到新目录
        let dst = root.path().join("session-002");
        write_import_bundle(&dst, &bundle, "session-002").unwrap();

        // session id 替换、原字段保留、importedAt 标记
        let session: Value =
            serde_json::from_str(&fs::read_to_string(dst.join("session.json")).unwrap()).unwrap();
        assert_eq!(session["id"], "session-002");
        assert_eq!(session["source"], "self");
        assert_eq!(session["startedAt"], 1000);
        assert_eq!(session["endedAt"], 2000);
        assert!(session["importedAt"].as_i64().unwrap() > 0);

        // windows / segments / annotations 还原
        assert_eq!(
            fs::read_to_string(dst.join("windows.jsonl"))
                .unwrap()
                .lines()
                .count(),
            2
        );
        assert_eq!(
            fs::read_to_string(dst.join("segments/main#0.jsonl"))
                .unwrap()
                .lines()
                .count(),
            2
        );
        let annos = read_annotations(&dst);
        assert_eq!(annos.len(), 1);
        assert_eq!(annos[0]["text"], "note here");
        assert_eq!(annos[0]["author"], "local");
    }

    /// 元信息合并：覆盖已有字段、新增字段、空串删除字段。
    #[test]
    fn merge_session_meta_overrides_adds_and_clears() {
        let root = tempdir().unwrap();
        let path = root.path().join("session.json");
        fs::write(
            &path,
            json!({"id":"s1","startedAt":100,"name":"old"}).to_string(),
        )
        .unwrap();

        // 覆盖 name + 新增 note/tags
        let v = merge_session_meta(
            &path,
            &json!({"name":"new name","note":"a note","tags":["bug","login"]}),
        )
        .unwrap();
        assert_eq!(v["name"], "new name");
        assert_eq!(v["note"], "a note");
        assert_eq!(v["tags"].as_array().unwrap().len(), 2);
        assert_eq!(v["startedAt"], 100); // 原字段不受影响

        // 空串 = 删除字段
        let v2 = merge_session_meta(&path, &json!({"name":""})).unwrap();
        assert!(v2.get("name").is_none());
        assert!(v2.get("note").is_some()); // 其他字段保留
    }
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
            list_annotations,
            save_annotations,
            update_session_meta,
            export_session,
            import_session,
            ingest::get_ingest_config,
            ingest::set_ingest_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use std::sync::Mutex;
