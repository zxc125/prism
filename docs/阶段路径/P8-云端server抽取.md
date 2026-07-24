# P8：云端 server 抽取 + Backend 抽象

> 阶段路径第 8 阶段（✅ 已完成）。目标：console 可连自托管云端 server——ingest + storage + 读 API 抽成独立服务，console 加 Backend 抽象，本地/云端切换。

## 落地结果

- **[`crates/observer-storage`](../../crates/observer-storage)**：纯存储层（落盘原语 + annotations + bundle 契约 + `read_session`/`list_sessions`/`import_bundle_content` 等读/导入导出），零 tauri 依赖，全吃 `&Path`。`tauri-plugin-observer::storage` 只留 `recordings_root` + re-export。
- **[`crates/observer-server`](../../crates/observer-server)**：HTTP server（`ObserverServer` + 独立二进制 `observer-server`）。路由 = ingest `/ingest/*`（复用原 `handle_route`）+ 读 API `GET /sessions`、`GET /sessions/:id`、`GET /sessions/:id/annotations`、`POST /sessions/:id/annotations`、`PATCH /sessions/:id`、`GET /sessions/:id/export`、`POST /sessions/import`、`DELETE /sessions/:id`。可绑 `127.0.0.1`（console 内嵌）或 `0.0.0.0`（自托管），配置 `--bind`/`--data-dir`/`--token`（或环境变量）。
- **console [ingest.rs](../../src-tauri/src/ingest.rs)**：退化为 `ObserverServer` 绑 127.0.0.1 的薄封装，同一份代码。`IngestState` 持 server 句柄 + data_dir，`set_ingest_config` 热更新 token/enabled。
- **[lib.rs](../../src-tauri/src/lib.rs)**：所有命令收窄成 observer-storage 薄封装；新增 `read_text_file`（HttpBackend 导入路径：文件选择器拿 path -> 读内容 -> 上传云端）。
- **[backend.ts](../../src/composables/backend.ts)**：`Backend` 接口 + `TauriBackend`（invoke）/ `HttpBackend`（HTTP），localStorage 存配置，`getBackend()` 单例。[SettingsView.vue](../../src/views/SettingsView.vue) 加「云端连接」分组（模式 toggle + endpoint + API key）。[MainView.vue](../../src/views/MainView.vue) 表头加连接模式指示器，数据访问全走 `getBackend()`，录制 Sink 正交不动。
- **鉴权**：单一 Bearer token（空 token = 不鉴权，本机回环 dev 友好）。单租户起步，多租户留 P9。
- **验收**：`cargo test` 13 passed（observer-storage 10 + observer-server 3）；observer-server 二进制端到端 11 项冒烟全过（401 未鉴权 -> 建会话 -> ingest segment/events -> end -> list/read/export/import/PATCH/DELETE -> 404）；本地模式（默认）行为不变。

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
