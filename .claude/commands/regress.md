---
description: 回归 — spawn fresh subagent 独立验证 + 产出测试流程文档（/cycle 第 4 步）
---

# /regress — 回归

## 前置

承接 `/impl` 的代码改动。当前会话应该刚完成实施且 gate（类型检查/编译）通过。

## 你的角色

你是回归协调者。但你**不是验证者**——验证交给一个 fresh context 的 subagent，避免你的确认偏差（「我写的所以我确认没问题」）。

## 流程

### 1. 梳理改动面

运行 `git diff` 拿到改动全貌，列出本次改动触及的验收线（参考 P 文档的实施顺序表）。

### 2. spawn fresh subagent 独立验证

用 Agent tool spawn 一个 **general-purpose** agent（fresh context，不知道主 agent 怎么写的）。给它的指令：

> 读 `docs/阶段路径/Pxx-*.md` 和当前 git diff，独立验证本次改动是否满足 P 文档的实施顺序表每行的「可验证产出」。执行：
> 1. 按改动面跑测试：
>    - Rust 改动 → `cargo test`（受影响 crate）
>    - 前端改动 → `pnpm build`（含 vue-tsc）
>    - 官网改动 → `pnpm build:site`
> 2. 复核契约：新增的函数签名/配置格式/API 是否与方案文档一致？
> 3. 列出发现的任何不一致、缺失或回归
> 
> 返回：(a) 测试结果（通过数/失败数）、(b) 契约复核结论、(c) 发现的问题清单

**不要替 subagent 跑测试**——让它独立跑、独立判断。它的结论可能与你不一致，那正是价值所在。

### 3. 处理 subagent 发现

- 如果 subagent 发现问题 → 主 agent 修复，修完再 spawn 一次 subagent 复核（或主 agent 自验）
- 如果 subagent 确认通过 → 继续

### 4. 产出测试流程文档

用 `docs/模板/测试流程.md` 起草 `docs/测试/Pxx-测试流程.md`（接续 P10 之后断档，P11 起补回）：
- 覆盖本次每条验收线的复现/验证步骤
- 前序 P 回归项
- `cargo test` 的通过数字

如果该 P 已有测试流程文档，增量补充而非重写。

### 5. 更新 CLAUDE.md 验证数字

把最新的 `cargo test` 数字、`vue-tsc` 结果、site build 结果记录下来，供 `/sync-docs` 写入进度表。

### 6. 呈现结果

把 subagent 结论 + 测试流程文档 + 测试数字汇总呈现，**建议执行 `/sync-docs`**。

## Gate（过关条件）

- [ ] fresh subagent 独立验证通过（或问题已修完复验）
- [ ] `cargo test` / `pnpm build` / `pnpm build:site` 按改动面通过
- [ ] 测试流程文档落盘（用模板）
- [ ] CLAUDE.md 进度表「验证」列数字已就绪
- [ ] 用户确认进 `/sync-docs`

## 注意

- **subagent 的价值在于独立判断**——不要在 prompt 里暗示结论（如「这应该没问题」）
- subagent 也可能复述主 agent 的结论（O3 风险）——如果发现它只是确认而没独立检查，重新给更具体的指令
- 如果项目还没有 JS/TS 测试框架（当前没有），前端回归靠 `pnpm build` 类型检查 + 手动验证步骤，不要假装有单测
