# P7：离线采集与 bundle 契约

> 阶段路径第 7 阶段。目标：离线闭环 + 统一 bundle 契约 + import 安全加固——用户可脱离 console 自行录制、导出 bundle，上传后回放；bundle 成为跨进程/跨机迁移的唯一契约。

## 目标

1. **离线闭环**：被观测 web 应用不连 console 也能录制，导出 `prism-session` bundle，上传 console 回放。
2. **bundle 统一契约**：三路投递（本地文件 / 本地 server / 云端上传）收敛到同一 bundle 格式，TS 与 Rust 共守规范。
3. **import 安全加固**：修路径穿越（B1）、加版本校验、原子写。

## 范围

1. **bundle 契约**：[bundle-规范.md](../架构/bundle-规范.md) + TS `buildBundle`/`parseBundle`/`validateSegmentId`。
2. **IndexedDBSink 补全**：多 store schema + 读路径（`readSession`/`listSessions`/`clearSession`/`clearAll`）+ `endSession` 标 endedAt/补 hidden，与 HttpSink 行为对齐。
3. **recordOffline API**：`recordOffline()` / `OfflineController`（stop/export/download/list/clear/destroy），与 `init()`（HttpSink）并列，差别仅在 Sink。
4. **B1 安全**：`validate_segment_id` + `write_import_bundle` 校验 + `import_session` 版本校验 + 原子写（tmp + rename）。
5. **import_session_path**：Rust 读文件避免大 JSON 过 IPC；挂 `tauri-plugin-dialog`，MainView 用原生选择器。
6. **redaction 钩子**：`redact()`（stripNetworkBody/Headers、scrubbers、dropNetwork/Console），接到 export/download。

## 改动

- SDK：[bundle.ts](../../packages/observer-sdk/src/bundle.ts)、[redact.ts](../../packages/observer-sdk/src/redact.ts) 新增；[sinks.ts](../../packages/observer-sdk/src/sinks.ts)（IndexedDBSink 重写）、[index.ts](../../packages/observer-sdk/src/index.ts)（recordOffline + 导出）、[types.ts](../../packages/observer-sdk/src/types.ts)（Annotation/OfflineSessionData）。
- 后端：[lib.rs](../../src-tauri/src/lib.rs)（`validate_segment_id`/`import_session_content`/`import_session_path`/原子写/版本校验/常量 + 2 测试）；[Cargo.toml](../../src-tauri/Cargo.toml) + [capabilities/default.json](../../src-tauri/capabilities/default.json)（dialog 插件）。
- 前端：[MainView.vue](../../src/views/MainView.vue)（dialog 原生选择器 + `import_session_path`）。
- 文档：[bundle-规范.md](../架构/bundle-规范.md)、[离线导出与云端部署（方案）.md](../架构/离线导出与云端部署（方案）.md)。

## 验收

- 离线录制 -> stop -> download -> console 导入 -> 回放 + 诊断信号还原。
- 恶意 bundle（`../evil#0`）被拒、不落盘；高版本 bundle 被拒。
- 大 bundle 走 `import_session_path`，不过 IPC 大字符串。
- `cargo test`（9 passed）+ `pnpm build` 通过。

> 落地：bundle 格式 `{ format, version:1, exportedAt, session, windows, segments{<id>:events}, annotations }`，TS `parseBundle` 与 Rust `write_import_bundle` 共守 [bundle-规范.md](../架构/bundle-规范.md)，segmentId 校验 `^[A-Za-z0-9_-]+#[0-9]+$`（路径穿越防护）。`IndexedDBSink` 升 v2 多 store（sessions/segments/events/lifecycle 带索引），`beginSegment` 记 shown、`endSession` 为开段补 hidden——与 `/ingest/segment` + `/ingest/session/end` 对齐，离线会话回放侧无感。`recordOffline` = `SegmentRecorder` + `IndexedDBSink`，`export` 经 `redact`（可选）-> `buildBundle`。`import_session` 拆出 `import_session_content` 共用，`import_session_path` 读文件调同一逻辑；原子写先落 `<id>.tmp` 再 rename，失败清理。MainView 导入换 `@tauri-apps/plugin-dialog` `open()` + `import_session_path`，移除 `<input type=file>`/FileReader。redaction 默认剥离 network body/headers，`scrubbers` 正则替换 url/args。

> 编译验证：`cargo test --lib` 9 passed（含 `segment_id_validation`/`import_rejects_path_traversal`）；SDK `tsc --noEmit` 零错误；`pnpm build`（vue-tsc + vite）通过。运行时 E2E（离线录 -> 导出 -> 导入 -> 回放）待实测。
