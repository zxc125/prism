# P8：云端 server 抽取 + Backend 抽象

> 阶段路径第 8 阶段（规划中）。目标：console 可连自托管云端 server——ingest + storage + 读 API 抽成独立服务，console 加 Backend 抽象，本地/云端切换。

## 目标

私有云自托管：用户在自己服务器下载安装运行 `observer-server`，SDK 上报到云端，console 连云端读/回放；离线 bundle 也可上传云端。

与锁定决策 #1 的关系：**自托管 ≠ SaaS RUM**（用户 own 数据、零厂商云依赖），作为 opt-in 拓扑，本地优先保持默认。

## 范围

1. **observer-server crate**：复用 `tauri-plugin-observer::storage`（已与 AppHandle 解耦）+ `handle_route` ingest 路由；加读 API（`GET /sessions`、`GET /sessions/:id`、annotations、export）+ `POST /sessions/import`（bundle 上传）；可绑 `0.0.0.0`，配置 `bind`/`dataDir`/`auth`/`tls`。
2. **console 内嵌 server 退化**：[ingest.rs](../../src-tauri/src/ingest.rs) 退化为 `observer-server` 绑 127.0.0.1 的薄封装，同一份代码。
3. **Backend 抽象**：console 前端 `Backend` 接口（`TauriBackend`=invoke / `HttpBackend`=HTTP），设置页切换，默认 Tauri。录制 Sink 与 Backend 正交。
4. **鉴权**：每租户 API key（`Authorization: Bearer`），server 映射 key -> tenantId + appId 集合；TLS 建议反代（nginx/caddy）终止。
5. **单租户起步**：本阶段先单租户（一个 key、扁平目录），多租户留 P9。

## 改动

- 新 crate `observer-server`（或在 plugin 内 `server` feature + binary）。
- [lib.rs](../../src-tauri/src/lib.rs)：内嵌 server 改调 observer-server。
- console：`Backend` 接口 + `TauriBackend`/`HttpBackend`；[SettingsView.vue](../../src/views/SettingsView.vue) 加「云端连接」（endpoint + API key）。
- [sinks.ts](../../packages/observer-sdk/src/sinks.ts) `HttpSink.endpoint` 已可指云端，无需改。

## 验收

- 自托管 server 二进制跑起来，SDK `HttpSink` 上报到云端，console 连云端读/回放。
- 离线 bundle `POST /sessions/import` 上传云端，回放还原。
- 本地模式（默认）行为不变。

> 备注：交付物为单静态二进制（Rust cross-compile）或 Docker 镜像，「下载安装运行 + 最小配置（bind/dataDir/API key/TLS）」。最大块是 console 需 web 可访问——本阶段用 Tauri-as-cloud-client（装一次 desktop client 连云端）覆盖，不做浏览器版（留 P10）。详见 [离线导出与云端部署（方案）.md](../架构/离线导出与云端部署（方案）.md) Phase B。
