# P5：Tauri Plugin

> 阶段路径第 5 阶段。目标：把现 `lib.rs` 的多窗口对齐录制逻辑抽成 `tauri-plugin-observer`，别的 Tauri 2 应用安装即得，HTTP 上报到 console。**这是把项目独有牌（多窗口对齐）产品化的阶段。**

## 目标

支撑「Tauri 客户端观测」：把现 `lib.rs` 的多窗口对齐录制逻辑抽成 `tauri-plugin-observer`，别的 Tauri 2 应用安装即得，HTTP 上报到 console。

这是把项目独有牌（多窗口对齐）产品化的阶段--Web SDK（[P4](./P4-Web-SDK.md)）只能单 tab，多窗口对齐是桌面端独有差异点。

## 范围

1. 从 [lib.rs](../../src-tauri/src/lib.rs) 抽出录制相关逻辑（Session 状态、segment 管理、`on_window_event` 生命周期拦截）为独立 crate `tauri-plugin-observer`。
2. 插件 JS 侧复用 [P2](./P2-诊断信号采集.md) 信号采集 + `HttpSink`（与 Web SDK 共享采集代码）。
3. Rust 侧管窗口 show/hide/focus + segmentId 分配，上报生命周期与事件到 console。

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
