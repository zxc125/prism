# 快速开始

**Prism 是什么**：一个本地优先的前端观测平台。它把用户在你应用里的操作录成「可回放的 DOM 快照流」，并把报错、console 输出、网络请求作为诊断信号交错进同一条时间轴——出问题时，你可以像看监控录像一样回放「用户当时看到了什么」，同时看到「代码当时发生了什么」。所有数据都留在你自己的机器上。

本页带你从零跑通最小闭环，预计 5 分钟：

1. 起 console（接收端）→ ✅ 主窗口出现
2. 跑一个自带 SDK 的示例应用 → ✅ 页面显示「采集中」
3. 回 console 看会话与回放 → ✅ 会话列表出现记录
4. （可选）接入你自己的应用

## 前置条件

| 工具 | 版本 | 用途 | 检查 |
| --- | --- | --- | --- |
| Node.js | ≥ 18 | 运行所有前端部分 | `node -v` |
| pnpm | ≥ 8 | 包管理器 | `pnpm -v`（没有：`npm i -g pnpm`） |
| Rust 工具链 | stable | 仅第 1 步需要（console 是 Tauri 桌面应用） | `rustc --version`（没有：装 [rustup](https://rustup.rs)） |
| 浏览器 | 现代版本 | 打开示例应用 | — |

## 第 1 步：起 console

console 是数据落盘 + 回放分析的地方，形态是 Tauri 桌面应用：

```sh
git clone https://github.com/zxc125/prism.git
cd prism
pnpm install
pnpm tauri dev
```

**✅ 验证**：Prism 主窗口出现。

打开 **设置页**（侧边导航 → 设置），记下两样东西，后面要用：

- **本地 server 地址**：默认 `http://127.0.0.1:1421`
- **token**（可选）：默认不开启鉴权；一旦开启，被测应用必须传同一个 token

## 第 2 步：跑示例应用（零代码）

仓库自带一个已接入 SDK 的被测示例 [examples/web-demo](https://github.com/zxc125/prism/tree/main/examples/web-demo)，不用写一行代码：

```sh
# 新开一个终端，仍在仓库根目录
pnpm dev:web-demo
```

然后浏览器打开 `http://localhost:1422`。

**✅ 验证**：页面右上角状态徽标显示「采集中」（绿色）。

随手点点页面上的按钮、输入框、开关——这些操作正在被录下来。

## 第 3 步：回 console 看会话

回到 Prism 主窗口，进入 **会话浏览器**。

**✅ 验证**：列表里出现一条来源为 web、appId 为 `web-demo` 的会话。点进去即可回放：拖动时间轴上的播放头，DOM 画面逐步重现，error / console / network 信号与画面交错对齐——你点过的按钮、发过的请求，都发生在它们该发生的时间点上。

## 第 4 步（可选）：接入你自己的应用

在你的应用里装 SDK，在应用入口调用一次 `init()`：

```sh
pnpm add @prism-obs/observer-sdk
```

```ts
import { init } from "@prism-obs/observer-sdk";

init({
  appId: "my-app",
  endpoint: "http://127.0.0.1:1421", // 第 1 步在设置页记下的地址
  // token: "…",                     // console 开启鉴权时必传
});
```

刷新你的应用，回 console——你的应用会话应该已经出现在列表里。更多能力（录制量控、离线采集、脱敏、框架集成）见 [Web SDK](./web)。

## 故障排查

| 现象 | 多半是 | 解法 |
| --- | --- | --- |
| `pnpm tauri dev` 报 Rust / cargo 错误 | 没装 Rust 工具链 | 装 [rustup](https://rustup.rs) 后重试 |
| 示例页右上角显示「连接失败」（红） | console 没在跑 / endpoint 不对 / token 不匹配 | 确认主窗口开着；对照设置页的地址与端口；开启鉴权时应用侧要传同一个 token |
| 会话列表一直是空的 | endpoint 或 token 不匹配 | 打开被测页 DevTools → Network，找发往 `127.0.0.1:1421` 的 `/ingest` 请求，按状态码定位（401 = token 不对，连不上 = console 没起） |
| 启动报端口被占用 | 1420 / 1421 / 1422 被其他进程占用 | 关掉占用进程；或在设置页改 server 端口，并同步改应用侧 endpoint |

## 下一步

- 搞懂会话 / 段 / 信号模型 → [核心概念](./concepts)
- 完整接入：录制量控、离线采集、脱敏、框架集成 → [Web SDK](./web)
- Tauri 桌面应用多窗口接入 → [Tauri Plugin](./tauri)
- 团队 / 私有云部署 → [私有化部署](./deploy)
