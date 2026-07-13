# P4：Web SDK

> 阶段路径第 4 阶段。目标：支撑「PC web 端观测」--任意 web 应用嵌入 npm 包，采集上报到 console。

## 目标

支撑「PC web 端观测」：任意 web 应用嵌入 npm 包，采集上报到 console。

## 范围

1. console 起本地 HTTP server（`/ingest/*`），存储层复用 self-obs 的落盘函数。
2. 采集逻辑打包成 npm 包（`@your/observer-sdk`），用 `HttpSink`。

## console 侧

- [lib.rs](../../src-tauri/src/lib.rs) 新增 HTTP server 模块（或独立文件），监听 `127.0.0.1:<port>`，handler 调用 [P3](./P3-sink抽象.md) 抽出的存储函数。
- 仅本地 token 鉴权，避免同机误投。
- MainView 源监控机架的 web 通道点亮（接入指示）。

## SDK 侧

- 入口 `init({ appId, endpoint, env, release, signals })`，启动 rrweb record + [P2](./P2-诊断信号采集.md) 信号 hooks + `HttpSink`。
- 批量 flush（1s / N 条）、失败退避重试、unload `sendBeacon`。
- 会话 = 一次页面访问；SPA 路由连续，整页刷新新段。

## 不做

- 不做多 tab 协调（web 单 tab 是先天限制，多窗口对齐是 [P5](./P5-Tauri-Plugin.md) 的事）。
- 不做 sourcemap（本地优先 dev 场景，源码未压缩）。

## 验收

- 一个独立 web 应用嵌入 SDK，操作后 console 能列出该会话、诊断信号流正确呈现 error/console/network。
- `pnpm build` 通过；SDK 包能被外部 Vite 项目 import。
