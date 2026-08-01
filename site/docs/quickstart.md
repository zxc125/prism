# 快速开始

3 分钟跑通：装 SDK → 起 console → 嵌入被观测页 → 在 console 看回放。

## 1. 起 console（接收端）

console 是数据落盘 + 回放分析的地方。两种形态任选其一：

- **桌面 App**：从 [GitHub](https://github.com/zxc125/prism) 下载构建，或本地 `pnpm tauri dev` 启动。启动后打开 **设置页**，记下本地 server 地址与端口（默认 `http://127.0.0.1:1421`）及可选 token。
- **单二进制 server**：`observer-server --bind 127.0.0.1:8080 --data-dir ./recordings`（详见 [私有化部署](./deploy)）。

## 2. 在被观测 Web 应用装 SDK

```sh
pnpm add @prism-obs/observer-sdk
```

在应用入口调用一次 `init()`：

```ts
import { init } from "@prism-obs/observer-sdk";

const ctrl = await init({
  appId: "my-app",
  endpoint: "http://127.0.0.1:1421", // console 设置页可查
  token: "<可选本地 token>",           // console 开启鉴权时必传
  env: "dev",
  release: "1.0.0",
});

// 显式停止（可选）；页面卸载会自动 sendBeacon 兜底
await ctrl.stop();
```

## 3. 触发信号，回 console 看会话

打开被观测页，点按钮、发请求、抛个错——console 的 **会话浏览器** 会列出本次会话，点进去即可回放，时间轴上 DOM 变化与 error / console / network 信号交错对齐。

## 下一步

- 了解会话 / 段 / 信号模型 → [核心概念](./concepts)
- Web 应用完整接入（离线采集、脱敏、框架集成）→ [Web SDK](./web)
- Tauri 桌面应用接入 → [Tauri Plugin](./tauri)
- 私有云 / 团队部署 → [私有化部署](./deploy)
