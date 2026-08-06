---
description: 开发 — 按 plan 实施，dispatch skills，跑类型检查/编译（/cycle 第 3 步）
---

# /impl — 开发

## 前置

承接 `/plan` 的 P 文档实施顺序表 + TodoWrite 清单。如果当前会话没有 plan 产出，先确认基于哪份 P 文档开工。

## 你的角色

你是实施者。按 P 文档的实施顺序表逐步落地代码。**长上下文是你的资产**——你负责跨文件协调，不拆 subagent。

## 流程

### 1. 确认实施范围

读 P 文档实施顺序表 + TodoWrite，确认本次 `/impl` 覆盖哪几个阶段（可以一次做一个阶段，也可以连做）。向用户确认范围后开始。

### 2. 改动预检（pre-flight gate — 输出微方案，用户确认后才动手）

**不要直接开始改代码。** 基于 plan 的实施顺序表，先输出一份「改动微方案」呈现给用户。微方案必须包含：

1. **目标**：本次要做什么（一句话）
2. **设计依据**：基于 P 文档哪条实施顺序 / 什么设计决策
3. **影响范围**：
   - 文件清单（逐文件：新增 / 修改 / 删除 + 一句话改动摘要）
   - 上下游影响（涉及哪些模块、依赖链、是否有连带改动）
4. **技术方案**：
   - 具体怎么改（关键代码结构 / 函数签名 / 数据流 / 配置格式）
   - 有替代方案时列出对比 + 选择理由（为什么不选另一个）
5. **风险标注**：
   - 是否动 CLAUDE.md「四条锁定决策」/ bundle 契约 / 对外 API
   - 是否有破坏性变更（改签名 / 改格式 / 删功能）
   - 中/高风险项单列
6. **验证方式**：改完怎么验证（跑什么命令 / 手动验证步骤）

→ **用户确认后才进 Step 3 实施**。用户要求调整就改微方案再确认。
→ 预检发现影响面超预期（触及多模块 / 动契约 / 动锁定决策）→ 停下，建议升格 `/cycle`。
→ 这和 `/spec` 的方案文档不同——spec 是设计层面，这里是你**即将动手的最后一关**，具体到文件级。

### 3. dispatch skills（按改动面）

改动面决定是否触发 skill。**先读 skill 再动手**，不要凭记忆：

| 改动面 | 触发 skill | 何时读 |
|---|---|---|
| 视觉/样式（新增视图、改配色、改排版、改交互形态） | `/frontend-design` | **写第一行 CSS/模板前** |
| 录制/回放/session/segment/useRecorder/usePlayer | `/rrweb-recording` skill | **动录制/回放代码前** |

CLAUDE.md「视觉设计约定」是硬约束：任何视觉变更**必须先走 frontend-design**，不要直接套 Element Plus 默认或通用暗色模板。设计 token 在 `src/styles/theme.css`，轨道色在 `usePlayer.ts` 的 `LANE_COLORS`。

### 4. 实施约定（遵守 CLAUDE.md）

前端：
- 自动导入已开（Vue API + Element Plus API/组件）—— **不要手动加这些 import**
- Tauri API 依赖走 `src/composables/tauri.ts` 抽象 —— 不要直接 `import { invoke } from "@tauri-apps/api/core"`
- UI 文案中文（zh-CN）
- P10 组件结构：每视图 < 200 行、单一职责

后端：
- 新增 Tauri command 必须在 `lib.rs` 的 `generate_handler![]` 注册
- 新增带新 label 模式的路由 → 同步改 `src-tauri/capabilities/default.json`

### 5. Gate（过关条件）

每完成一个实施阶段，跑对应检查：

| 改动面 | 检查命令 | 失败处理 |
|---|---|---|
| 前端（`src/`） | `pnpm build`（含 `vue-tsc --noEmit`） | 类型错误必须修，不跳过 |
| Rust（`src-tauri/`、`crates/`） | `cd src-tauri && cargo check` 或受影响 crate 内 `cargo check` | 编译错误必须修 |
| 官网（`site/`） | `pnpm build:site` | 构建失败必须修 |

### 6. 完成一个阶段后

- TodoWrite 标完成
- P 文档实施顺序表对应行状态 📋→🚧→✅
- 继续下一阶段，或如果用户要分段则停下来

### 7. 全部完成后

呈现改动汇总（文件清单 + Todo 完成情况），**建议执行 `/regress`**。

## Gate（整体过关条件）

- [ ] 改动预检微方案已通过用户确认
- [ ] TodoWrite 全勾（或标注未完成原因）
- [ ] `pnpm build` / `cargo check` / `pnpm build:site` 按改动面通过
- [ ] 视觉变更走过 `frontend-design` skill
- [ ] 录制/回放变更走过 `rrweb-recording` skill
- [ ] 未触碰 CLAUDE.md「四条锁定决策」（触碰 = 停下，回 `/spec`）
- [ ] 用户确认进 `/regress`

## 注意

- **不要在 impl 阶段写测试流程文档**——那是 `/regress` 的事
- **不要在 impl 阶段更新 CLAUDE.md 进度表**——那是 `/sync-docs` 的事
- 遇到方案没料到的陷阱（如 P13 的双重 vite 编译），记下来追加到 P 文档「关键陷阱」段
