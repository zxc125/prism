# Tauri Plugin

`tauri-plugin-observer`（Rust）+ `@prism-obs/observer-tauri`（JS）—— 给 Tauri 2 桌面应用装上**多窗口**录制协调，经 HTTP 上报到 console。

> **适用版本**：`@prism-obs/observer-tauri` **0.6.x**（依赖 `@prism-obs/observer-sdk` 0.4.x）/ `tauri-plugin-observer` **0.5.x**。

**本页你将完成**：Rust 侧装插件 → JS 侧每个窗口调 `initTauri()` → capabilities 授权 → 多窗口跑通。前置：一个能跑的 Tauri 2 应用；console 已按 [快速开始](./quickstart) 跑通。

## 两种模式

| 模式 | 用途 | 落盘 | 谁用 |
| --- | --- | --- | --- |
| **Local** | console 自录 | Rust 直接落 `appDataDir/recordings/` | console 自身 |
| **Remote** | 外部 Tauri 应用 | 不落本地盘，前端 `HttpSink` 上报 console | **你的应用** |

外部应用一律用 **Remote**：Rust 只管窗口协调 + 状态 + 事件驱动，事件流经 HttpSink 跨进程上报。

## 安装

Rust 侧（`src-tauri/Cargo.toml`）：

```toml
[dependencies]
tauri-plugin-observer = "0.2"
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
| `source` | `"self"` | 写入 session.json 的 `source`；外部应用本地落盘建议 `"tauri"` |
| `app_id` | `None` | 写入 session.json 的 `appId`；`None` 时省键（P16） |
| `dir_base` | `AppData` | Local 落盘基目录：`AppData`（`appDataDir`）或 `ResourceDir`（资源目录，Windows NSIS 安装布局下为主程序 exe 所在目录）（P20） |
| `dir_name` | `"recordings"` | 基目录下的子目录名（P20）；**切换落盘根不迁移历史会话**——旧根下的会话不再出现在 `listSessions`/导出中（数据不删） |

## JS 侧：`initTauri()`

**每个窗口**都要调用一次。主窗口传 `autoStart: true`（启动会话并广播）；子窗口不传（等主窗口的广播自动启动）。

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

// 停止本窗口录制并 flush 已缓冲事件；会话结束由插件 stop_session 广播驱动
await ctrl.stop();
```

### `initTauri()` 选项全集

| 选项 | 类型 | 必填 | 默认 | 说明 |
| --- | --- | --- | --- | --- |
| `mode` | `"remote" \| "local"` | ➖ | `"remote"` | 部署模式：`"remote"` 经 HttpSink 上报 console；`"local"` 插件 Rust 直接落盘到本应用落盘根 `<dir_base>/<dir_name>`（默认 `appDataDir/recordings/`，见下文「本地落盘」） |
| `appId` | `string` | ✅ | — | 应用标识，随会话上报，console 侧区分来源；local 模式下本地 session.json 的 source/appId 由 Rust 侧 `ObserverConfig` 决定 |
| `endpoint` | `string` | ✅ | — | console 本地 HTTP server 地址，如 `http://127.0.0.1:1421`（remote 必填，local 忽略） |
| `token` | `string` | ➖ | — | 本地鉴权 token；console 开启鉴权时必传 |
| `env` | `string` | ➖ | — | 环境标记 |
| `release` | `string` | ➖ | — | 版本标记 |
| `autoStart` | `boolean` | ➖ | — | **主窗口传 `true`**：启动会话（HttpSink.startSession + 插件 bind_session 广播）。**子窗口不传**：等待 `recording-session` 广播拿到 sessionId 后自启 |
| `signals` | `SignalSet` | ➖ | `"all"` | 诊断信号开关，同 Web SDK |
| `recording` | `RecordingOptions` | ➖ | — | 录制量控（三档 profile + domBlocks），同 [Web SDK](./web#录制量控-recording)；缺省 = 全量录制 |
| `gating` | `"manual"` | ➖ | — | 手动档段门控（见[下节](#手动档段门控-p17)）：会话广播与挂载兜底不再自动开段，段边界由宿主经 controller 驱动；缺省 = 常录 |
| `meta` | `object` | ➖ | — | 透传到 session meta 的额外字段 |

机制：主窗口 `autoStart` 从 console server 取得 sessionId 后经插件 `bind_session` 广播；各窗口监听 `recording-session` / `segment` / `observer-lifecycle` 事件，驱动 `SegmentRecorder` 开 / 停段，经 `HttpSink` 上报。窗口隐藏 / 聚焦由 Rust 检测后 emit，前端转发上报。

### 手动档段门控（P17）

默认行为是**会话活跃即持续录制**。宿主想自己控制段边界（交互开段、空闲停段、异常兜底）时，加 `gating: "manual"`：`recording-session{active}` 广播与挂载兜底不再自动开段，段边界改由 controller 驱动；窗口复用/隐藏驱动的 `segment` 事件不受影响（复用显示仍开新段、隐藏仍停段）。

controller 在 `stop()` / `listSessions()` / `exportSession()` 之外提供：

| 成员 | 作用 |
| --- | --- |
| `active` | 段是否活跃（只读） |
| `startSegment()` | 手动开段（幂等，段已活跃时早退） |
| `stopSegment()` | 手动停段（幂等，不补 hidden 生命周期——那是窗口隐藏语义） |
| `signal(plugin, payload)` | 注入 type:6 诊断信号进当前段流；段未活跃时丢弃 |

最小门控策略（交互开段 + 30s 判闲停段 + 空闲异常兜底）：

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

// 空闲期异常兜底：开段后注入（signal 段未活跃时丢弃；段内异常由信号 hook 捕获，勿重复注入）
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

完整可跑样例：[examples/tauri-demo](https://github.com/zxc125/prism/tree/main/examples/tauri-demo) 的 `VITE_OBSERVER_GATING=manual`（判闲节拍与异常兜底全套）。

::: tip 为什么需要门控
`recording` 量控只控制「段开着时录什么」；**门控是空闲期零录制成本的唯一手段**（空闲期零观察、零序列化、零落盘）。高频渲染场景的完整打法见 [Web SDK 手册 · 高频渲染场景](./web#高频渲染场景)。
:::

### 会话级启停与用户归属（P22）

默认会话边界是进程启动（`autoStart`）到退出。宿主想按自己的策略划会话——最典型是**登录~登出区间，会话归属到当前用户**——用 controller 的会话级方法：

| 成员 | 作用 |
| --- | --- |
| `startSession(meta?)` | 启动新会话（幂等：已活跃先自动收口再开新会话）；`meta` 注入会话元数据 |
| `stopSession()` | 结束当前会话；监听不销毁，进程内可再次 `startSession` |

```ts
// 登录成功：开一个归属到该用户的会话
await ctrl.startSession({ user: { id: user.id, name: user.name } });
// 登出：收口当前会话（Local 写 endedAt；Remote 广播停各窗 + 主窗上报会话结束）
await ctrl.stopSession();
```

语义与规则：

- **meta 保留键**：`id` / `startedAt` / `source` / `appId` 是平台身份字段，宿主 meta 不可覆盖；非 object meta 在 Local 模式会被 `startSession` 直接拒绝（Remote 由 TS 类型约束，服务端忽略非 object body）。`user` 形态为 `{ id, name? }`，建议只放 id + 姓名，勿带账号/手机号——`user` 明文写入 session.json（Local）或上报 body（Remote），入镜范围由宿主自审。
- **与 `autoStart` 的关系**：改用 `startSession` 划界时主窗口可不传 `autoStart`，登录后再开会话；两者同用也不冲突（`startSession` 幂等收口）。
- **幂等收口的广播窗口**：已活跃时 `startSession` 先停旧会话再开新会话，各窗口经历一次 `active:false→true`；`gating: "manual"` 宿主无感，默认档有一次 <100ms 的空段窗口（重启段快照），正常使用无感。

### 本地落盘（Local 模式，P16）

不想连 console server？`mode: "local"` 让插件 Rust 把事件流直接写到本应用落盘根 `<dir_base>/<dir_name>`（默认 `appDataDir/recordings/`，目录结构与 console 同构），数据不离开本机。Rust 侧装插件时声明会话元数据与落盘根：

```rust
tauri_plugin_observer::init_with(tauri_plugin_observer::ObserverConfig {
    mode: tauri_plugin_observer::Mode::Local,
    source: "tauri".into(),               // 写入 session.json 的 source
    app_id: Some("my-tauri-app".into()),  // 写入 session.json 的 appId（None 省键）
    // 落盘根 = <dir_base>/<dir_name>，默认 appDataDir/recordings（P20）：
    // dir_base: tauri_plugin_observer::DirBase::ResourceDir,  // 落盘随安装目录（Windows NSIS 布局 = exe 所在目录）
    // dir_name: "prism".into(),
    ..Default::default()
})
```

::: warning 切换落盘根不迁移
`dir_base`/`dir_name` 进程期固定；改为非默认根后，旧 `appDataDir/recordings` 下的历史会话不再出现在 `listSessions()`/导出里（数据不删，改回默认根即恢复可见）。
:::

JS 侧 `initTauri({ mode: "local", appId, autoStart, ... })`（`endpoint` / `token` 忽略）。之后随时把会话闭环回 console：

```ts
const sessions = await ctrl.listSessions();     // 本地会话元信息列表（仅 Local 可用）
const bundle = await ctrl.exportSession(id);    // prism-session bundle JSON（仅 Local 可用）
// 大会话推荐：插件直接写盘，返回写入字节数（仅 Local 可用，P18；path 常来自系统保存对话框）
const bytes = await ctrl.exportSessionToFile(id, "/path/to/session.bundle.json");
// 自行落盘 / 传输后，在 console 会话页「导入」即可回放 + 看诊断信号
```

**上报地址热切**：endpoint / token 可存 localStorage，提供配置 UI 切换后 reload 即可（本地 server ↔ 云端 observer-server 自由切）。[examples/tauri-demo](https://github.com/zxc125/prism/tree/main/examples/tauri-demo) 里有现成的配置 UI 可抄。

## Capabilities 授权

::: danger 每个窗口 label 都必须列入 capabilities
Tauri 2 的 capabilities 按**窗口 label**（或 glob）授权。任何窗口——包括动态开的子窗口——只要 label 没出现在 capabilities 的 `windows` 列表里，就**没有插件调用权限**，典型报错是 `plugin:observer|begin_segment not allowed`。新增带新 label 模式的路由时，记得同步改这个文件。
:::

插件命令需授权 `observer:default`。文件在 `src-tauri/capabilities/`（脚手架默认生成 `default.json`）：

```json
{
  "identifier": "default",
  "windows": ["main", "child-*"],
  "permissions": ["observer:default"]
}
```

## 路由为什么用 hash（`index.html#{route}`）

新窗口加载 `index.html` 后靠 hash 定位到对应视图（如 `index.html#/child/123`）。这是因为 Tauri 通过自定义协议提供页面，**history 路由在窗口刷新或深链打开时会 404**——hash 由前端解析，不经过服务器，天然没有这个问题。上例 Rust `open_window` 的 `initialization_script` 就是把路由写进 hash。

## 多窗口行为

- **关闭子窗口 = 隐藏**：录制期间，子窗口的 `CloseRequested` 被拦截为 `hide()` + 记 `hidden`；再次 `open_window` 复用时 `show()` + 开新段。
- **主窗口关闭 = 退出进程**（不拦截）。
- **跨窗口对齐**：所有窗口共享墙上时钟，事件带绝对 `timestamp`，回放时按 shown/hidden 区间在主时间轴同步驱动各段。

## 故障排查

| 现象 | 多半是 | 解法 |
| --- | --- | --- |
| 报 `plugin:observer|… not allowed` | 窗口 label 没进 capabilities | 把该 label（或匹配的 glob）加进 `src-tauri/capabilities/*.json` 的 `windows` |
| console 里根本没有会话出现 | 主窗口没传 `autoStart: true`，或 endpoint / token 不对 | 确认主窗口判定逻辑（上例按 hash 判定）；对照 console 设置页 |
| 子窗口不录 | 子窗口没调 `initTauri()`，或 label 无权限 | 每个窗口的入口都要调；检查 capabilities |
| 会话有了但窗口只有一条轨道 | 子窗口 label 与主窗口相同被单实例复用 | 不同 label = 不同轨道；检查 `window_label()` 推导 |
| 录制开启后空闲期也有事件落盘 | 未配 `gating: "manual"`（缺省会话活跃即常录） | 见[手动档段门控](#手动档段门控-p17)；高频渲染另见 [Web SDK · 高频渲染场景](./web#高频渲染场景) |

## 完整示例

完整可跑样例见仓库 [`examples/tauri-demo`](https://github.com/zxc125/prism/tree/main/examples/tauri-demo)（多窗口 + 上报地址热切配置 UI，跑法见其 README）。
