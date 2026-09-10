# Tauri Plugin

`tauri-plugin-observer` (Rust) + `@prism-obs/observer-tauri` (JS) — add **multi-window** recording coordination to a Tauri 2 desktop app, reporting to the console over HTTP.

> **Applies to**: `@prism-obs/observer-tauri` **0.5.x** / `tauri-plugin-observer` **0.3.x**.

**In this page**: register the plugin on the Rust side → call `initTauri()` in every window → grant capabilities → run multi-window. Prerequisite: a working Tauri 2 app; the console running per [Quick Start](./quickstart).

## Two modes

| Mode | Use | Storage | Used by |
| --- | --- | --- | --- |
| **Local** | console self-recording | Rust writes to `appDataDir/recordings/` | the console itself |
| **Remote** | external Tauri app | none local; frontend `HttpSink` reports to console | **your app** |

External apps always use **Remote**: Rust only coordinates windows + state + events; the event stream crosses processes via HttpSink.

## Install

Rust (`src-tauri/Cargo.toml`):

```toml
[dependencies]
tauri-plugin-observer = "0.2"
```

JS:

```sh
pnpm add @prism-obs/observer-tauri @prism-obs/observer-sdk
```

## Rust: register plugin + open windows

Init the plugin in Remote mode and provide an `open_window` command (same label = single-instance focus; different label = new instance):

```rust
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// Derive the window label from a route: / -> main, /child/123 -> child-123
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

`ObserverConfig` fields:

| Field | Default | Notes |
| --- | --- | --- |
| `mode` | `Local` | external apps set `Remote` |
| `main_label` | `"main"` | main window label — closing it exits the process (never intercepted as hide) |
| `skip_focus_prefix` | `""` | label prefix to skip focus recording for (e.g. player windows) |

## JS: `initTauri()`

Call once **in every window**. The main window passes `autoStart: true` (creates the session + broadcasts); child windows omit it (they self-start on the broadcast).

```ts
import { initTauri } from "@prism-obs/observer-tauri";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

const label = getCurrentWebviewWindow().label;
const isMain = !window.location.hash || window.location.hash === "#/";

const ctrl = await initTauri({
  appId: "my-tauri-app",
  endpoint: "http://127.0.0.1:1421",   // console local server
  token: "<optional token>",
  env: "dev",
  release: "1.0.0",
  autoStart: isMain,                    // main window creates the session and broadcasts
});

// stop this window's recording and flush buffered events; session end is driven
// by the plugin's stop_session broadcast
await ctrl.stop();
```

### Full `initTauri()` options

| Option | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `mode` | `"remote" \| "local"` | ➖ | `"remote"` | deployment mode: `"remote"` reports to the console via HttpSink; `"local"` makes the plugin's Rust write straight to this app's `appDataDir/recordings/` (see "Local disk" below) |
| `appId` | `string` | ✅ | — | app identifier, reported with the session; in local mode the session.json source/appId come from the Rust-side `ObserverConfig` |
| `endpoint` | `string` | ✅ | — | console local HTTP server, e.g. `http://127.0.0.1:1421` (required in remote mode, ignored in local) |
| `token` | `string` | ➖ | — | local auth token; required when console auth is on |
| `env` | `string` | ➖ | — | environment tag |
| `release` | `string` | ➖ | — | release tag |
| `autoStart` | `boolean` | ➖ | — | **main window: `true`** — starts the session (HttpSink.startSession + plugin `bind_session` broadcast). **child windows: omit** — they self-start once the `recording-session` broadcast arrives |
| `signals` | `SignalSet` | ➖ | `"all"` | signal switches, same as the Web SDK |
| `recording` | `RecordingOptions` | ➖ | — | recording profile (three profiles + domBlocks), same as the [Web SDK](./web#recording); omitted = full recording |
| `gating` | `"manual"` | ➖ | — | manual segment gating (see [below](#manual-segment-gating-p17)): the session broadcast and the mount fallback no longer auto-open segments — the host drives boundaries via the controller; default = record continuously |
| `meta` | `object` | ➖ | — | extra fields forwarded into session meta |

Mechanism: the main window's `autoStart` gets a sessionId from the console server and broadcasts it via the plugin's `bind_session`; every window listens for `recording-session` / `segment` / `observer-lifecycle` events to drive `SegmentRecorder` start/stop, reporting via `HttpSink`. Window hide/focus is detected by Rust and forwarded.

### Manual segment gating (P17)

The default behavior is **record continuously while the session is active**. If the host wants to own segment boundaries (open on interaction, stop when idle, exception fallback), pass `gating: "manual"`: the `recording-session{active}` broadcast and the mount fallback no longer auto-open segments — boundaries are driven through the controller instead. Window reuse/hide `segment` events are unaffected (re-show still opens a new segment, hide still stops).

Beyond `stop()` / `listSessions()` / `exportSession()`, the controller exposes:

| Member | Purpose |
| --- | --- |
| `active` | whether a segment is currently open (read-only) |
| `startSegment()` | manually open a segment (idempotent, no-op when active) |
| `stopSegment()` | manually close the segment (idempotent; does not report hidden — that's window-visibility semantics) |
| `signal(plugin, payload)` | inject a type:6 diagnostic signal into the current stream; dropped when no segment is open |

Minimal gating policy (open on interaction, stop after 30s idle, idle-exception fallback):

```ts
const ctrl = await initTauri({
  appId: "my-tauri-app",
  endpoint: "http://127.0.0.1:1421",
  autoStart: isMain,
  gating: "manual",
});

let lastActiveAt = Date.now();
window.addEventListener(
  "pointerdown",
  () => {
    lastActiveAt = Date.now();
    if (!ctrl.active) void ctrl.startSegment();
  },
  { capture: true, passive: true },
);

// idle-exception fallback: open a segment, then inject (signal is dropped while no
// segment is open; in-segment errors are already captured by the signal hooks)
window.addEventListener("error", (ev) => {
  if (ctrl.active) return;
  void ctrl.startSegment().then(() =>
    ctrl.signal("error", { message: ev.message, stack: ev.error?.stack }),
  );
});

setInterval(() => {
  if (ctrl.active && Date.now() - lastActiveAt > 30_000) void ctrl.stopSegment();
}, 1_000);
```

A complete runnable sample: [`examples/tauri-demo`](https://github.com/zxc125/prism/tree/main/examples/tauri-demo) with `VITE_OBSERVER_GATING=manual` (idle timer and exception fallback included).

::: tip Why gating
`recording` tuning only controls **what gets recorded while a segment is open**; gating is the only way to make idle time cost nothing (no observing, no serialization, no writes). For hot-rendering pages see the [Web SDK manual · High-frequency rendering](./web#high-frequency-rendering).
:::

### Local disk (Local mode, P16)

Don't want to reach the console server? `mode: "local"` makes the plugin's Rust write the event streams straight into **this app's own** `appDataDir/recordings/` (same layout as the console); data never leaves the machine. Declare session metadata when installing the plugin on the Rust side:

```rust
tauri_plugin_observer::init_with(tauri_plugin_observer::ObserverConfig {
    mode: tauri_plugin_observer::Mode::Local,
    source: "tauri".into(),               // written into session.json as `source`
    app_id: Some("my-tauri-app".into()),  // written as `appId` (key omitted when None)
    ..Default::default()
})
```

JS side: `initTauri({ mode: "local", appId, autoStart, ... })` (`endpoint` / `token` ignored). Close the loop back to the console any time:

```ts
const sessions = await ctrl.listSessions();     // local session metadata list (Local mode only)
const bundle = await ctrl.exportSession(id);    // prism-session bundle JSON (Local mode only)
// large sessions: let the plugin write the file directly, returns bytes written (Local mode only;
// the path usually comes from the system save dialog)
const bytes = await ctrl.exportSessionToFile(id, "/path/to/session.bundle.json");
// save / transport it yourself, then import it on the console sessions page for replay + diagnostics
```

**Hot-switch the endpoint**: store endpoint/token in localStorage, provide a config UI, reload to re-init (local server ↔ cloud observer-server). [examples/tauri-demo](https://github.com/zxc125/prism/tree/main/examples/tauri-demo) ships a ready-made config UI you can copy.

## Capabilities

::: danger Every window label must be listed in capabilities
Tauri 2 authorizes capabilities per **window label** (or glob). Any window — including dynamically opened child windows — whose label is missing from the capabilities `windows` list has **no permission for plugin commands**; the typical error is `plugin:observer|begin_segment not allowed`. When you add a route with a new label pattern, update this file too.
:::

Plugin commands need `observer:default`. The file lives in `src-tauri/capabilities/` (scaffolding generates `default.json`):

```json
{
  "identifier": "default",
  "windows": ["main", "child-*"],
  "permissions": ["observer:default"]
}
```

## Why routes use hash (`index.html#{route}`)

New windows load `index.html` and locate their view via the hash (e.g. `index.html#/child/123`). Tauri serves pages over a custom protocol, and **history routing 404s on window reload or deep links** — the hash is parsed by the frontend and never hits a server, which is exactly why it's used. The `initialization_script` in the Rust `open_window` above writes the route into the hash.

## Multi-window behavior

- **Closing a child window = hide**: during recording, a child's `CloseRequested` is intercepted as `hide()` + a `hidden` lifecycle entry; reopening via `open_window` `show()`s it and opens a new segment.
- **Closing the main window = exit** (not intercepted).
- **Cross-window alignment**: all windows share wall-clock time; events carry absolute `timestamp`s, aligned on the main timeline by shown/hidden spans on replay.

## Troubleshooting

| Symptom | Likely cause | Fix |
| --- | --- | --- |
| `plugin:observer|… not allowed` | window label missing from capabilities | add the label (or a matching glob) to `src-tauri/capabilities/*.json` `windows` |
| No session ever appears in the console | main window didn't pass `autoStart: true`, or endpoint/token wrong | verify the main-window detection (hash-based in the example); cross-check console Settings |
| A child window doesn't record | `initTauri()` not called there, or label lacks permission | call it in every window's entry; check capabilities |
| Session exists but only one lane | child window shares the main window's label and got reused | different label = different lane; check `window_label()` |
| Events land on disk even while idle | `gating: "manual"` not set (default records while the session is active) | see [Manual segment gating](#manual-segment-gating-p17); for hot rendering also [Web SDK · High-frequency rendering](./web#high-frequency-rendering) |

## Full example

Full runnable sample: [`examples/tauri-demo`](https://github.com/zxc125/prism/tree/main/examples/tauri-demo) (multi-window + hot-switch config UI; see its README).
