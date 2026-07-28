# @prism/observer-sdk

本地优先前端观测平台 · Web 采集 SDK。嵌入任意 web 应用，录制 DOM + 诊断信号，上报到 console（Tauri 应用）的本地 HTTP server。

## 用法

```ts
import { init } from "@prism/observer-sdk";

const ctrl = await init({
  appId: "my-app",
  endpoint: "http://127.0.0.1:1421", // console 本地 server（端口/ token 见设置页）
  token: "<可选本地 token>",
  env: "dev",
  release: "1.0.0",
});

// 显式停止（可选）；页面卸载会自动 sendBeacon 兜底
await ctrl.stop();
```

会话 = 一次页面访问；SPA 路由连续，整页刷新开新段。`signals` 可按需只开 `error`/`console`/`network`，默认全开。

## 注意：被观测页需显式设置背景色

rrweb 只录 DOM 样式，**不录浏览器画布默认色**（`color-scheme` / 浏览器默认白底）。若页面仅靠 `color-scheme: light dark` 取默认背景与文字色，回放时 iframe 画布透明会透出播放器底色，深色系统下浅色文字可能落在白底上看不见。请给 `html, body` 显式写 `background` 与 `color`，录制即可忠实还原（参见 `examples/web-demo`）。

## 架构

采集逻辑（rrweb `record` + 信号 hook + 缓冲 flush）经 `Sink` 接口与落盘/上报解耦：

- `HttpSink` — 本 SDK 默认，POST 到 console `/ingest/*`，批量 + 重试 + unload beacon。
- `SegmentRecorder` — 单段录制器，self-obs（`useRecorder`）与外部 SDK 共用，差别仅在 Sink 注入与驱动方式。

详见仓库 `docs/架构/被观测侧（采集）.md`。
