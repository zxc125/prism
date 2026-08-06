# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 命令

包管理器为 **pnpm**（由 [src-tauri/tauri.conf.json](src-tauri/tauri.conf.json) 中的 `beforeDevCommand`/`beforeBuildCommand` 使用）。

| 任务 | 命令 |
| --- | --- |
| 仅启动前端 dev 服务器（端口 1420） | `pnpm dev` |
| 类型检查 + 构建前端到 `dist/` | `pnpm build` |
| 以开发模式运行完整 Tauri 桌面应用 | `pnpm tauri dev`（自动执行 `pnpm dev`） |
| 构建可安装的桌面安装包 | `pnpm tauri build`（自动执行 `pnpm build`） |
| Rust 检查/测试（在 `src-tauri/` 内执行） | `cargo check` / `cargo test` |

`pnpm build` 会先执行 `vue-tsc --noEmit` - 类型错误会导致构建失败。项目未配置 JS/TS 测试框架；仅有的测试是 Rust 的 `cargo test`（目前没有任何测试）。

## 平台演进方向

本项目正从「自录 + 回放工具」演进为**本地优先的前端观测平台**（支持 web / tauri 外部观测，闭环到「回放 + 诊断 + 导出/标注/分享」）。架构与阶段规划见 [docs/架构/](docs/架构/) 与 [docs/阶段路径/](docs/阶段路径/)。

四条锁定决策：
1. **本地优先** - Tauri App 当分析台（console），零云依赖，不建云后端 RUM。自托管私有云为 opt-in 拓扑（用户 own 数据，非 SaaS RUM），见 [P8](docs/阶段路径/P8-云端server抽取.md)。
2. **观测外部** - Web SDK（npm，单 tab）+ `tauri-plugin-observer`（多窗口对齐），统一走 HTTP sink -> console 本地 server。
3. **闭环到回放+诊断** - 不做告警/生产 RUM；诊断 = 回放带 error/console/network 上下文 + 导出/标注/分享。
4. **交错事件模型** - error/console/network 以 rrweb plugin 事件（`type:6`）交错进同一条事件流，与 DOM 共享时间轴。

阶段路径：P1 分析端页面改造 -> P2 诊断信号采集 -> P3 sink 抽象 -> P4 Web SDK -> P5 Tauri Plugin -> P6 导出/标注/分享 -> P7 离线采集与 bundle 契约 -> P8 云端 server 抽取 -> P9 多租户与运营加固 -> P10 console 2.0 重设计 + 浏览器化 -> P11 官网与品牌 -> P12 官网 i18n -> P13 官网文档站（VitePress，📋 计划）。各阶段实现细节见 [docs/阶段路径/](docs/阶段路径/)，架构方案见 [docs/架构/](docs/架构/)，品牌见 [docs/品牌/](docs/品牌/)。

**进度**：P1-P12 全部 ✅；P13 官网文档站 Phase 1 ✅（[方案](docs/架构/官网文档站（方案）.md) · [P13](docs/阶段路径/P13-官网文档站.md)）。下表只列「是什么 + 1-2 个核心指针 + 验证状态」，实现细节归各阶段文档（零信息丢失，已逐一核对指针落在对应 P 文档内）。

| 阶段 | 主题 | 关键产出指针 | 验证 |
| --- | --- | --- | --- |
| P1 | 分析端页面改造 | 源监控机架 + 会话浏览器 + 诊断信号流 | — |
| P2 | 诊断信号采集 | error/console/network hook，`type:6` 交错进事件流 | — |
| P3 | sink 抽象 | [sink.ts](src/composables/sink.ts) `Sink` 接口 + `TauriSink` | — |
| P4 | Web SDK | [ingest.rs](src-tauri/src/ingest.rs) + [packages/observer-sdk](packages/observer-sdk)（`@prism-obs/observer-sdk` · ✅ [npm](https://www.npmjs.com/package/@prism-obs/observer-sdk) 0.1.0） | — |
| P5 | Tauri Plugin | [plugins/tauri-plugin-observer](plugins/tauri-plugin-observer) Local/Remote 双模式 · ✅ [crates.io](https://crates.io/crates/tauri-plugin-observer) + [npm](https://www.npmjs.com/package/@prism-obs/observer-tauri) `@prism-obs/observer-tauri` 0.1.0 | — |
| P6 | 导出/标注/分享 | session 级 `annotations.jsonl` + 单文件 bundle（`format: prism-session`） | — |
| P7 | 离线采集 + bundle 契约 | [bundle.ts](packages/observer-sdk/src/bundle.ts) + [bundle-规范.md](docs/架构/bundle-规范.md) + 路径穿越防护 | `cargo test` 9 |
| P8 | 云端 server 抽取 | [crates/observer-storage](crates/observer-storage) + [crates/observer-server](crates/observer-server) + [backend.ts](src/composables/backend.ts) | `cargo test` 13 |
| P9 | 多租户运营加固 | per-tenant 配额/保留 + gzip + 服务端 redact + 限流 | `cargo test` 45 |
| P10 | console 2.0 + 浏览器化 | App shell + 视觉刷新 + [tauri.ts](src/composables/tauri.ts) 抽象 + `--web-dir` 静态托管 + `/whoami` | `cargo test` 52 |
| P11 | 官网与品牌 | [site/](site/) 12 节 + 菱形折射 logo + GH Pages 上线 + 全库改名 `prism` | `cargo test` 53 + site build |
| P12 | 官网 i18n | [useLang](site/src/composables/useLang.ts) + zh-CN/en 字典 + 13 组件接 `t()` | `vue-tsc` 0 + site build 184.7KB |
| P13 | 官网文档站 | VitePress 接管 `site/` + 5 篇手册（web/tauri/deploy）zh/en · [方案](docs/架构/官网文档站（方案）.md) | Phase 1 ✅ `vitepress build` 通过 |

## 架构

Tauri 2 桌面应用：Vue 3 + Vite 6 前端位于 [src/](src/)，Rust 后端位于 [src-tauri/](src-tauri/)。基于 rrweb 2 实现**多窗口录制与回放**。

### 录制 / 回放系统（横跨前端 + 后端）

rrweb 在前端 webview 里跑，每个窗口各一个 `record()` 实例；事件按 segment 流式落盘到 `appDataDir/recordings/<sessionId>/`：

```
recordings/<sessionId>/
  session.json          # { id, startedAt, endedAt?, source?, name?, note?, tags?, importedAt? }
  windows.jsonl         # 窗口生命周期: shown/hidden/focus，带 segmentId
  segments/<label>#<n>.jsonl   # 每段 rrweb 事件流（一次 show ~ hide = 一段）
  annotations.jsonl     # 用户标注（P6，session 级，与事件流分离）
```

关键机制（需结合多文件理解）：

- **会话与段**：`start_session` 置 active 并广播 `recording-session` 事件；各窗口的 `useRecorder`（[src/composables/useRecorder.ts](src/composables/useRecorder.ts)，由 [App.vue](src/App.vue) 挂载）收到后 `invoke("plugin:observer|begin_segment")` 分配 segmentId `<label>#<n>` 并启动 rrweb。`player-*` 窗口跳过录制；P10 起 in-app player 路由 `/s/:id` 也跳过（hash 路由守卫 `isPlayerRoute()` + `hashchange` 监听，进入 player 暂停当前段、离开若会话仍活跃则开新段）。
- **录制协调已抽成插件**：`start_session`/`stop_session`/`is_recording_active`/`begin_segment`/`append_events` 等录制命令与 `on_window_event` 生命周期拦截已搬进独立 crate [`plugins/tauri-plugin-observer`](plugins/tauri-plugin-observer)（`tauri-plugin-observer`）。两种模式：**Local**（console self-obs，Rust 直接落盘到 `appDataDir/recordings/`）与 **Remote**（外部 Tauri 应用，Rust 只管窗口协调 + 事件驱动，前端 `HttpSink` 上报到 console）。console 装插件 Local 模式（`skip_focus_prefix: "player-"`），前端 `TauriSink` 调 `plugin:observer|*` 命令；`list_sessions`/`read_session`/`delete_session`/`open_window`/ingest 仍留 [src-tauri/src/lib.rs](src-tauri/src/lib.rs)。插件命令需在 capabilities 授权 `observer:default`（[default.json](src-tauri/capabilities/default.json)）；console 自定义 command（`open_window` 等）仍无需授权。
- **子窗口关闭=隐藏**：录制期间，子窗口的 `CloseRequested` 被拦截为 `hide()` + 记 `hidden` + `emit_to` segment:stop；再次 `open_window` 复用已隐藏窗口时 `show()` + 插件 `emit_segment_start_if_active` 开新段。主窗口关闭不拦截 = 退出进程。见插件 [lifecycle.rs](plugins/tauri-plugin-observer/src/lifecycle.rs)（Tauri 2 plugin Builder 无 `on_window_event`，用 `on_window_ready` 给每窗口挂 `Window::on_window_event`）。
- **跨窗口对齐**：rrweb 事件带绝对 `timestamp`，所有窗口共享墙上时钟，回放时按 `shown.t ~ hidden.t` 区间在主时间轴上同步驱动各 segment 的 `Replayer`。见 [src/composables/usePlayer.ts](src/composables/usePlayer.ts)。
- **回放**：[PlayerShell](src/components/player/PlayerShell.vue)（路由 `/s/:id`，P10 起 in-app + 面包屑，不再开独立窗口）用底层 `Replayer`（rrweb-player 是 Vue 2，不能用）+ 自建 Element Plus 控制条，平铺显示当前活跃窗口。子组件 [ReplayGrid](src/components/player/ReplayGrid.vue)/[Timeline](src/components/player/Timeline.vue)/[DiagnosisPanel](src/components/player/DiagnosisPanel.vue) 通过 `provide/inject`（`PLAYER_CTX`）共享 usePlayer 实例。
- **bundle 契约与离线采集**（P7）：会话可序列化为 `prism-session` bundle（`{ format, version, session, windows, segments, annotations }`），是跨进程/跨机迁移唯一契约（本地文件 / 本地 server / 云端上传三路共用），规范见 [bundle-规范.md](docs/架构/bundle-规范.md)。SDK `IndexedDBSink` + `recordOffline()` 支持脱离 console 离线录 -> 导出 bundle；`import_session`/`import_session_path` 校验 segmentId（`^[A-Za-z0-9_-]+#[0-9]+$`）防路径穿越 + 原子写（先 `.tmp` 再 rename）。
- **存储层与 server 抽取 + Backend 抽象**（P8）：纯存储逻辑（落盘 + annotations + bundle 契约 + 读/列举/导入导出）抽到独立 crate [`crates/observer-storage`](crates/observer-storage)（零 tauri 依赖，全吃 `&Path`），[`tauri-plugin-observer::storage`](plugins/tauri-plugin-observer/src/storage.rs) 只留 `recordings_root` + re-export。HTTP server（ingest `/ingest/*` + 读 API `/sessions/*` + `POST /sessions/import`）抽到 [`crates/observer-server`](crates/observer-server)（`ObserverServer` + 独立二进制 `observer-server`，可绑 `0.0.0.0`）；console [ingest.rs](src-tauri/src/ingest.rs) 退化为绑 127.0.0.1 的薄封装，同一份代码。console 前端加 `Backend` 抽象（[backend.ts](src/composables/backend.ts)：`TauriBackend`=invoke / `HttpBackend`=HTTP），设置页切换，默认 Tauri；**录制 Sink 与 Backend 正交**。单租户 Bearer token 鉴权，多租户留 P9。P10 起：`GET /whoami` 暴露 tenant 上下文 + 配额余量（`QuotaTracker.usage` 读 AtomicU64）；`ServerConfig.web_dir` 启用后未命中 API 的请求 fallback 到静态文件（SPA 模式 + 路径穿越防护 + MIME 分派），`observer-server --web-dir <dir>` 单二进制零安装托管 console；浏览器模式 `getBackend()` 强制 HttpBackend + `UnconfiguredBackend` 占位（LoginGate 拦截）。
- 新增的 Tauri command：console 自有命令（`greet`/`open_window`/`list_sessions`/`read_session`/`delete_session`/`list_annotations`/`save_annotations`/`update_session_meta`/`export_session`/`import_session`/`import_session_path`/`read_text_file`/`get_ingest_config`/`set_ingest_config`）注册在 [src-tauri/src/lib.rs](src-tauri/src/lib.rs) `generate_handler![]`，均为 observer-storage 薄封装，无需 capabilities 授权；插件命令（`plugin:observer|*`）注册在插件 `Builder::invoke_handler`，需 `observer:default` 权限。

### 多窗口系统（横跨前端 + 后端）

这是核心机制，需要结合多个文件一起理解：

1. **前端** 调用 `invoke("open_window", { route })` - 见 [src-tauri/src/lib.rs](src-tauri/src/lib.rs)。P10 起多数导航走 in-app 路由（`router.push`），`open_window` 仅用于「新窗口打开」场景。
2. **后端** [src-tauri/src/lib.rs](src-tauri/src/lib.rs) 的 `window_label()` 由路由推导窗口 label：`/settings` -> `settings`，`/s/abc` -> `s-abc`，`/` -> `main`。**相同 label = 单实例（聚焦已有窗口）；不同 label = 多实例。**
3. 新窗口加载 `index.html#{route}` - **hash 模式路由**（[src/router/index.ts](src/router/index.ts)）据此解析并渲染对应视图。此处必须用 hash 模式：Tauri 通过自定义协议 / 本地文件提供服务，`createWebHistory` 在刷新或深链时会 404。P10 浏览器化同样依赖 hash 模式（observer-server 静态托管 SPA fallback）。
4. **Capabilities** [src-tauri/capabilities/default.json](src-tauri/capabilities/default.json) 必须列出每个窗口 label 或 glob（`"player-*"`、`"s-*"`、`"tenants-*"` 等），否则窗口缺少权限。新增带新 label 模式的路由时需要同步修改此文件。

### 前端约定

- **已开启自动导入**（[vite.config.ts](vite.config.ts)）：`unplugin-auto-import` 全局提供 Vue API（`ref`、`reactive`、`computed`、`useRoute` 等）与 Element Plus API（`ElMessage`、`ElMessageBox`），**无需手动 import**。`unplugin-vue-components` 自动注册 Element Plus 组件（`el-button`、`el-card`…）。不要为这些添加手动导入 - 缺少 import 是有意为之，不是 bug。
- 类型声明生成到 [src/auto-imports.d.ts](src/auto-imports.d.ts) 与 [src/components.d.ts](src/components.d.ts)。这两个是构建产物 - 不要手动编辑；运行 `pnpm dev`/`pnpm build` 即可重新生成。
- UI 文案为中文（zh-CN）。新增用户可见文本时请保持一致的语言。
- **P10 组件结构**（见 [docs/架构/P10-console2.0重设计（方案）.md](docs/架构/P10-console2.0重设计（方案）.md) §5）：`src/components/{shell,sessions,live,player,tenants,settings,common}/`，每视图 < 200 行、单一职责。视图（`src/views/`）薄包装 AppShell + 组件；composable（usePlayer/useAnnotations/useRecorder）不动，组件消费。Player 子组件通过 `provide/inject`（[context.ts](src/components/player/context.ts) 的 `PLAYER_CTX`）共享 usePlayer 实例。
- **Tauri 抽象**（P10）：对 Tauri API 的依赖收敛到 [src/composables/tauri.ts](src/composables/tauri.ts)（`isTauri()` + `openRoute`/`pickBundleFile`/`onWindowFocus`/`currentWindowLabel` dispatch），`@tauri-apps/api` 动态 import。新代码不要直接 `import { invoke } from "@tauri-apps/api/core"`，走 tauri.ts 抽象。

### 记忆与决策存放约定

- **项目事实/决策/状态**（i18n 决策、命名决策、阶段产出）进仓库，不进本地 memory：
  - 高频必知 + 锁定决策 → 本文件「四条锁定决策」段或上方进度表。
  - 阶段实现细节 → `docs/阶段路径/P*n.md`；架构方案 → `docs/架构/`；品牌 → `docs/品牌/`；决策日志 → `docs/决策/`。
- **个人偏好/机器相关/未定草稿** → 本地 `~/.claude/.../memory/` 或 `.claude.local.md`（均 gitignored，不跨机）。
- 遇到决策类问题，先查本文件「四条锁定决策」+ `docs/决策/`，再动手。

### 开发流程 harness

本项目以「P 阶段」为开发单位（P1–P13 已落地，见进度表）。开发流已工具化为 7 个 slash commands + 4 份模板 + 2 个非阻断 hooks，完整方案见 [开发流harness（方案）](docs/架构/开发流harness（方案）.md)。

**7 个命令入口**：

| 命令 | 阶段 | 何时用 |
|---|---|---|
| `/cycle <需求>` | 全流程 | 新需求，串起 spec→plan→impl→regress→sync-docs 五步，每步 gate 通过才进下一步 |
| `/spec <需求>` | 评审 | 单独做需求评审（产出决策 + 方案文档） |
| `/plan` | 拆阶段 | 单独起草 P 文档 + TodoWrite |
| `/impl` | 开发 | 按 plan 实施（dispatch `frontend-design` / `rrweb-recording` skill） |
| `/regress` | 回归 | spawn fresh subagent 独立验证 + 产出测试流程文档 |
| `/sync-docs` | 同步 | 更新 CLAUDE.md 进度表 + P 文档状态 |
| `/fix <bug>` | 修复 | **既有产出的局部 bug 修复**（回写原 P 文档修复记录，不重走 cycle） |

**`/fix` vs `/cycle` 升格准则**：单 P 内局部修 = `/fix`；跨阶段 / 动锁定决策 / 改 bundle 契约 / 改对外 API = 升格新 P 走 `/cycle`。

**subagent 边界**：仅 `/spec`（spawn `Explore` 全文档库对照）+ `/regress`（spawn `general-purpose` fresh context 独立复核）两点用 subagent；其余主 agent 执行。

**文档模板**：起草新 P 文档 / 决策 / 架构方案 / 测试流程时，用 `docs/模板/` 对应模板。`/fix` 修复记录追加到原 P 文档的 `## 修复记录` 段，格式 `YYYY-MM-DD <commit> <一句话>`。

**hooks**（全非阻断，见 [settings.json](.claude/settings.json)）：`git commit` 前提醒跑回归；会话结束时若有未提交改动提醒 `/sync-docs`。阻断型 gate 在各 command 内显式执行，不挂全局 hook。

### 视觉设计约定

任何涉及视觉/样式的变更（新增视图、改配色、改排版、重做交互形态）**必须先调用 `/frontend-design` skill**，按其流程（brainstorm -> critique -> build -> critique again）做出有意图的设计选择，不要直接套用 Element Plus 默认样式或通用暗色模板。

现有设计语言（P10 「现代科技感」）已落地，变更需与之保持一致：

- **设计 token** 集中在 [src/styles/theme.css](src/styles/theme.css)：**中性偏冷暗底**（`--ink: #0A0C10` / `--slate` 深空感，替代旧暖褐）、**偏冷白文字** `--bone`、**琥珀信号主色** `--amber`（收敛为信号/激活/实时数据流，带 `--amber-glow` 发光）、**冷青数据辅色** `--teal`（网络/信息流，与琥珀暖冷对比）、**牛血红** `--oxblood` 用于 REC/危险。等宽字体严格限定时间码/ID/技术值。不要退回暖褐胶片底、中性纯黑或 acid-green/vermilion 等通用暗色模板配色。
- **Element Plus 深色化** 通过在 `:root:root` 覆盖 `--el-*` 变量实现（双 specificity 压过 EP 内置 `:root`）；新组件沿用此方式，避免 `el-card shadow="hover"` 之类默认包装，视图为平铺布局。
- **轨道色** 集中在 [src/composables/usePlayer.ts](src/composables/usePlayer.ts) 的 `LANE_COLORS`，新增窗口轨道色在此扩展。**来源色**：本机=`--amber`、web=`--teal`、tauri=`--lane-5`。
- **签名元素**：Sessions = 会话卡列表 + 来源色点；Live = 源机架（多通道控制室输入 bay，本机通道承袭 REC 脉冲 DNA）；Player = 多轨时间轴 + 真实播放头 + 诊断信号流；TopBar = tenant 上下文 + 配额余量条。新视图应贡献自己的一个签名元素，而非复刻通用模板。

### 后端约定

- 应用入口为 [src-tauri/src/lib.rs](src-tauri/src/lib.rs) 的 `run()`；[src-tauri/src/main.rs](src-tauri/src/main.rs) 仅在 release 模式下屏蔽 Windows 控制台窗口并委托调用。新增 Tauri command 必须在 `lib.rs` 的 `generate_handler![]` 中注册。
- 每个路由的窗口标题/尺寸在 `open_window` 中按路由首段匹配决定 - 新增需要自定义尺寸的路由时，扩展该 match 分支。

### 环境说明

- [src-tauri/.cargo/config.toml](src-tauri/.cargo/config.toml) 将 crates.io 重定向到 **rsproxy.cn** 镜像（国内）。若 Rust 依赖拉取失败，问题/解法均在此镜像配置 - 无充分理由不要移除。
- Vite 固定使用端口 1420 且 `strictPort: true`（Tauri 依赖此端口）；`src-tauri/**` 已被排除出 Vite 文件监听。
