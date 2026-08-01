# 核心概念

理解四个概念，就理解了 Prism 的全部数据模型。

## 会话（Session）

**一次会话 = 一次连续的观测**。Web SDK 里会话 = 一次页面访问；SPA 内路由切换算同一会话，整页刷新开新段。会话落盘为一个目录：

```
recordings/<sessionId>/
  session.json          # { id, startedAt, endedAt?, source?, appId?, ... }
  windows.jsonl         # 窗口生命周期：shown / hidden / focus，带 segmentId
  segments/<label>#<n>.jsonl   # 每段 rrweb 事件流
  annotations.jsonl     # 用户标注（session 级，与事件流分离）
```

## 段（Segment）

rrweb 以事件流录制 DOM。一个窗口「显示 → 隐藏」之间的事件流 = **一段**，文件名 `<label>#<n>`（如 `web#0`、`main#1`）。多窗口应用里，每个窗口各有若干段，靠**绝对时间戳**在主时间轴上对齐回放。

## 交错事件模型（type:6 诊断信号）

Prism 不把 error / console / network 单独存成日志，而是把它们包装成 rrweb 的 plugin 事件（`type: 6`），**交错进同一条事件流**，与 DOM 共享时间轴：

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

回放时，DOM 画面与诊断信号在同一时间轴上同步呈现——你看到按钮点下的同时，看到那一次失败的请求和抛出的错误。这是「棱镜分光」隐喻的来源：一束用户行为，折射成 DOM 流与信号流。

## 来源（Source）

每个会话标记来源，console 用不同轨道色区分：

| 来源 | 含义 | 轨道色 |
| --- | --- | --- |
| `self` | console 自录（本机观测） | 琥珀 |
| `web` | Web SDK 上报 | 冷青 |
| `tauri` | Tauri Plugin 上报 | lane-5 |

## bundle 契约

会话可序列化为 `prism-session` bundle——一个 JSON 文件，是**会话跨进程 / 跨机迁移的唯一契约**：

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

三条传输拓扑共用它：本地文件分享、本地 server 实时流、离线录 + 上传。离线采集（[Web SDK](./web) 的 `recordOffline`）先把数据落 IndexedDB，再 `export` 成 bundle 下载或上传。
