# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 命令

包管理器为 **pnpm**（由 [src-tauri/tauri.conf.json](src-tauri/tauri.conf.json) 中的 `beforeDevCommand`/`beforeBuildCommand` 使用）。

| 任务 | 命令 |
| --- | --- |
| 仅启动前端 dev 服务器（端口 1420） | `pnpm dev` |
| 类型检查 + 构建前端到 `dist/` | `pnpm build` |
| 以开发模式运行完整 Tauri 桌面应用 | `pnpm tauri dev`（自动执行 `pnpm dev`） |
| 构建可安装的桌面安装包 | `pnpm tauri build`（自动执行 `pnpm build`） |
| Rust 检查/测试（在 `src-tauri/` 内执行） | `cargo check` / `cargo test` |

`pnpm build` 会先执行 `vue-tsc --noEmit` - 类型错误会导致构建失败。项目未配置 JS/TS 测试框架；仅有的测试是 Rust 的 `cargo test`（目前没有任何测试）。

## 平台演进方向

本项目正从「自录 + 回放工具」演进为**本地优先的前端观测平台**（支持 web / tauri 外部观测，闭环到「回放 + 诊断 + 导出/标注/分享」）。架构与阶段规划见 [docs/架构/](docs/架构/) 与 [docs/阶段路径/](docs/阶段路径/)。

四条锁定决策：
1. **本地优先** - Tauri App 当分析台（console），零云依赖，不建云后端 RUM。
2. **观测外部** - Web SDK（npm，单 tab）+ `tauri-plugin-observer`（多窗口对齐），统一走 HTTP sink -> console 本地 server。
3. **闭环到回放+诊断** - 不做告警/生产 RUM；诊断 = 回放带 error/console/network 上下文 + 导出/标注/分享。
4. **交错事件模型** - error/console/network 以 rrweb plugin 事件（`type:6`）交错进同一条事件流，与 DOM 共享时间轴。

阶段路径（[docs/阶段路径/](docs/阶段路径/)）：P1 分析端页面改造 -> P2 诊断信号采集 -> P3 sink 抽象 -> P4 Web SDK -> P5 Tauri Plugin -> P6 导出/标注/分享。

**进度**：P1 ✅（源监控机架 + 会话浏览器 + 诊断信号流）· P2 ✅（error/console/network hook，`type:6` 交错进事件流）· P3 ✅（[sink.ts](src/composables/sink.ts) 抽 `Sink` 接口 + `TauriSink`，useRecorder 不再直接 invoke）· P4 ✅（Web SDK：console 起 `127.0.0.1` 本地 HTTP server `/ingest/*`（[ingest.rs](src-tauri/src/ingest.rs)）+ 打包 [`packages/observer-sdk`](packages/observer-sdk)（`@rrweb-demo/observer-sdk`）走 `HttpSink`，self-obs 与 SDK 共用 `SegmentRecorder`；样例 [`examples/web-demo`](examples/web-demo)）· P5 ✅（抽 [`plugins/tauri-plugin-observer`](plugins/tauri-plugin-observer) 独立 crate：Local（console self-obs，Rust 落盘）+ Remote（外部应用，前端 `HttpSink` 上报）双模式；录制命令搬进插件，console 装 `plugin:observer|*`；[`packages/observer-tauri`](packages/observer-tauri) 提供 `initTauri()` 驱动；样例 [`examples/tauri-demo`](examples/tauri-demo)）· P6 ✅（标注存 session 级 `annotations.jsonl`（`{ id, t, label, text, author, createdAt }`）与 segment 事件流分离；console 新增 `list_annotations`/`save_annotations`/`update_session_meta`/`export_session`/`import_session` 命令（新建 [annotations.rs](src-tauri/src/annotations.rs)）；[`useAnnotations`](src/composables/useAnnotations.ts) 持有完整列表、增删改后立即整体覆写；PlayerView 诊断栏加「信号/标注」tab + 时间轴骨白圆点标记，MainView 会话行加编辑/导出 + 顶部导入；导出为单文件 JSON bundle（`format: rrweb-demo-session`）零新依赖，导入分配新 id 重建目录）。

## 架构

Tauri 2 桌面应用：Vue 3 + Vite 6 前端位于 [src/](src/)，Rust 后端位于 [src-tauri/](src-tauri/)。基于 rrweb 2 实现**多窗口录制与回放**。

### 录制 / 回放系统（横跨前端 + 后端）

rrweb 在前端 webview 里跑，每个窗口各一个 `record()` 实例；事件按 segment 流式落盘到 `appDataDir/recordings/<sessionId>/`：

```
recordings/<sessionId>/
  session.json          # { id, startedAt, endedAt?, source?, name?, note?, tags?, importedAt? }
  windows.jsonl         # 窗口生命周期: shown/hidden/focus，带 segmentId
  segments/<label>#<n>.jsonl   # 每段 rrweb 事件流（一次 show ~ hide = 一段）
  annotations.jsonl     # 用户标注（P6，session 级，与事件流分离）
```

关键机制（需结合多文件理解）：

- **会话与段**：`start_session` 置 active 并广播 `recording-session` 事件；各窗口的 `useRecorder`（[src/composables/useRecorder.ts](src/composables/useRecorder.ts)，由 [App.vue](src/App.vue) 挂载）收到后 `invoke("plugin:observer|begin_segment")` 分配 segmentId `<label>#<n>` 并启动 rrweb。`player-*` 窗口跳过录制（避免回放被录进会话）。
- **录制协调已抽成插件**：`start_session`/`stop_session`/`is_recording_active`/`begin_segment`/`append_events` 等录制命令与 `on_window_event` 生命周期拦截已搬进独立 crate [`plugins/tauri-plugin-observer`](plugins/tauri-plugin-observer)（`tauri-plugin-observer`）。两种模式：**Local**（console self-obs，Rust 直接落盘到 `appDataDir/recordings/`）与 **Remote**（外部 Tauri 应用，Rust 只管窗口协调 + 事件驱动，前端 `HttpSink` 上报到 console）。console 装插件 Local 模式（`skip_focus_prefix: "player-"`），前端 `TauriSink` 调 `plugin:observer|*` 命令；`list_sessions`/`read_session`/`delete_session`/`open_window`/ingest 仍留 [src-tauri/src/lib.rs](src-tauri/src/lib.rs)。插件命令需在 capabilities 授权 `observer:default`（[default.json](src-tauri/capabilities/default.json)）；console 自定义 command（`open_window` 等）仍无需授权。
- **子窗口关闭=隐藏**：录制期间，子窗口的 `CloseRequested` 被拦截为 `hide()` + 记 `hidden` + `emit_to` segment:stop；再次 `open_window` 复用已隐藏窗口时 `show()` + 插件 `emit_segment_start_if_active` 开新段。主窗口关闭不拦截 = 退出进程。见插件 [lifecycle.rs](plugins/tauri-plugin-observer/src/lifecycle.rs)（Tauri 2 plugin Builder 无 `on_window_event`，用 `on_window_ready` 给每窗口挂 `Window::on_window_event`）。
- **跨窗口对齐**：rrweb 事件带绝对 `timestamp`，所有窗口共享墙上时钟，回放时按 `shown.t ~ hidden.t` 区间在主时间轴上同步驱动各 segment 的 `Replayer`。见 [src/composables/usePlayer.ts](src/composables/usePlayer.ts)。
- **回放**：[src/views/PlayerView.vue](src/views/PlayerView.vue) 用底层 `Replayer`（rrweb-player 是 Vue 2，不能用）+ 自建 Element Plus 控制条，平铺显示当前活跃窗口。
- 新增的 Tauri command：console 自有命令（`greet`/`open_window`/`list_sessions`/`read_session`/`delete_session`/`list_annotations`/`save_annotations`/`update_session_meta`/`export_session`/`import_session`/`get_ingest_config`/`set_ingest_config`）注册在 [src-tauri/src/lib.rs](src-tauri/src/lib.rs) `generate_handler![]`，无需 capabilities 授权；插件命令（`plugin:observer|*`）注册在插件 `Builder::invoke_handler`，需 `observer:default` 权限。

### 多窗口系统（横跨前端 + 后端）

这是核心机制，需要结合多个文件一起理解：

1. **前端** 调用 `invoke("open_window", { route })` - 见 [src/views/MainView.vue](src/views/MainView.vue)。
2. **后端** [src-tauri/src/lib.rs](src-tauri/src/lib.rs) 的 `window_label()` 由路由推导窗口 label：`/settings` -> `settings`，`/player/abc` -> `player-abc`，`/` -> `main`。**相同 label = 单实例（聚焦已有窗口）；不同 label = 多实例。** 这就是设置窗口为单实例、播放器窗口支持多开的原因。
3. 新窗口加载 `index.html#{route}` - **hash 模式路由**（[src/router/index.ts](src/router/index.ts)）据此解析并渲染对应视图。此处必须用 hash 模式：Tauri 通过自定义协议 / 本地文件提供服务，`createWebHistory` 在刷新或深链时会 404。
4. **Capabilities** [src-tauri/capabilities/default.json](src-tauri/capabilities/default.json) 必须列出每个窗口 label 或 glob（`"player-*"`），否则窗口缺少权限。新增带新 label 模式的路由时需要同步修改此文件。

### 前端约定

- **已开启自动导入**（[vite.config.ts](vite.config.ts)）：`unplugin-auto-import` 全局提供 Vue API（`ref`、`reactive`、`computed`、`useRoute` 等）与 Element Plus API（`ElMessage`、`ElMessageBox`），**无需手动 import**。`unplugin-vue-components` 自动注册 Element Plus 组件（`el-button`、`el-card`…）。不要为这些添加手动导入 - 缺少 import 是有意为之，不是 bug。
- 类型声明生成到 [src/auto-imports.d.ts](src/auto-imports.d.ts) 与 [src/components.d.ts](src/components.d.ts)。这两个是构建产物 - 不要手动编辑；运行 `pnpm dev`/`pnpm build` 即可重新生成。
- UI 文案为中文（zh-CN）。新增用户可见文本时请保持一致的语言。

### 视觉设计约定

任何涉及视觉/样式的变更（新增视图、改配色、改排版、重做交互形态）**必须先调用 `/frontend-design` skill**，按其流程（brainstorm -> critique -> build -> critique again）做出有意图的设计选择，不要直接套用 Element Plus 默认样式或通用暗色模板。

现有设计语言已落地，变更需与之保持一致：

- **设计 token** 集中在 [src/styles/theme.css](src/styles/theme.css)：暖色偏暗底（`--ink`/`--slate`）、骨白文字、**示波器琥珀** `--amber` 主色、**牛血红** `--oxblood` 用于 REC/危险态；等宽字体是身份嗓音（时间码、轨道标签、ID）。不要退回中性纯黑或 acid-green/vermilion 等通用暗色模板配色。
- **Element Plus 深色化** 通过在 `:root:root` 覆盖 `--el-*` 变量实现（双 specificity 压过 EP 内置 `:root`）；新组件沿用此方式，避免 `el-card shadow="hover"` 之类默认包装，视图为平铺布局。
- **轨道色** 集中在 [src/composables/usePlayer.ts](src/composables/usePlayer.ts) 的 `LANE_COLORS`，新增窗口轨道色在此扩展。**来源色**（P1）复用 lane 调色板：本机=`--amber`、web=`--lane-7`、tauri=`--lane-5`。
- **签名元素**：MainView = 源监控机架（多通道控制室输入 bay，本机通道承袭 REC 脉冲 DNA）；PlayerView = 多轨时间轴 + 真实播放头 + 诊断信号流；SettingsView 保持安静。新视图应贡献自己的一个签名元素，而非复刻通用模板。（P1 重做中：旧 MainView 的硬件式 REC 控件已演化为源机架的本机通道；设计详见 [docs/阶段路径/P1-分析端页面改造.md](docs/阶段路径/P1-分析端页面改造.md)。）

### 后端约定

- 应用入口为 [src-tauri/src/lib.rs](src-tauri/src/lib.rs) 的 `run()`；[src-tauri/src/main.rs](src-tauri/src/main.rs) 仅在 release 模式下屏蔽 Windows 控制台窗口并委托调用。新增 Tauri command 必须在 `lib.rs` 的 `generate_handler![]` 中注册。
- 每个路由的窗口标题/尺寸在 `open_window` 中按路由首段匹配决定 - 新增需要自定义尺寸的路由时，扩展该 match 分支。

### 环境说明

- [src-tauri/.cargo/config.toml](src-tauri/.cargo/config.toml) 将 crates.io 重定向到 **rsproxy.cn** 镜像（国内）。若 Rust 依赖拉取失败，问题/解法均在此镜像配置 - 无充分理由不要移除。
- Vite 固定使用端口 1420 且 `strictPort: true`（Tauri 依赖此端口）；`src-tauri/**` 已被排除出 Vite 文件监听。
