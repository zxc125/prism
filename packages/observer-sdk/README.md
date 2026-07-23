# @rrweb-demo/observer-sdk

本地优先前端观测平台 · Web 采集 SDK。嵌入任意 web 应用，录制 DOM + 诊断信号，上报到 console（Tauri 应用）的本地 HTTP server。

## 用法

```ts
import { init } from "@rrweb-demo/observer-sdk";

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

## 架构

采集逻辑（rrweb `record` + 信号 hook + 缓冲 flush）经 `Sink` 接口与落盘/上报解耦：

- `HttpSink` — 本 SDK 默认，POST 到 console `/ingest/*`，批量 + 重试 + unload beacon。
- `SegmentRecorder` — 单段录制器，self-obs（`useRecorder`）与外部 SDK 共用，差别仅在 Sink 注入与驱动方式。

详见仓库 `docs/架构/被观测侧（采集）.md`。
