---
description: 全流程编排 — 串起 spec→plan→impl→regress→sync-docs，每步 gate 通过才进下一步
argument-hint: <一句话需求>
---

# /cycle — 全流程编排

## 输入

需求：$ARGUMENTS

## 你的角色

你是开发流编排者。你串起五个阶段，每步 gate 通过且用户确认才进下一步。你不替各阶段做决定——你只负责**编排 + gate 检查 + 用户确认衔接**。每一步的具体执行参照对应 command 的完整流程。

## 流程

依次执行 5 步，每步完成后回到你这里做 gate 检查 + 用户确认：

### Step 1: /spec — 需求评审

执行 `/spec` 的完整流程：复述需求 → spawn Explore 对照 → 必答 4 问 → 产出决策文档 + 方案文档。

**gate**：4 问有明确答案 + 文档落盘 + 用户确认归属判定。

**分流判断**（gate 后）：
- spec 判定「新 P / P.x」→ 继续 Step 2
- spec 判定「已有 P 内局部修」→ 提醒用户：可改走 `/fix` 轻量路径（`/cycle` 五步对小修偏重）。用户坚持 `/cycle` 则继续

### Step 2: /plan — 拆分阶段

执行 `/plan` 的完整流程：起草 P 文档（用模板）+ TodoWrite 分解。

**gate**：P 文档五段齐全（关键陷阱/修复记录除外）+ 实施顺序每行可验证 + 用户确认。

### Step 3: /impl — 开发

执行 `/impl` 的完整流程：dispatch skills（视觉→frontend-design，录制→rrweb-recording）+ 实施 + 类型检查/编译。

**gate**：Todo 全勾 + `pnpm build`/`cargo check`/`pnpm build:site` 按改动面通过 + 视觉/录制走过 skill + 用户确认。

**异常处理**：impl 过程发现需要改契约/动锁定决策 → 停下，回 Step 1 补 spec。

### Step 4: /regress — 回归

执行 `/regress` 的完整流程：spawn fresh `general-purpose` subagent 独立验证 + 跑测试 + 产出测试流程文档。

**gate**：subagent 独立验证通过 + `cargo test`/`pnpm build`/`pnpm build:site` 按改动面通过 + 测试流程文档落盘 + 用户确认。

### Step 5: /sync-docs — 更新文档

执行 `/sync-docs` 的完整流程：CLAUDE.md 进度表 + P 文档状态 + 回写架构/决策检查。

**gate**：文档与代码对齐 + 用户确认。

## 编排原则

- **每步必须用户确认才进下一步**——不自动连跑（step 内部子步骤可连续）
- **gate 可经用户确认跳过**——用户说"跳过 regress"或"不需要测试流程文档"，尊重决定但记录跳过原因
- **升格触发**——任何一步发现需要升格（impl 改了契约 / fix 跨 P），停下回 Step 1
- **降级触发**——spec 判定已有 P 内补全，提醒改走 `/fix`

## 完成标志

- [ ] 5 步全部完成（或用户确认跳过某步 + 记录原因）
- [ ] 每步 gate 有明确结论
- [ ] 最终产出清单：决策文档 + 方案文档 + P 文档 + 测试流程文档 + CLAUDE.md 进度表更新
