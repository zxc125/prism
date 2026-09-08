# 核心概念

读完这页，你能看懂 Prism 的全部数据模型：会话、段、交错事件、来源、bundle，以及「录制量控」在模型里的位置。建议第一次使用前通读一遍，遇到名词疑惑再回来查。

## 先说清楚：录的是「结构快照」，不是视频

Prism 的画面录制基于 [rrweb](https://github.com/rrweb-io/rrweb)：它不录屏幕像素，而是把页面 DOM 树及其变更**序列化成一条结构化事件流**——首次全量快照，之后只记增量（某个节点变了、鼠标动了、输入了什么）。回放时在沙箱 iframe 里按事件流重建画面。

这带来三个与「录屏」的本质差异：

- **体积小、可检索**：存的是结构化 JSON，不是视频帧；
- **隐私可控**：敏感区域可以用 CSS selector 声明「免录区」，根本不进事件流；
- **与诊断信号同轴**：因为都是结构化事件，报错/请求可以和画面精确对齐（见下文交错事件模型）。

代价是：rrweb 只录 DOM 声明的样式，不录浏览器画布默认色（深色页面要注意，见 [Web SDK 的背景色须知](./web#被观测页需显式设置背景色)）。

## 会话（Session）

**一次会话 = 一次连续的观测**。Web SDK 里会话 = 一次页面访问；SPA 内路由切换算同一会话，整页刷新才开新会话。会话落盘为一个目录：

```
recordings/<sessionId>/
  session.json          # { id, startedAt, endedAt?, source?, appId?, ... }
  windows.jsonl         # 窗口生命周期：shown / hidden / focus，带 segmentId
  segments/<label>#<n>.jsonl   # 每段 rrweb 事件流
  annotations.jsonl     # 用户标注（session 级，与事件流分离）
```

## 段（Segment）

一个「窗口」从显示到隐藏之间的连续事件流 = **一段**，文件名 `<label>#<n>`（如 `web#0`、`main#1`）。

Web 单页应用里通常只有一个隐含的「窗口」，段接近「一次刷新后的一段连续画面」。**多窗口应用**（如 Tauri 桌面应用开了 3 个窗口）里，每个窗口各有若干段；所有窗口共享墙上时钟，事件带绝对时间戳，回放时按「显示 ~ 隐藏」区间在一条主时间轴上同步驱动——这就是多轨时间轴的由来。

## 交错事件模型（type:6 诊断信号）

Prism 不把 error / console / network 单独存成日志文件，而是把它们包装成 rrweb 的 plugin 事件（`type: 6`），**交错进同一条事件流**，与 DOM 共享时间轴：

```jsonc
{
  "type": 6,                 // 诊断信号
  "timestamp": 1754000000000,
  "data": {
    "plugin": "network",     // error | console | network
    "payload": { "url": "/api/order", "method": "POST", "status": 500, "duration": 42 }
  }
}
```

回放时，画面与诊断信号在同一时间轴上同步呈现——你看到按钮点下的同时，能看到那一次失败的请求和抛出的错误。这是「棱镜分光」隐喻的来源：一束用户行为，折射成 DOM 流与信号流。

## 录制量控（recording 档位）

高频页面（行情推送、密集动画）会产生海量 DOM 变更事件。SDK 提供 `recording` 配置：`full`（全量，默认）/ `balanced`（收紧高频交互）/ `minimal`（仅关键交互），外加 `domBlocks` 免录区（命中元素及其子树不进事件流，回放显示占位）。详见 [Web SDK · 录制量控](./web#录制量控-recording)。

## 来源（Source）

每个会话标记来源，console 会话列表与回放轨道用不同颜色区分：

| 来源 | 含义 | 轨道色 |
| --- | --- | --- |
| `self` | console 自录（本机观测） | 琥珀 |
| `web` | Web SDK 上报 | 冷青 |
| `tauri` | Tauri Plugin 上报 | 绿 |

## bundle 契约

会话可序列化为 `prism-session` bundle——**一个 JSON 文件装下整个会话**（元信息 + 窗口生命周期 + 全部事件流 + 标注），是会话跨进程 / 跨机迁移的唯一契约：

```jsonc
{
  "format": "prism-session",
  "version": 1,
  "session": { /* ... */ },
  "windows": [ /* ... */ ],
  "segments": { "web#0": [ /* rrweb events */ ] },
  "annotations": [ /* ... */ ]
}
```

三条传输拓扑共用它：本地文件分享、本地 server 实时流、离线录 + 上传。离线采集（[Web SDK](./web) 的 `recordOffline`）先把数据落浏览器 IndexedDB，再 `export` 成 bundle 下载或上传。
