# Tauri Plugin

`tauri-plugin-observer` + `@prism-obs/observer-tauri` —— 给 Tauri 2 桌面应用装上多窗口录制协调，经 HTTP 上报到 console。

## 两种模式

| 模式 | 用途 | 落盘 | 谁用 |
| --- | --- | --- | --- |
| **Local** | console 自录 | Rust 直接落 `appDataDir/recordings/` | console 自身 |
| **Remote** | 外部 Tauri 应用 | 不落本地盘，前端 `HttpSink` 上报 console | **你的应用** |

外部应用一律用 **Remote**：Rust 只管窗口协调 + 状态 + 事件驱动，事件流经 HttpSink 跨进程上报。

## 安装

Rust 侧（`Cargo.toml`）：

```toml
[dependencies]
tauri-plugin-observer = "0.1"
```

JS 侧：

```sh
pnpm add @prism-obs/observer-tauri @prism-obs/observer-sdk
```

## Rust 侧：装插件 + 开窗

用 Remote 模式初始化插件，并提供一个 `open_window` 命令（相同 label = 单实例聚焦，不同 label = 多实例）：

```rust
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// 由路由推导窗口 label：/ -> main，/child/123 -> child-123
fn window_label(route: &str) -> String {
    let label = route.trim_start_matches('/').replace('/', "-");
    if label.is_empty() { "main".to_string() } else { label }
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
        .title(format!("App · {}", label))
        .inner_size(640.0, 480.0)
        .initialization_script(&init_script)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(label)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_observer::init_with(
            tauri_plugin_observer::ObserverConfig {
                mode: tauri_plugin_observer::Mode::Remote,
                ..Default::default()
            },
        ))
        .invoke_handler(tauri::generate_handler![open_window])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

`ObserverConfig` 字段：

| 字段 | 默认 | 说明 |
| --- | --- | --- |
| `mode` | `Local` | 外部应用设 `Remote` |
| `main_label` | `"main"` | 主窗口 label，其关闭 = 退出进程（不拦截为隐藏） |
| `skip_focus_prefix` | `""` | 跳过 focus 记录的 label 前缀（如回放窗口） |

## JS 侧：`initTauri()`

**每个窗口**调用一次。主窗口传 `autoStart: true`（建会话 + 广播 sessionId）；子窗口不传（等广播自启）。

```ts
import { initTauri } from "@prism-obs/observer-tauri";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

const label = getCurrentWebviewWindow().label;
const isMain = !window.location.hash || window.location.hash === "#/";

const ctrl = await initTauri({
  appId: "my-tauri-app",
  endpoint: "http://127.0.0.1:1421",   // console 本地 server
  token: "<可选 token>",
  env: "dev",
  release: "1.0.0",
  autoStart: isMain,                    // 主窗口启动会话并广播
});

// 停止（触发插件 stop_session，广播各窗口停段）
await ctrl.stop();
```

机制：主窗口 `autoStart` 从 console server 取得 sessionId 后经插件 `bind_session` 广播；各窗口监听 `recording-session` / `segment` / `observer-lifecycle` 事件，驱动 `SegmentRecorder` 开 / 停段，经 `HttpSink` 上报。窗口隐藏 / 聚焦由 Rust 检测后 emit，前端转发上报。

**上报地址热切**：endpoint / token 可存 localStorage，提供配置 UI 切换后 reload 即可（本地 server ↔ 云端 observer-server 自由切）。

## Capabilities 授权

插件命令需在 capabilities 授权 `observer:default`：

```json
{
  "permissions": ["observer:default"]
}
```

## 多窗口行为

- **关闭子窗口 = 隐藏**：录制期间，子窗口的 `CloseRequested` 被拦截为 `hide()` + 记 `hidden`；再次 `open_window` 复用时 `show()` + 开新段。
- **主窗口关闭 = 退出进程**（不拦截）。
- **跨窗口对齐**：所有窗口共享墙上时钟，事件带绝对 `timestamp`，回放时按 shown/hidden 区间在主时间轴同步驱动各段。

完整可跑样例见仓库 [`examples/tauri-demo`](https://github.com/zxc125/prism/tree/main/examples/tauri-demo)。
