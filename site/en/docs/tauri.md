# Tauri Plugin

`tauri-plugin-observer` + `@prism-obs/observer-tauri` — add multi-window recording coordination to a Tauri 2 desktop app, reporting to the console over HTTP.

## Two modes

| Mode | Use | Storage | Used by |
| --- | --- | --- | --- |
| **Local** | console self-recording | Rust writes to `appDataDir/recordings/` | the console itself |
| **Remote** | external Tauri app | none local; frontend `HttpSink` reports to console | **your app** |

External apps always use **Remote**: Rust only coordinates windows + state + events; the event stream crosses processes via HttpSink.

## Install

Rust (`Cargo.toml`):

```toml
[dependencies]
tauri-plugin-observer = "0.1"
```

JS:

```sh
pnpm add @prism-obs/observer-tauri @prism-obs/observer-sdk
```

## Rust: register plugin + open windows

Init the plugin in Remote mode and provide an `open_window` command (same label = single-instance focus; different label = new instance):

```rust
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

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
        // reusing a hidden window: if recording, emit segment:start for a new segment
        tauri_plugin_observer::emit_segment_start_if_active(&app, &label);
        return Ok(label);
    }
    let init_script = format!("if (!window.location.hash) window.location.replace('#{route}');");
    WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("index.html".into()))
        .inner_size(640.0, 480.0)
        .initialization_script(&init_script)
        .build().map_err(|e| e.to_string())?;
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
        .run(tauri::generate_context!()).expect("error while running tauri application");
}
```

`ObserverConfig`: `mode` (set `Remote`), `main_label` (default `"main"` — closing it exits the process), `skip_focus_prefix` (skip focus recording for e.g. player windows).

## JS: `initTauri()`

Call once **in every window**. The main window passes `autoStart: true` (creates the session + broadcasts the sessionId); child windows omit it (they self-start on the broadcast).

```ts
import { initTauri } from "@prism-obs/observer-tauri";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

const isMain = !window.location.hash || window.location.hash === "#/";
const ctrl = await initTauri({
  appId: "my-tauri-app",
  endpoint: "http://127.0.0.1:1421",
  token: "<optional token>",
  autoStart: isMain,
});

await ctrl.stop();
```

Mechanism: the main window gets a sessionId from the console server, broadcasts it via the plugin's `bind_session`; every window listens for `recording-session` / `segment` / `observer-lifecycle` events to drive `SegmentRecorder` start/stop, reporting via `HttpSink`. Window hide/focus is detected by Rust and forwarded.

**Hot-switch the endpoint**: store endpoint/token in localStorage, provide a config UI, reload to re-init (local server ↔ cloud observer-server).

## Capabilities

Authorize `observer:default` in your capabilities file:

```json
{ "permissions": ["observer:default"] }
```

## Multi-window behavior

- **Closing a child window = hide**: during recording, a child's `CloseRequested` is intercepted as `hide()` + a `hidden` lifecycle entry; reopening via `open_window` `show()`s it and opens a new segment.
- **Closing the main window = exit** (not intercepted).
- **Cross-window alignment**: all windows share wall-clock time; events carry absolute `timestamp`s, aligned on the main timeline by shown/hidden spans on replay.

Full runnable sample: [`examples/tauri-demo`](https://github.com/zxc125/prism/tree/main/examples/tauri-demo).
