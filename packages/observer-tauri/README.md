# @prism-obs/observer-tauri

本地优先前端观测平台 · Tauri Plugin 采集端（Remote 模式）。配合 [`@prism-obs/observer-sdk`](https://www.npmjs.com/package/@prism-obs/observer-sdk) 与 Rust 端 `tauri-plugin-observer`，在**外部 Tauri 2 应用**中录制多窗口 DOM + 诊断信号，经 `HttpSink` 上报到 console（本地 / 自托管 server）。

## 安装

```bash
pnpm add @prism-obs/observer-tauri @prism-obs/observer-sdk
```

外部 Tauri 应用还需在 `Cargo.toml` 引入 Rust 端插件 `tauri-plugin-observer`（见仓库 [`plugins/tauri-plugin-observer`](https://gitee.com/guoo139/rrweb-demo/tree/main/plugins/tauri-plugin-observer)）。

## 用法

```ts
import { initTauri } from "@prism-obs/observer-tauri";

// 主窗口（autoStart: true）建会话并广播；子窗口不传 autoStart，等待广播自启
const ctrl = await initTauri({
  appId: "my-tauri-app",
  endpoint: "http://127.0.0.1:1421", // console 本地 server
  token: "<可选本地 token>",
  autoStart: true, // 仅主窗口传 true
});

await ctrl.stop();
```

## 架构

- `initTauri()` 监听插件 `recording-session` / `segment` / `observer-lifecycle` 事件驱动 `SegmentRecorder` 开/停段；
- sessionId 由主窗口从 console server 取得后经插件 `bind_session` 广播，子窗口共享；
- 窗口生命周期（hidden/focus）由 Rust 检测后 emit 事件，前端转发为 `HttpSink` lifecycle 上报。

详见仓库 `docs/阶段路径/P5-Tauri-Plugin.md`。
