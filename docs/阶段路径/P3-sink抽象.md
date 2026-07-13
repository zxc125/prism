# P3：Sink 抽象

> 阶段路径第 3 阶段。目标：把录制器与 Tauri 解耦，为外部观测（P4/P5）铺路。

## 目标

抽 `Sink` 接口，现有自录迁移到 `TauriSink`，加 `HttpSink`/`IndexedDBSink` 骨架。不改存储格式、不改回放。

## 范围

抽 `Sink` 接口（定义见 [被观测侧](../架构/被观测侧（采集）.md)），现有自录迁移到 `TauriSink`，加 `HttpSink`/`IndexedDBSink` 骨架。

## 改动

1. 定义 `Sink` 接口（`startSession` / `beginSegment` / `appendEvents` / `appendLifecycle` / `endSession`）。
2. [useRecorder.ts](../../src/composables/useRecorder.ts) 的 `invoke` 调用全部走注入的 `Sink`；[P2](./P2-诊断信号采集.md) 的信号采集逻辑保持不变（只依赖 emit）。
3. `TauriSink`：包装现有 `invoke` 命令，行为不变。
4. `HttpSink`：POST `/ingest/*`，批量 + 重试 + unload beacon（骨架，[P4](./P4-Web-SDK.md) 接入真实 server 后联调）。
5. `IndexedDBSink`：本地缓存骨架（纯 web 独立回放场景预留）。

console 自录仍用 `TauriSink`（进程内、零开销）；外部采集器用 `HttpSink`。

## 不做

- 不起 HTTP server（[P4](./P4-Web-SDK.md)）。
- 不打包 SDK（[P4](./P4-Web-SDK.md)）。
- 不抽 Tauri plugin（[P5](./P5-Tauri-Plugin.md)）。

## 验收

- self-obs 行为完全不变（回归 [P2](./P2-诊断信号采集.md) 的信号采集）。
- `useRecorder` 内无直接 `invoke`，全部走 Sink。
- `pnpm build` 通过。
