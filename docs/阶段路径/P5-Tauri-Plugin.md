# P5：Tauri Plugin

> 阶段路径第 5 阶段。目标：把现 `lib.rs` 的多窗口对齐录制逻辑抽成 `tauri-plugin-observer`，别的 Tauri 2 应用安装即得，HTTP 上报到 console。**这是把项目独有牌（多窗口对齐）产品化的阶段。**

## 目标

支撑「Tauri 客户端观测」：把现 `lib.rs` 的多窗口对齐录制逻辑抽成 `tauri-plugin-observer`，别的 Tauri 2 应用安装即得，HTTP 上报到 console。

这是把项目独有牌（多窗口对齐）产品化的阶段--Web SDK（[P4](./P4-Web-SDK.md)）只能单 tab，多窗口对齐是桌面端独有差异点。

## 范围

1. 从 [lib.rs](../../src-tauri/src/lib.rs) 抽出录制相关逻辑（Session 状态、segment 管理、`on_window_event` 生命周期拦截）为独立 crate `tauri-plugin-observer`。
2. 插件 JS 侧复用 [P2](./P2-诊断信号采集.md) 信号采集 + `HttpSink`（与 Web SDK 共享采集代码）。
3. Rust 侧管窗口 show/hide/focus + segmentId 分配，上报生命周期与事件到 console。

> 落地：插件 crate [`plugins/tauri-plugin-observer`](../../plugins/tauri-plugin-observer)，双模式 `Mode::{Local,Remote}`--Local（console 自录，Rust 落盘到 `appDataDir/recordings/`）/ Remote（外部应用，Rust 只协调 + 事件驱动，前端 `HttpSink` 上报）。录制命令（`start/stop_session`/`begin_segment`/`append_events`/`is_recording_active`/`bind_session`/`session_id`/`notify_segment_start`）注册为 `plugin:observer|*`，权限 `observer:default`。Tauri 2 plugin Builder 无 `on_window_event`，改用 `on_window_ready` 给每窗口挂 `Window::on_window_event`。Remote 跨窗口 sessionId 共享：主窗口 `HttpSink.startSession` 取得后 `bind_session` 广播，子窗口 `useSessionId` 注入。JS 驱动 [`packages/observer-tauri`](../../packages/observer-tauri)（`@prism-obs/observer-tauri`）`initTauri()`；样例 [`examples/tauri-demo`](../../examples/tauri-demo)。console 装 Local 模式，前端 `TauriSink` 调 `plugin:observer|*`，self-obs 回归；MainView tauri 通道点亮。

## 关键点

- 多窗口对齐是 Web SDK 无法复制的核心能力，是平台差异点。
- 插件化后，console 自身的 self-obs 可改为「安装自己的插件」的形态，统一路径（可选，向后兼容 `TauriSink`）。
- 跨进程上报靠墙上时钟；同会话单源无漂移。
- console 源监控机架的 tauri 通道点亮。

## 不做

- 不做跨会话/跨源时钟校正（同会话单源已够）。
- 不做云聚合（本地优先）。

## 验收

- 一个独立 Tauri 应用安装插件、开多窗口录制，console 能列出会话、多窗口对齐回放正确（spotlight + 色带 + 漂移纠偏）。
- console 自录回归正常。

> 编译验证：插件 crate / console / demo 三者 `cargo check` 通过；console `cargo test`（ingest 落盘）通过；前端 `pnpm build`（vue-tsc）通过；`observer-tauri` / demo 前端 `tsc --noEmit` 通过。运行时 E2E（demo 上报 -> console 列表 -> 多窗口对齐回放）仍待实测。
