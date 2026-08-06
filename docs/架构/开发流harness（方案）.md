# 开发流 harness（最佳实践方案）

本文档是把项目已有的「P 阶段」开发模式（P1–P13 一路走来的隐式约定）**显式化 + 工具化 + 强制化**的落地方案。载体为 Claude Code 的 slash commands + 文档模板 + hooks。目标是让「需求评审 → 拆阶段 → 开发 → 回归 → 更新文档」五步成为一个可重复、可审计、不依赖记忆的开发流。

实施细节（命令 prompt 全文、模板文件、hooks JSON）在落地阶段产出；本文给契约与影响范围，供评审。

---

## 1. 总览

### 1.1 一句话定位

**不引入新流程，只把现有流程显式化。** P1–P13 每次 de facto 都走了「想清楚 → 拆步 → 写 → 验证 → 更新进度表」这五步，但靠 CLAUDE.md 约定 + 自觉遵守。harness 把每一步绑定一个 slash command 入口 + 一份产出契约 + 一个过关条件（gate），并补上当前最弱的两环：**模板化**（P 文档结构靠手抄）和**回归可审计**（P10 之后测试流程文档断档）。

### 1.2 改动块

```
.claude/commands/      [新增] 7 个 slash command prompt（spec/plan/impl/regress/sync-docs/cycle/fix）
.claude/settings.json  [修改] 加 hooks（commit 前 type-check、Stop 提示未 sync）
docs/模板/              [新增] 4 份模板（决策/架构方案/P文档/测试流程）
docs/架构/              [新增] 本方案文件（已在此）
CLAUDE.md              [修改] 加「开发流程 harness」段，描述 7 个命令入口与约定
```

### 1.3 非目标

- **不引入测试框架**（Vitest 等）—— 那是另一个独立缺口，建议单开 P14，不混进 harness。
- **不引入 CI**（GitHub Actions 跑测试）—— 目前 CI 仅 `deploy-site.yml`，加测试 CI 是 P14 之后的事。
- **不改任何产品代码**（`src/`、`src-tauri/`、`crates/`、`packages/`、`site/`、`plugins/`）—— harness 纯流程层。
- **不替代既有 skills**（`frontend-design`、`rrweb-recording`）—— `/impl` 会 dispatch 它们，不重新实现。
- **不强制 worktree / 多 agent 并行**——单线流够用，进阶场景留接口（见 §10）。

---

## 2. 影响范围（评审重点）

### 2.1 新增文件（共 12 个）

| 路径 | 类型 | 说明 |
|---|---|---|
| `.claude/commands/spec.md` | command | `/spec <需求>` 需求评审入口 |
| `.claude/commands/plan.md` | command | `/plan` 拆分阶段入口 |
| `.claude/commands/impl.md` | command | `/impl` 开发入口（dispatch skills） |
| `.claude/commands/regress.md` | command | `/regress` 回归入口（spawn fresh subagent） |
| `.claude/commands/sync-docs.md` | command | `/sync-docs` 文档同步入口 |
| `.claude/commands/cycle.md` | command | `/cycle <需求>` 全流程编排 |
| `.claude/commands/fix.md` | command | `/fix <bug>` 修复短路径 |
| `docs/模板/决策.md` | 模板 | `docs/决策/*.md` 起草骨架 |
| `docs/模板/架构方案.md` | 模板 | `docs/架构/*（方案）.md` 起草骨架 |
| `docs/模板/P文档.md` | 模板 | `docs/阶段路径/Pxx-*.md` 起草骨架 |
| `docs/模板/测试流程.md` | 模板 | `docs/测试/Pxx-测试流程.md` 起草骨架 |
| `docs/架构/开发流harness（方案）.md` | 文档 | 本方案（已存在） |

### 2.2 修改文件（共 2 个）

| 路径 | 改动 | 风险 |
|---|---|---|
| `.claude/settings.json` | 在现有 `permissions`/`enabledPlugins` 之外加 `hooks` 段 | **低**：hooks 默认非阻断（仅提醒），配错最坏情况是多一条提示，不阻断工作流 |
| `CLAUDE.md` | 在「记忆与决策存放约定」之后插入「开发流程 harness」段（约 30 行） | **低**：纯文档增量，不改现有约定 |

### 2.3 不动的文件

- `src/**`、`src-tauri/**`、`crates/**`、`packages/**`、`site/**`、`plugins/**` —— 零产品代码改动
- 现有 `docs/阶段路径/*.md`、`docs/决策/*.md`、`docs/架构/*.md`、`docs/测试/*.md` —— 内容不动，模板只是给未来新文档用
- `package.json`、`pnpm-workspace.yaml`、`Cargo.toml`、`vite.config.ts` —— 零工程配置改动
- 现有 `.claude/skills/` —— 不改，被 `/impl` dispatch

### 2.4 风险评估

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| command prompt 写得不好，执行时跑偏 | 中 | 中 | 每个 command 的 body 先写最小版，用 1-2 次真实任务验证后迭代；落地阶段分两批（先 spec/plan/sync-docs，后 impl/regress） |
| hooks 配错阻断正常工作 | 低 | 高 | **全部 hooks 起步为非阻断**（仅 stdout 提醒）；需要阻断的 gate（如 commit 前 type-check）只在 `/regress` 内显式跑，不挂全局 hook |
| 模板与现有文档结构漂移 | 低 | 低 | 模板从 P9/P10/P13 + i18n 决策 **直接提炼**，落地时逐一核对样本 |
| harness 变成形式主义，每步都要走 | 中 | 中 | `/fix` 短路径 + `/cycle` 内 gate 可跳过（用户确认即可）避免一刀切 |

---

## 3. 设计决策

| # | 决策点 | 选择 | 理由 |
|---|---|---|---|
| D1 | harness 载体 | Claude Code slash commands + 模板 + hooks | 项目已在 Claude Code 内开发；commands 是原生入口，零额外工具链 |
| D2 | 是否引入新流程 | **不引入**，显式化现有 P 阶段流 | P1–P13 已验证；重新发明流程税大于收益 |
| D3 | `/fix` vs `/cycle` 边界 | 单 P 局部修 = `/fix`；跨阶段 / 动锁定决策 / 改契约 = 新 P 走 `/cycle` | 修复不该重走完整 cycle；但跨阶段修复影响面大，需完整文档 |
| D4 | `/fix` 修复记录写哪 | **回写原 P 文档**末尾「修复记录」段，不新建 `docs/修复/` | 修复上下文在原 P 文档最完整；分散反而难查 |
| D5 | 是否每阶段用 subagent | **否**，仅 `/spec`（Explore 对照）+ `/regress`（fresh 复核）两点用 | 主 agent 长上下文是资产；`/impl` 拆 subagent 会割裂。两点 subagent 价值不可替代：避免确认偏差 + fan-out 扫文档库 |
| D6 | 模板放哪 | `docs/模板/` | 与 `docs/架构/`、`docs/决策/` 同级，不动现有目录 |
| D7 | hooks 策略 | **起步全非阻断**（提醒为主）；阻断型 gate 放 command 内而非全局 hook | 避免配错锁死工作流；阻断逻辑显式可见更好审 |
| D8 | 进度表是否加行 | **不加**。`/fix` 只更新「验证」列数字 + 原 P 文档修复记录；`/cycle` 才加新 P 行 | P 阶段是范围单位不是时间单位；每次修复加行会污染表 |

---

## 4. 命令契约

每个 command 是 `.claude/commands/<name>.md`，frontmatter（`description`/`argument-hint`）+ body（prompt）。下表给契约，body 全文落地阶段产出。

### 4.1 `/spec` — 需求评审

| 项 | 内容 |
|---|---|
| 调用 | `/spec <一句话需求>` |
| 输入 | 一句话需求描述 |
| 产出 | `docs/决策/<topic>.md`（用模板）+ `docs/架构/<topic>（方案）.md`（用模板）+ 归属判定（新 P / P.x / 已有 P 内） |
| **必答 4 问（gate）** | (1) 是否动 CLAUDE.md「四条锁定决策」？(2) 属于新 P、P.x、还是已有 P 内？(3) 影响哪些 crate/包/目录？(4) 是否涉及视觉变更（触发 `frontend-design`）？|
| subagent | **中段 spawn `Explore`** 并行扫全部 `docs/阶段路径/`+`docs/决策/`+`docs/架构/`，判断「是否已做过一半」「是否与既有锁定决策冲突」 |
| 完成标志 | 决策文档 + 方案文档落盘，4 问有明确答案，用户确认归属判定 |

### 4.2 `/plan` — 拆分阶段

| 项 | 内容 |
|---|---|
| 调用 | `/plan`（承接当前会话的 spec 产出） |
| 输入 | spec 阶段的决策 + 方案文档 |
| 产出 | `docs/阶段路径/Pxx-*.md`（用模板，五段结构：目标/范围/设计决策表/实施顺序/关键陷阱）+ TodoWrite 分解 |
| gate | 实施顺序表每行有可验证产出（不模糊）；关键风险点单列；指针落在真实文件 |
| 完成标志 | P 文档骨架落盘 + Todo 生成 + 用户确认开 `/impl` |

### 4.3 `/impl` — 开发

| 项 | 内容 |
|---|---|
| 调用 | `/impl`（承接 plan） |
| 输入 | P 文档实施顺序表 + Todo |
| 产出 | 代码改动 |
| dispatch | 视觉变更 → `frontend-design` skill；录制/回放/session/segment → `rrweb-recording` skill |
| **pre-flight gate** | **动手前输出改动微方案**（目标 + 影响范围 + 技术方案 + 风险标注 + 验证方式），用户确认才进实施 |
| gate | 前端改动跑 `pnpm build`（含 vue-tsc）；Rust 改动跑 `cargo check`（受影响 crate）；不动锁定决策 |
| 完成标志 | Todo 全勾 + 类型/编译通过 + 用户确认进 `/regress` |

### 4.4 `/regress` — 回归

| 项 | 内容 |
|---|---|
| 调用 | `/regress` |
| 输入 | impl 产出（代码改动 + diff） |
| 产出 | `docs/测试/Pxx-测试流程.md`（用模板，接续 P10 之后断档）+ 测试结果记录 |
| subagent | **spawn `general-purpose` fresh context**：独立读代码 + 跑测试 + 复核契约，避免主 agent 确认偏差 |
| gate | 按改动面跑：`cargo test`（受影响 crate）/ `pnpm build` / `pnpm build:site`；CLAUDE.md 进度表「验证」列有数字 |
| 完成标志 | 测试流程文档落盘 + 全部 gate 通过 + 用户确认进 `/sync-docs` |

### 4.5 `/sync-docs` — 更新文档

| 项 | 内容 |
|---|---|
| 调用 | `/sync-docs` |
| 输入 | regress 通过的完整改动 |
| 产出 | 更新 CLAUDE.md 进度表（新 P 行或「验证」列数字）+ P 文档状态标记（📋→✅）+ 必要时回写架构/决策文档 |
| gate | diff 检查：进度表指针/验证数字与当前代码一致；新增 command/路由/文件已在 CLAUDE.md 登记 |
| 完成标志 | CLAUDE.md + P 文档对齐代码现实 + 用户确认 |

### 4.6 `/cycle` — 全流程编排

| 项 | 内容 |
|---|---|
| 调用 | `/cycle <一句话需求>` |
| 输入 | 一句话需求 |
| 产出 | 串起 spec → plan → impl → regress → sync-docs，**每步 gate 通过且用户确认才进下一步** |
| 升格判断 | spec 阶段若判定「跨阶段/动锁定决策」，自动走完整新 P 流程；否则可走轻量 P.x |
| 完成标志 | 全部 5 步完成 |

### 4.7 `/fix` — 修复短路径

| 项 | 内容 |
|---|---|
| 调用 | `/fix <bug 描述>` |
| 输入 | bug 描述 + 复现路径 |
| 产出 | 代码修复 + 原 P 文档末尾「修复记录」段追加（`YYYY-MM-DD <commit> <一句话>`）+ CLAUDE.md「验证」列数字更新 |
| **pre-flight gate** | **动手前输出改动微方案**（目标 + 根因 + 影响范围 + 技术方案 + 风险标注 + 验证方式），用户确认才进修复 |
| gate | **必须复现**（写复现步骤）→ 改 → 跑该 P 已有测试 → 回写文档 |
| **升格准则** | 跨多个 P / 动锁定决策 / 改 bundle 契约 / 改 API → 停止 `/fix`，升格为新 P 走 `/cycle` |
| 回归范围 | 仅该 P 已有测试 + `docs/测试/Pxx-测试流程.md` 的复现步骤，不强制全量 |
| 完成标志 | 复现路径留档 + 该 P 测试通过 + 修复记录入库 |

---

## 5. 文档模板骨架

四份模板从现有样本提炼，放 `docs/模板/`。落地时逐一核对样本不漂移。

### 5.1 `决策.md`（样本：`docs/决策/i18n.md`、`npm-crates发布.md`）
```
# <topic> 决策
> <一句话上下文 + 指向对应 P 文档>
## 范围决策
## 技术路线决策
## 边界决策
## 未做（留后续）
```

### 5.2 `架构方案.md`（样本：`docs/架构/P9-*.md`、`官网文档站（方案）.md`）
```
# <topic>（最佳实践方案）
> <定位 + 指向 P 文档>
## 1. 总览（改动块 + 非目标）
## 2. 影响范围（新增/修改/不动 + 风险）
## 3. 设计决策（表）
## 4-N. 技术方案分章
## N. 与现有代码/文档的关系
## N+1. 分阶段实施
## N+2. 不做什么（scope 纪律）
## N+3. 风险与开放问题
```

### 5.3 `P文档.md`（样本：`docs/阶段路径/P13-*.md`）
```
# Pxx：<主题>
> <阶段路径第 N 阶段。目标/承接/方案指针。进度标记>
## 目标
## 范围（覆盖 / 不在本阶段）
## 设计决策（已拍）（表）
## 实施顺序（表，每行：阶段/内容/产出/状态）
## 关键陷阱（实施时发现）
## 修复记录（/fix 追加，YYYY-MM-DD <commit> <一句话>）
```

### 5.4 `测试流程.md`（样本：`docs/测试/P9-测试流程.md`、`P10-测试流程.md`）
```
# Pxx 测试流程
> <覆盖哪些验收线>
## 准备：构建 + 配置
## 场景 1-N（复现/验证步骤 + 预期）
## 回归（前序 P 不退化）
```

---

## 6. Hooks 配置

加到 `.claude/settings.json`，起步**全非阻断**（stdout 提醒，不 exit 2）。

| 事件 | matcher | 行为 | 阻断? |
|---|---|---|---|
| `PreToolUse` | `Bash` 命中 `git commit` | 提醒「是否已跑 `/regress` 或至少 `pnpm build`」 | 否（提醒） |
| `PostToolUse` | `Edit`/`Write` 命中 `src/**/*.vue` 且本会话未触发 `frontend-design` | 提醒「视觉变更建议走 `/frontend-design`」 | 否（提醒） |
| `Stop` | — | 提示「当前改动是否有未 sync 的 CLAUDE.md 进度表条目」 | 否（提醒） |

> **不设全局阻断 hook**。阻断型 gate（commit 前 type-check、cargo test）放在 `/regress` command 内显式执行，可见可绕（用户确认可跳）。

具体 JSON 片段（落地阶段 fine-tune 语法）：
```jsonc
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "Bash",
      "hooks": [{ "type": "command", "command": "<检测 git commit，stdout 提醒>" }]
    }],
    "PostToolUse": [{
      "matcher": "Edit|Write",
      "hooks": [{ "type": "command", "command": "<检测 .vue 且未触发 skill，提醒>" }]
    }],
    "Stop": [{
      "hooks": [{ "type": "command", "command": "<检查未 sync 进度表，提醒>" }]
    }]
  }
}
```

---

## 7. CLAUDE.md 改动

在「记忆与决策存放约定」段之后插入新段「开发流程 harness」，约 30 行，内容：

- 7 个命令入口一句话说明 + 指向本方案文档
- `/fix` vs `/cycle` 升格准则（一句话）
- subagent 使用边界（仅 spec/regress 两点）
- hooks 非阻断策略说明
- 指向 `docs/模板/` 的 4 份模板

**不改**现有「四条锁定决策」「记忆与决策存放约定」「视觉设计约定」等段。

---

## 8. 与现有文档/代码的关系

| 现有资产 | harness 关系 |
|---|---|
| CLAUDE.md「四条锁定决策」 | `/spec` gate 第 1 问检查是否触碰 |
| `docs/阶段路径/Pxx-*.md`（13 份） | `/plan` 用模板起草新 P；`/fix` 回写「修复记录」段 |
| `docs/架构/*（方案）.md`（7 份） | `/spec` 用模板起草；本方案是其中第 8 份 |
| `docs/决策/*.md`（2 份） | `/spec` 用模板起草 |
| `docs/测试/Pxx-测试流程.md`（P8/P9/P10，后断档） | `/regress` 用模板接续，P11 起补回 |
| `.claude/skills/frontend-design` | `/impl` dispatch，不改 |
| `.claude/skills/rrweb-recording` | `/impl` dispatch，不改 |
| CLAUDE.md 进度表 | `/sync-docs` 更新；`/fix` 只改「验证」列 |

---

## 9. 分阶段实施

| Phase | 内容 | 产出 | 依赖 |
|---|---|---|---|
| **A — 模板先行** | 4 份模板提炼落盘 | `docs/模板/*.md` | 无（从现有样本提炼） |
| **B — 文档类命令** | `/spec` `/plan` `/sync-docs` 三个 command | `.claude/commands/*.md` | Phase A（引用模板） |
| **C — 执行类命令** | `/impl` `/regress` `/fix` 三个 command | `.claude/commands/*.md` | Phase B（风格对齐） |
| **D — 编排 + hooks** | `/cycle` orchestrator + hooks 配置 + CLAUDE.md 改动 | settings.json + CLAUDE.md | Phase B/C |
| **E — 验证** | 用一个真实小任务（如补 P11 测试流程文档）跑一遍 `/fix` 或 `/cycle` 子集 | 验证报告 + command prompt 迭代 | Phase D |

Phase A→B 可一消息内并行；C 依赖 B 风格；D 收尾。

---

## 10. 不做什么（scope 纪律）

- **不引入测试框架**（Vitest 等）—— 独立缺口，建议 P14 单开
- **不引入 CI**—— `deploy-site.yml` 之外不加 workflow
- **不做 command 间的状态机**（命令间靠会话上下文 + 用户确认衔接，不存持久状态机）
- **不做多线并行 worktree 编排**—— 单线流够用；未来需要时配合 `EnterWorktree` 扩展
- **不做 command 版本化**—— prompt 迭代直接改 `.md`，git 历史即版本
- **不把 `/fix` 扩展成跨 P 追踪系统**—— 跨 P 自动升格 `/cycle`

---

## 11. 风险与开放问题

| # | 问题 | 当前倾向 | 待定 |
|---|---|---|---|
| O1 | command 间状态如何传递（spec 产出的决策文档如何被 plan 自动继承） | 靠会话上下文 + 文件路径（spec 落盘 → plan 读盘） | 验证阶段确认是否需要显式 state 文件 |
| O2 | hooks 的 shell 命令在 macOS 上的具体语法（matcher 匹配 git commit 的可靠性） | 落地时用最小 hook 验证 | Phase D 实测 |
| O3 | `/regress` 的 fresh subagent 是否真的能避免确认偏差（subagent 也可能复述主 agent 结论） | subagent prompt 强制「重新独立读代码 + 列契约」 | Phase E 验证 |
| O4 | 模板是否会过度僵化（P13 的「关键陷阱」段是实施时才出现的，模板不该强制） | 模板标「可选段」 | Phase A 样本核对时定 |
| O5 | `/cycle` 是否会因每步 gate 太重导致用户绕过 | gate 可经用户确认跳过（不硬阻断） | Phase E 验证使用体感 |

---

## 评审清单

请你逐项确认：

1. **影响范围（§2）**：12 新增 + 2 修改 + 零代码改动，是否遗漏？
2. **设计决策（§3）**：D1–D8 有无异议？特别是 D3（/fix 边界）、D5（subagent 仅两点）、D8（进度表不加行）。
3. **命令契约（§4）**：7 个命令的输入/产出/gate 是否符合预期？有无多余或缺失的 gate？
4. **hooks 策略（§6）**：全非阻断起步 + 阻断放 command 内，是否认可？
5. **实施顺序（§9）**：A→B→C→D→E 五 phase，是否要先做某一 phase 看效果再继续？
