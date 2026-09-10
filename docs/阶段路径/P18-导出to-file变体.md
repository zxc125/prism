# P18：导出 to-file 变体

> 阶段路径第 18 阶段。目标：插件新增 `export_session_to_file`（Local 模式，Rust 代执行大 JSON 序列化 + 落盘），根治导出链路②③热点。承接 P16（外部应用本地落盘）+ P17 补丁 0.2.2（改动 A：`export_session` async 化）。来源：bond/frontend 0910 提案改动 B。方案见 [导出to-file变体（方案）](../架构/导出to-file变体（方案）.md)。
>
> **进度**：🚧 进行中。spec ✅（决策 + 方案落盘），实施/回归/发版进行中。

## 目标 ★

bond/frontend 导出实测发现导出链路三热点（提案 §1）：①Rust 命令同步执行阻塞主线程（已由 0.2.2 修复）；②前端 `JSON.stringify` 大 bundle；③字节数组 + 大载荷 IPC 放大。②③是 P16 D5「返回数据、宿主落盘」形态的固有成本——大会话（几十 MB）导出冻结宿主全部窗口。本阶段给 D5 加第三形态：宿主传 `path`，插件代执行「序列化 + 写盘」，前端只传两个字符串，②③根治。

## 范围 ★

1. **覆盖**：插件新命令 `export_session_to_file` + 权限 + Rust 单测；observer-tauri TS 包装；手册 zh/en 补文档；发版 crates 0.3.0 + npm 0.5.0。
2. **不在本阶段**：console 前端切 to-file（留独立小改）；`read_session` 大载荷（P19+ 候选）；bond §4 兜底删除（bond 本仓决策）。

## 设计决策（已拍）★

| # | 决策点 | 选择 | 理由 |
|---|---|---|---|
| D1 | 命令签名 | `(app, state, session_id, path) -> Result<u64, String>` | 返回写入字节数，宿主可提示 |
| D2 | 门禁 | `ensure_local` + `validate_session_id`，与 `export_session` 同款 | P16 D6 + P7 防护复用 |
| D3 | 执行模型 | `async fn` + `spawn_blocking` 包住 build→serialize→write | 秒级操作不占 async worker；区别于 P17 D3 属性（新命令无包袱） |
| D4 | 落盘 | 同级 `.tmp` + `rename` 原子写 | P7 惯例，防残缺 bundle |
| D5 | D5 关系 | **扩展**（宿主仍决定落盘目标，插件代执行 IO） | 不推翻 P16 D5 原意 |
| D6 | 权限 | `allow-export-session-to-file` 入 `observer:default` | 与 export-session 同数据面，无能力升级 |
| D7 | 发版 | crates 0.3.0 + npm 0.5.0；observer-storage 不动 | additive minor 先例 |

论证细节见 [决策/导出to-file变体.md](../决策/导出to-file变体.md)；对 bond 提案的三处修正（验收 exportedAt 豁免 / spawn_blocking / 原子写）见方案 §5。

## 实施顺序 ★

| 阶段 | 内容 | 产出 | 状态 |
|---|---|---|---|
| 1.0 | /spec：决策 + 方案落盘 | [决策](../决策/导出to-file变体.md) · [方案](../架构/导出to-file变体（方案）.md) | ✅ |
| 1.1 | 插件命令 + 权限 + handler 注册 + 新增单测 ×3（Remote 闸门/坏 id 两案例由既有测试覆盖） | commands.rs / lib.rs / build.rs / default.toml | 🚧 |
| 1.2 | TS 包装 + 双包版本 + 手册 zh/en | index.ts / package.json ×2 / tauri.md ×2 | 📋 |
| 1.3 | 回归：cargo test + 包 typecheck/build + subagent 复核 + 测试流程文档 | docs/测试/P18-测试流程.md | 📋 |
| 1.4 | 发版 crates 0.3.0 + npm 0.5.0 + npm-crates发布.md + sync-docs | 两 registry 可拉 | 📋 |

## 关键陷阱（可选，实施时发现）

- **`State` 不能跨 `spawn_blocking` move**：门禁与路径拼装须在闭包外完成，`state.lock()` 不得跨 await 持锁（方案 §4.1 已按此写）。
- **验收「逐字节一致」陷阱**：`exportedAt: now_ms()` 每次调用必变，比对待实现为「剔除/固定 exportedAt 后逐字节一致」。

## 修复记录（/fix 追加）

- _（暂无）_
