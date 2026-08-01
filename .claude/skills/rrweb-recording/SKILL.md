---
name: rrweb-recording
description: Guidance for the rrweb-based multi-window recording & replay system in this Tauri 2 + Vue 3 project. Covers implementation plan, current status, and unimplemented features. Use this skill whenever the user works on anything related to recording, replay, 回放, 录制, session/segment files, the useRecorder/usePlayer composables, PlayerView/MainView recording controls, the Rust session commands (start_session, stop_session, begin_segment, append_events, list_sessions, read_session, delete_session), the recordings/ data directory, or asks about the recording architecture, current state, or planned features. Trigger even when the user doesn't say "rrweb" explicitly-any mention of window recording, session replay, multi-window playback, or close-to-hide window behavior should activate this skill.
---

# rrweb 录制与回放系统

本 skill 记录该项目中 rrweb 录制/回放功能的**实现方案**、**当前现状**与**未实现功能**。在修改录制相关代码、排查回放问题或规划新功能前，先通读本 skill，避免破坏已有的跨窗口对齐与分段机制。

> 平台演进方向（本地优先观测平台：外部 web/tauri 观测、导出/标注/分享等）见 `docs/架构/` 与 `docs/阶段路径/`。Sink 传输抽象（P3）、Web SDK + 本地接收 server（P4）、Tauri Plugin（P5）、导出/标注/分享（P6）均已落地，见下文。

## 实现方案

### 核心思路

rrweb 在**前端 webview** 里跑（JS），录制它所在窗口的 DOM 变更与用户交互。Tauri 每个窗口 = 独立 webview = 独立 JS 上下文，所以**每个窗口各跑一个 `record()` 实例**，事件按 segment 流式落盘到本地文件。回放时用每窗口一个 `Replayer`，靠**绝对时间戳**对齐到同一主时间轴。

### 数据布局

录制产物存放在 Tauri `appDataDir/recordings/<sessionId>/`：

```
recordings/<sessionId>/
  session.json              # { id, startedAt, endedAt?, source?, name?, note?, tags?, importedAt? }
  windows.jsonl             # 窗口生命周期，每行一个 JSON
  segments/<label>#<n>.jsonl  # 每段事件流：DOM 事件 + 交错 type:6 诊断信号（error/console/network）
  annotations.jsonl         # 用户标注（P6，session 级，与事件流分离）：{ id, t, label?, text, author, createdAt }
```

- `windows.jsonl` 每行：`{ type: "shown"|"hidden"|"focus", label, segmentId?, t }`，`t` 为 epoch 毫秒。
- segmentId 格式 `<label>#<n>`，同一窗口每次重新显示 `n` 自增。一次 show~hide = 一段，每段自带一次全量快照，可独立回放。
- segment 文件内 DOM 事件与 `type:6` 诊断信号按 `timestamp` 自然交错，回放侧无需跨流对齐。

### 会话与段的生命周期

| 事件 | Rust 动作 | 前端动作 |
| --- | --- | --- |
| 主窗口点「开始录制」 | `start_session`：置 active、建目录、写 session.json、补记初始 focus、广播 `recording-session{active:true}` | 各窗口 `useRecorder` 收到广播 -> `sink.beginSegment()` -> 启动 rrweb + 安装信号 hook |
| 新窗口首次创建 | `open_window` build | 新窗口 `useRecorder` 挂载时 `sink.isRecordingActive()` 兜底为 true -> `sink.beginSegment()` |
| 已隐藏窗口再次打开 | `open_window`：`show()`+`focus()`，录制中则 `emit_to(label,"segment",{start})` | `useRecorder` 收到 start -> `sink.beginSegment()` 开新段 |
| 子窗口点 X 关闭（录制中） | `on_window_event` CloseRequested：`prevent_close`+`hide`、记 hidden、`emit_to` segment:stop | `useRecorder` 收到 stop -> 卸载信号 hook、调 rrweb stop fn、flush |
| 窗口聚焦 | `on_window_event` Focused：记 focus（跳过 `player-*`） | - |
| 主窗口关闭 | 不拦截 -> 退出进程 | - |
| 点「停止录制」 | `stop_session`：置 inactive、关闭所有活跃段、广播 `recording-session{active:false}`、写 endedAt | 各窗口停录 |

### 关键设计决策与原因

- **hash 模式路由 + `index.html#{route}`**：新窗口靠 hash 路由定位视图，Tauri 走自定义协议/本地文件，history 模式刷新会 404。
- **segment 而非整窗单文件**：子窗口「关闭=隐藏、再开重录」导致同一窗口多次活跃；每段独立快照才能正确回放每次显示的初始状态。
- **绝对时间戳对齐**：所有窗口共享墙上时钟，rrweb 事件自带 `timestamp`，无需额外同步协议。主窗口 click 与子窗口 shown 落在同一时间轴，「连接感」自然成立。
- **子窗口关闭改隐藏**：保留 webview 与 Vue 状态，`useRecorder` 实例不销毁，靠 Rust 定向事件驱动 start/stop 段。
- **player-* 窗口跳过录制**：回放窗口本身不能被录进会话，否则污染数据。
- **底层 `Replayer` 而非 `rrweb-player`**：后者是 Vue 2 组件，不能用于 Vue 3；自建 transport（自定义播放按钮 + Element Plus slider/select/switch）+ 平铺网格槽位，磁贴内 rrweb 内容保留白底以还原被录页面。
- **JSONL 格式**：可流式追加、断电不丢整文件、回放按行 parse。
- **稳定槽位 + spotlight 主窗口**：回放时每个 label 占一个固定网格槽位（不随 show/hide reflow）；主窗口由 focus 时间线自动跟踪或手动选择，占大格，其余为侧槽。主槽/侧槽切换只改 CSS class（`is-main` + `grid-row:1/-1`），**不 reparent** Replayer 容器--rrweb 用 iframe 承载 mirror，reparent 会触发重新加载。
- **动态 tile 元素不可用 scoped CSS**：`usePlayer` 用 `document.createElement` 创建 `.tile-slot/.tile-header/.tile-root/.tile-placeholder`，不带 scoped 的 `data-v` 属性，`<style scoped>` 选择器全部失配（曾导致 spotlight 不跨行 + 横向溢出）。相关规则放 PlayerView 的非 scoped `<style>` 块（以 `.grid` 限定作用域），并配 `min-width:0` + `minmax(0,...)` 列模板防 rrweb 原始宽度撑爆网格。
- **tile 等比缩放（方案 E）**：`.replayer-wrapper` 固定为录制视口尺寸（Meta 事件的 width/height），`transform: translate() scale()` fit-contain 到 `.tile-root` 并居中、`transform-origin: top left`；`ResizeObserver` 监听各 tile-root，spotlight 切主 / 窗口缩放 / 段显隐时重算。
- **漂移阈值纠偏**：各 Replayer 独立 RAF 与主时钟（`setInterval(50)` + `performance.now()`）可能漂移；`tick` 内读 `replayer.getCurrentTime()` 与期望值对比，超 `DRIFT_THRESHOLD`(120ms) 则 `replayer.play(expect)` re-seek 拉回。rrweb 2.x `play(offset)` 内部先 PAUSE 再 PLAY(offset)，seek+续播一次完成。
- **focus 时间线**：`windows.jsonl` 的 focus 事件驱动自动主窗口；`start_session` 时补记初始 focus（遍历 `webview_windows().is_focused()`），避免 t=0 时间线为空；player-* 窗口的 focus 已过滤，不产生孤儿记录。
- **交错诊断信号（type:6）**：error/console/network 信号以 rrweb plugin 事件（`type:6`，`data:{plugin,payload}`）交错进同一段事件流，与 DOM 共享绝对时间戳，无需跨流对齐。采集侧 `useRecorder` 在段录制期间安装 hook（error: `onerror`+`unhandledrejection`；console: patch log/warn/error/info/debug + args 序列化截断循环引用；network: patch `fetch`+`XMLHttpRequest`，默认不记 body/headers），经同一 `emit` 落盘，并带 `delay` 供 Replayer 调度安全。回放侧 `usePlayer` 从各 segment 收集 type:6 为统一信号流 + error 红标；rrweb Replayer 对无 handler 的 plugin 事件 no-op，不影响 DOM 回放。
- **Sink 传输抽象（P3）+ Web SDK（P4）**：采集逻辑（rrweb record + 信号 hook + 缓冲 flush）与落盘/上报解耦，经 `Sink` 接口（`startSession`/`beginSegment`/`appendEvents`/`appendLifecycle`/`endSession`/`isRecordingActive`）对接不同后端。采集核心（`Sink` 接口、`HttpSink`/`IndexedDBSink`、信号 hook、`SegmentRecorder`）下沉到 npm 包 [`packages/observer-sdk`](packages/observer-sdk)（`@prism-obs/observer-sdk`），self-obs 与外部 SDK 共用同一份 `SegmentRecorder`：差别仅在 Sink 注入与驱动方式。`TauriSink`（console 自录，进程内 invoke，零序列化）留在 [sink.ts](src/composables/sink.ts)；`HttpSink`（外部 SDK 上报 console 本地 HTTP server，按 segment 缓冲/达量或定时 flush/失败重试/`beforeunload` 用 `sendBeacon` 兜底）已对接 P4 的 `/ingest/*` server；`IndexedDBSink`（纯 web 独立回放，IDB 缓存）仍为预留骨架。`useRecorder(sink = new TauriSink())` 默认 TauriSink；外部 web 应用 `init({appId, endpoint, token, ...})` 自驱。会话级命令在 self-obs 由 Rust/MainView 驱动（useRecorder 不调 startSession/endSession），外部 SDK 用完整接口。

### 文件职责

| 文件 | 职责 |
| --- | --- |
| [src-tauri/src/lib.rs](src-tauri/src/lib.rs) | console 应用入口：`open_window`（路由->label、复用窗口调插件 `emit_segment_start_if_active`）、`list_sessions`/`read_session`/`delete_session`（读自身 recordings/）、`list_annotations`/`save_annotations`/`update_session_meta`/`export_session`/`import_session`（P6 标注/元信息/导出导入，核心逻辑抽纯函数 `build_export_bundle`/`write_import_bundle`/`merge_session_meta`，[annotations.rs](src-tauri/src/annotations.rs)）、声明 `ingest` 模块、管理 `IngestState`、启动接收 server、注册 `get/set_ingest_config`；装 `tauri-plugin-observer` Local 模式（`skip_focus_prefix:"player-"`） |
| [plugins/tauri-plugin-observer](plugins/tauri-plugin-observer) | 独立 crate `tauri-plugin-observer`：`Session` 状态、录制命令（`plugin:observer|*`）、`on_window_ready`+`Window::on_window_event` 生命周期拦截；`Mode::{Local,Remote}`；`storage` 模块（落盘函数，console ingest 复用）；`emit_segment_start_if_active` helper；权限 `observer:default` |
| [src-tauri/src/ingest.rs](src-tauri/src/ingest.rs) | 本地 HTTP 接收 server（tiny_http，串行）：`/ingest/{session,segment,events,lifecycle,session/end}` -> 复用插件 storage 落盘；`IngestConfig`/`IngestState`、token 鉴权、CORS、配置持久化、`handle_route`（可测试） |
| [packages/observer-sdk](packages/observer-sdk) | npm 包 `@prism-obs/observer-sdk`：`Sink` 接口 + 类型、`HttpSink`（含 `useSessionId` 注入）/`IndexedDBSink`、信号 hook（`installSignalHooks`）、`SegmentRecorder`、`init()`；self-obs / web SDK / tauri plugin 共用 |
| [packages/observer-tauri](packages/observer-tauri) | npm 包 `@prism/observer-tauri`：`initTauri()` Remote 模式驱动--监听插件 `recording-session`/`segment`/`observer-lifecycle` 事件驱动 `SegmentRecorder`，hidden/focus 经 `HttpSink.appendLifecycle` 上报；主窗口 `autoStart` 建 session + `bind_session` 广播 |
| [examples/tauri-demo](examples/tauri-demo) | P5 验证样例：独立 Tauri 2 应用装插件 Remote 模式，多窗口（main + child-*），`initTauri` 上报到 console |
| [src/composables/sink.ts](src/composables/sink.ts) | `TauriSink`（包装 `plugin:observer|*` invoke，self-obs）；从 observer-sdk re-export `Sink`/`HttpSink`/`IndexedDBSink` 及类型 |
| [src/composables/useRecorder.ts](src/composables/useRecorder.ts) | 每窗口录制器：监听会话广播与段事件、`player-*` 跳过；用 SDK 的 `SegmentRecorder`（注入 `TauriSink`）驱动 start/stop，无直接 `invoke` |
| [src/composables/usePlayer.ts](src/composables/usePlayer.ts) | 回放控制器：加载会话、按区间驱动各 `Replayer`、play/pause/seek/倍速、稳定槽位、spotlight 主窗口（auto focus + 手动）、漂移纠偏、时间轴色带数据、tile 等比缩放（`fitSegment` + `ResizeObserver`）；从 type:6 事件收集诊断信号流（`signals`/`errorMarks`）；导出 `LANE_COLORS` 与 `Signal` 类型并为每个槽位标 `--lane-color` |
| [src/App.vue](src/App.vue) | 挂载 `useRecorder`，使每个窗口都参与录制 |
| [src/styles/theme.css](src/styles/theme.css) | 全局设计系统：warm-dark 控制台色板（琥珀 `--amber` / 牛血 `--oxblood` / 等宽时间码 `--font-mono`）、来源色（`--src-self/web/tauri`，复用 lane 调色板）、诊断信号类型色（`--sig-*`）、Element Plus 全量深色变量覆写（`:root:root` 提权） |
| [src/views/MainView.vue](src/views/MainView.vue) | 会话观测台：源监控机架（本机通道承袭 REC 脉冲 DNA、web 通道按 server 监听状态点亮 + 显示接入点与 web 会话数、tauri 待接入）+ 会话浏览器（来源过滤 chip/搜索 ID·名称/富行/回放，来源取 session.json 的 `source` 字段）；P6 加元信息编辑弹窗（name/note/tags）、导出（Blob 下载 `.rrweb-session.json`）、导入（file input）、会话行显示名称 + 导入标记 |
| [src/views/PlayerView.vue](src/views/PlayerView.vue) | 诊断工作台：spotlight 回放网格 + 诊断信号流（统一流+过滤，随播放头高亮/点击 seek）+ transport（色带 + error 红标 + focus 标记 + playhead + 倍速 + 跟随焦点），诊断栏可折叠；P6 加「信号/标注」tab（时间轴骨白圆点标记 + 打点输入 + 标注列表点击 seek） |
| [src/composables/useAnnotations.ts](src/composables/useAnnotations.ts) | P6 会话级标注：load `list_annotations`、add/update/remove 后立即 `save_annotations` 整体覆写（标注低频，无 debounce）；标注 `{ id, t, label?, text, author, createdAt }` 与 segment 事件流分离 |
| [src/views/SettingsView.vue](src/views/SettingsView.vue) | 采集/接收/保留配置项：接收（HTTP server enabled/端口/token）已接线持久化（`get/set_ingest_config`，端口重启生效）；采集开关固定全开，按需过滤待接线 |

### Rust 命令清单

分两类：
- **console 自有命令**（[src-tauri/src/lib.rs](src-tauri/src/lib.rs) `generate_handler![]`，无需 capabilities 授权）：`greet`、`open_window`、`list_sessions`、`read_session`、`delete_session`、`list_annotations`、`save_annotations`、`update_session_meta`、`export_session`、`import_session`、`get_ingest_config`、`set_ingest_config`。
- **插件命令**（`tauri-plugin-observer` `Builder::invoke_handler`，注册为 `plugin:observer|*`，需 `observer:default` 权限）：`start_session`、`stop_session`、`is_recording_active`、`begin_segment`、`append_events`、`bind_session`（Remote：绑定 server sessionId 并广播）、`session_id`（Remote：子窗口取 sessionId）、`notify_segment_start`（窗口复用 emit segment:start）。

`listen` 由 `core:default` 允许。`TauriSink` 包装 `plugin:observer|begin_segment`/`append_events`/`is_recording_active`（及 startSession/endSession 对应 `plugin:observer|start_session`/`stop_session`）。`get/set_ingest_config` 读写 `IngestState`（`enabled`/`port`/`token`，持久化到 `appDataDir/ingest-config.json`）。

## 当前现状

### 已实现

- 多窗口录制：主窗口 + 任意子窗口，会话期间新开的窗口自动接入。
- 子窗口关闭=隐藏、再开重录（分段），主窗口关闭=退出。
- 事件流式落盘（每秒 flush 一次）。
- 会话列表、删除。
- 多窗口回放：播放/暂停/进度拖动/倍速，按 shown/hidden 区间切换可见 segment。
- 稳定槽位布局：每个 label 一个固定网格槽位，show/hide 不 reflow；隐藏窗口显示占位。
- spotlight 主窗口：focus 时间线自动跟踪 + 手动点击选主 + 自动跟随开关；主槽占大格、侧槽堆叠，CSS 切换不 reparent。
- 时间轴色带：主进度条上方叠加每窗口活跃区间色带 + focus 切换标记 + playhead，支持点击 seek。
- tile 等比缩放（方案 E）：`.replayer-wrapper` 固定为录制视口尺寸（Meta 事件的 width/height），`transform: scale()` fit-contain 到 tile-root 并居中；spotlight 切主 / 窗口缩放 / 段显隐由 `ResizeObserver` 自动重算。
- 漂移阈值纠偏：各 Replayer 与主时钟偏差超 120ms 时 re-seek 对齐。
- focus 数据补强：会话开始补记初始 focus、过滤 player-* focus 事件。
- 诊断信号采集（P2）：error/console/network hook 在段录制期间 emit `type:6` 交错事件，与 DOM 同流落盘；console args 序列化（截断循环引用、Node/Error 转结构），network 默认不记 body/headers。
- 诊断信号流回放（P1+P2）：PlayerView 统一信号流（console/network/error 混排 + 过滤），随播放头高亮、点击 seek；时间轴叠加 error 红标；诊断栏可折叠。
- 会话观测台布局（P1）：MainView 源监控机架（本机/web/tauri 通道）+ 会话浏览器（来源过滤/搜索）；SettingsView 采集/接收/保留配置项。
- Sink 传输抽象（P3）：`useRecorder` 走 `SegmentRecorder`（SDK）+ 注入 `TauriSink`，self-obs 行为不变；采集核心下沉到 observer-sdk 包。
- Web SDK + 本地接收 server（P4）：console 启动 tiny_http server 监听 `127.0.0.1:1421`（端口可配），`/ingest/*` 复用 storage 落盘到同一 `recordings/` 结构（外部会话 `source:"web"`，带 `appId`/`env`/...）；token 鉴权 + CORS；`HttpSink` 对接真实后端，批量/重试/sendBeacon 兜底。npm 包 `@prism-obs/observer-sdk` 的 `init({appId, endpoint, token, ...})` 供外部 web 应用嵌入；样例见 `examples/web-demo`。`cargo test` 覆盖 ingest 完整会话落盘格式。
- Tauri Plugin（P5）：录制协调逻辑（Session/segment/窗口生命周期拦截）抽成独立 crate [`plugins/tauri-plugin-observer`](../../plugins/tauri-plugin-observer)，双模式 `Mode::{Local,Remote}`。Local（console self-obs）Rust 落盘、命令注册为 `plugin:observer|*`（`TauriSink` 调用，`observer:default` 权限）；Remote（外部 Tauri 应用）Rust 只协调 + 事件驱动、前端 `HttpSink` 上报，跨窗口 sessionId 由主窗口 `HttpSink.startSession` 取得后经 `bind_session` 广播共享。Tauri 2 plugin Builder 无 `on_window_event`，用 `on_window_ready` 给每窗口挂 `Window::on_window_event`。JS 驱动 [`packages/observer-tauri`](../../packages/observer-tauri) `initTauri()`（监听 `recording-session`/`segment`/`observer-lifecycle` 驱动 `SegmentRecorder`，hidden/focus 经 `HttpSink.appendLifecycle` 上报）；样例 [`examples/tauri-demo`](../../examples/tauri-demo)。console 装 Local 模式回归正常，MainView tauri 通道点亮。
- 导出/标注/分享（P6）：标注存 session 级 `annotations.jsonl`（`{ id, t, label?, text, author, createdAt }`）与 segment 事件流分离，回放时与 signals 共享相对会话起点时间轴。console 新增 `list_annotations`/`save_annotations`/`update_session_meta`/`export_session`/`import_session` 命令（核心逻辑抽纯函数 `build_export_bundle`/`write_import_bundle`/`merge_session_meta`，可测）；[`useAnnotations`](../../src/composables/useAnnotations.ts) 持有完整列表、增删改后立即整体覆写。PlayerView 诊断栏加「信号/标注」tab + 时间轴骨白圆点标记；MainView 会话行加编辑/导出 + 顶部导入 + 元信息弹窗。导出为单文件 JSON bundle（`format: prism-session`）零新依赖，导入分配新 id 重建目录、标记 `importedAt`。

### 编译验证

- `cargo check` / `cargo test`（src-tauri 内）通过（7 passed：ingest 落盘 + annotations 读写 + export/import roundtrip + 元信息合并）；插件 crate 作为 path 依赖一并编译。
- `cargo check`（`examples/tauri-demo/src-tauri` 内）通过（Remote 模式插件集成）。
- `pnpm exec vue-tsc --noEmit` 通过。
- `pnpm build`（含 vite 构建）通过。
- `pnpm --filter @prism-obs/observer-sdk build` 产 `dist/index.js`；`examples/web-demo` `tsc --noEmit` 通过。
- `pnpm --filter @prism/observer-tauri typecheck`、`pnpm --filter tauri-demo typecheck` 通过。

### 运行时实测

已跑 `pnpm tauri dev` 实测确认：

- ✅ spotlight 主槽/侧槽布局（`grid-row:1/-1` + 显式行数）、主窗口自动跟踪与手动切换；横向溢出已消除。
- ✅ tile 等比缩放（方案 E）生效，spotlight 主区放大 / 窗口缩放自适应。
- ✅ 会话观测台布局（P1）：源监控机架、会话浏览器（来源过滤/搜索）、诊断信号流 + 时间轴 error 红标（用户实测确认）。
- ✅ 诊断信号采集（P2）：error/console/network hook、`type:6` 交错回放、信号流随播放头高亮与点击 seek（用户实测确认）。
- ✅ Sink 抽象（P3）：`useRecorder` 走 Sink 接口、`TauriSink` 包装 invoke，self-obs 录制/回放/信号无回归（用户实测确认）。

仍待实测确认：

- 漂移纠偏的实际收敛效果、`play(offset)` 在长事件流上 re-seek 的延迟。
- 录制中关闭子窗口的拦截时序、`emit_to` 定向事件是否被目标窗口稳定接收。
- HttpSink 已对接 P4 本地 server（端到端 E2E 仍待跑：web demo 上报 -> console 列表 -> 回放）；IndexedDBSink 独立回放读取路径仍待补。
- P5 E2E 仍待跑：`examples/tauri-demo` 装插件 Remote 模式 -> console 列表出现 `source:"tauri"` 会话 -> 多窗口对齐回放正确；Remote 模式窗口关闭=隐藏（非主窗口）的拦截时序、`bind_session` 跨窗口 sessionId 广播、hidden/focus 经 HttpSink 上报是否齐全。
- P6 E2E 仍待跑：录一段 -> 打标注 -> 导出 -> 导入 -> 回放 + 诊断信号流 + 标注均还原（文件层面 export/import roundtrip 已有单测覆盖）。

### 已知 MVP 限制

- 回放各 segment 仍各自跑独立 RAF，靠 120ms 阈值纠偏拉回（有界，非零漂移）；彻底零漂移需主时钟步进（方案 A，未做）。
- 回放布局为稳定槽位 + spotlight，但**未还原原始窗口位置/尺寸**（方案 C，未做）。
- 录制中关闭主窗口=直接退出，session.json 无 endedAt（不影响回放，但列表时长显示会以「现在」估算）。
- 诊断信号 body/headers 默认关（PII）；SettingsView 采集开关固定全开、按需过滤待接线（接收开关已接线）。
- HttpSink 已对接 P4 本地 server；IndexedDBSink 仍为预留骨架（读取路径未补）。外部 web 会话无 `endedAt` 时列表时长以「现在」估算。

## 未实现功能（TODO）

按优先级粗略排序（P1-P6 均已落地；以下为增强项）：

1. **运行时测试与修 bug**：基本流程（开始录制 -> 开关子窗口 -> 停止 -> 回放）+ spotlight + 等比缩放 + P1/P2/P3 已实测通过；P4/P5/P6 的 E2E 仍待跑（见「运行时实测」）。
2. **位置精确回放（方案 C）**：录制时记窗口 x/y/w/h（`outer_position`/`outer_size`），回放按包围盒缩放 fit 视口、按原位置摆放，还原多窗口空间关系。
3. **无漂移同步（方案 A）**：主 RAF 循环每帧 `replayer.pause(offset)` 步进驱动各 segment，取代独立 `play`，彻底零漂移。当前已落地阈值纠偏（方案 B，120ms re-seek），A 为可选增强。
4. **进度条精度与性能**：当前 50ms tick 更新 `currentTime`，长录制下可考虑节流或换 `requestAnimationFrame`。
5. **标注增强**：inline 编辑文本、多作者区分（当前 author 固定 "local"）、标注关联具体窗口的视觉区分。

未来可选：音频录制、屏幕流（超出 rrweb DOM 录制范畴，需额外方案）。

## 扩展指引

- **新增录制配置**（采样、遮罩等）：改 `useRecorder.ts` 的 `record({ emit, ...options })` 调用。诊断信号 hook 在 `installSignalHooks`（error/console/network），新增信号类型在此扩展并对应 `usePlayer` 的 `Signal` 类型；network body/headers 默认关（PII），SettingsView 的开关待接线。
- **新增传输后端**：实现 `Sink` 接口（见 observer-sdk 的 `sinks.ts`），注入 `useRecorder(sink)` 或外部采集器。`HttpSink` 已对接 P4 server（endpoint/token 即 console 设置页的接入点/token；可加退避/容量上限增强）；`IndexedDBSink` 的独立回放读取路径待补。
- **新增会话级元数据**：`update_session_meta`（合并写入 session.json，空串/null 删除字段）已落地；`list_sessions`/`read_session` 自动读取全部字段。标注走独立的 `annotations.jsonl`（`list_annotations`/`save_annotations`）。
- **新增窗口生命周期事件**：在插件 [lifecycle.rs](../../plugins/tauri-plugin-observer/src/lifecycle.rs) 的 `handle_window_event` 增 match 分支；Local 模式 `append_lifecycle` 落 windows.jsonl，Remote 模式 emit 事件交前端 `HttpSink.appendLifecycle` 上报。回放侧 `usePlayer` 解析新 type。
- **新增 Rust 命令**：console 自有命令在 [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs) 加 `#[tauri::command]` 并注册到 `generate_handler![]`（无需 capabilities 授权）；插件命令在 [commands.rs](../../plugins/tauri-plugin-observer/src/commands.rs) 加、注册到插件 `Builder::invoke_handler`、并在 [build.rs](../../plugins/tauri-plugin-observer/build.rs) 的 `COMMANDS` 追加（生成 `allow-<cmd>` 权限）、按需加入 `permissions/default.toml`，调用方 capabilities 需 `observer:default`（或对应 allow 权限）。
- **新增带新 label 模式的路由**：同步改 [src-tauri/capabilities/default.json](src-tauri/capabilities/default.json) 的 `windows` glob，否则窗口缺权限。
- **新增窗口轨道色**：在 `usePlayer.ts` 的 `LANE_COLORS` 数组追加色值，时间轴色带与对应磁贴头圆点同时生效；勿在视图里写临时色值。新增**来源色**走 `theme.css` 的 `--src-*`（勿复用 lane 色做来源语义）。

## 约定

- UI 文案中文（zh-CN），与现有一致。
- 前端自动导入已开：`ref`/`reactive`/`ElMessage` 等无需手动 import（`.vue` 文件中）；`.ts` composable 内可显式 import vue API 以求清晰。
- 设计系统见 [src/styles/theme.css](src/styles/theme.css)：warm-dark 控制台主题，色板/字号/间距以 CSS 变量定义（`--ink`/`--slate`/`--amber`/`--oxblood`/`--font-mono` 等），Element Plus 经 `:root:root` 变量覆写为深色。视图用平铺布局，勿再用 `el-card shadow="hover"` 包裹；新色值走既有 token，勿临时写死。
- 类型声明 `src/auto-imports.d.ts`、`src/components.d.ts` 为构建产物，勿手改。
- Rust crates 走 rsproxy 镜像，勿删 `.cargo/config.toml`。
