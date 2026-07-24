# bundle 规范（rrweb-demo-session）

会话跨进程/跨机迁移的唯一 interchange 契约。TS 实现（[`packages/observer-sdk/src/bundle.ts`](../../packages/observer-sdk/src/bundle.ts)）与 Rust 实现（[`src-tauri/src/lib.rs`](../../src-tauri/src/lib.rs) `build_export_bundle`/`write_import_bundle`）共同遵守本规范。

## 顶层结构

```json
{
  "format": "rrweb-demo-session",
  "version": 1,
  "exportedAt": 1750000000000,
  "session": { ... },
  "windows": [ ... ],
  "segments": { "<segmentId>": [ ...events ] },
  "annotations": [ ... ]
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `format` | string | 固定 `"rrweb-demo-session"`，标识本规范 |
| `version` | number | 格式版本，当前 `1`。import 侧拒绝高于已知的版本 |
| `exportedAt` | number | 导出时间（epoch ms） |
| `session` | object | `session.json` 内容（见下） |
| `windows` | array | `windows.jsonl` 逐行：窗口生命周期事件 |
| `segments` | object | key=segmentId，value=该段 rrweb 事件数组（含 `type:6` 交错信号） |
| `annotations` | array | `annotations.jsonl` 逐行：用户标注 |

## session 对象

```json
{
  "id": "1750000000000",
  "startedAt": 1750000000000,
  "endedAt": 1750000005000,
  "source": "self" | "web" | "tauri",
  "appId": "my-app",
  "env": "dev",
  "release": "1.0.0",
  "userAgent": "...",
  "viewport": "1920x1080",
  "url": "https://...",
  "importedAt": 1750000006000
}
```

- `id`/`startedAt` 必填；`endedAt`/`source`/`appId`/... 可选。
- 导入时 `id` 由接收方重新分配（避免冲突），原 id 被覆盖；`importedAt` 打戳。

## windows 行

```json
{ "type": "shown" | "hidden" | "focus", "label": "main", "segmentId": "main#0", "t": 1750000000000 }
```

## segments

key 是 segmentId，形如 `<label>#<n>`（如 `web#1`、`main#0`）。value 是 rrweb 事件数组，事件带绝对 `timestamp`（epoch ms），`type:6` 为交错诊断信号（`data.plugin` = `error`/`console`/`network`）。

## annotations 行

```json
{ "id": "a1", "t": 100, "label": "main", "text": "这里卡了", "author": "local", "createdAt": 1750000001000 }
```

`id`/`t`/`text` 必填；`label`/`author`/`createdAt` 可选。

## 安全约束（import 侧必须执行）

- **segmentId 校验**：必须匹配 `^[A-Za-z0-9_-]+#[0-9]+$`。segment key 会成为文件名（`segments/<key>.jsonl`），未校验会导致路径穿越（`../` 写穿目录）。这是 B1 安全修复的核心。
- **拒绝未知高版本**：`version > 已知` 时拒绝导入，避免误解析。
- **大小/数量上限**：服务端 import 需限制 bundle 体积、segment 数、单段事件数（防 DoS）。
- **原子写**：import 先写临时目录，成功后 rename，不留半成品会话。

## 版本演进策略

- 不破坏性变更（加可选字段）：不 bump version，旧 import 兼容。
- 破坏性变更（字段语义改/移除）：bump version，import 侧按 version 分支处理，旧版仍可导入。
- 共享测试 fixture：同一份 bundle 样例同时验证 TS `parseBundle` 与 Rust `write_import_bundle`（待补）。

## 三条投递路径共用本格式

1. 本地文件分享：`export_session` -> bundle 文件 -> `import_session` / `import_session_path`。
2. 云端上传：`POST /sessions/import`（Phase B），body 即本 bundle。
3. 离线 SDK：`IndexedDBSink` 读路径 -> `buildBundle` -> 下载 / 上传。
