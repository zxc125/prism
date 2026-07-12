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

## 架构

Tauri 2 桌面应用：Vue 3 + Vite 6 前端位于 [src/](src/)，Rust 后端位于 [src-tauri/](src-tauri/)。尽管仓库名为 `rrweb-demo`，**rrweb 尚未接入** - [src/views/PlayerView.vue](src/views/PlayerView.vue) 是占位页面（"在此接入 rrweb 播放逻辑"）。

### 多窗口系统（横跨前端 + 后端）

这是核心机制，需要结合多个文件一起理解：

1. **前端** 调用 `invoke("open_window", { route })` - 见 [src/views/MainView.vue](src/views/MainView.vue)。
2. **后端** [src-tauri/src/lib.rs](src-tauri/src/lib.rs) 的 `window_label()` 由路由推导窗口 label：`/settings` -> `settings`，`/player/abc` -> `player-abc`，`/` -> `main`。**相同 label = 单实例（聚焦已有窗口）；不同 label = 多实例。** 这就是设置窗口为单实例、播放器窗口支持多开的原因。
3. 新窗口加载 `index.html#{route}` - **hash 模式路由**（[src/router/index.ts](src/router/index.ts)）据此解析并渲染对应视图。此处必须用 hash 模式：Tauri 通过自定义协议 / 本地文件提供服务，`createWebHistory` 在刷新或深链时会 404。
4. **Capabilities** [src-tauri/capabilities/default.json](src-tauri/capabilities/default.json) 必须列出每个窗口 label 或 glob（`"player-*"`），否则窗口缺少权限。新增带新 label 模式的路由时需要同步修改此文件。

### 前端约定

- **已开启自动导入**（[vite.config.ts](vite.config.ts)）：`unplugin-auto-import` 全局提供 Vue API（`ref`、`reactive`、`computed`、`useRoute` 等）与 Element Plus API（`ElMessage`、`ElMessageBox`），**无需手动 import**。`unplugin-vue-components` 自动注册 Element Plus 组件（`el-button`、`el-card`…）。不要为这些添加手动导入 - 缺少 import 是有意为之，不是 bug。
- 类型声明生成到 [src/auto-imports.d.ts](src/auto-imports.d.ts) 与 [src/components.d.ts](src/components.d.ts)。这两个是构建产物 - 不要手动编辑；运行 `pnpm dev`/`pnpm build` 即可重新生成。
- UI 文案为中文（zh-CN）。新增用户可见文本时请保持一致的语言。

### 后端约定

- 应用入口为 [src-tauri/src/lib.rs](src-tauri/src/lib.rs) 的 `run()`；[src-tauri/src/main.rs](src-tauri/src/main.rs) 仅在 release 模式下屏蔽 Windows 控制台窗口并委托调用。新增 Tauri command 必须在 `lib.rs` 的 `generate_handler![]` 中注册。
- 每个路由的窗口标题/尺寸在 `open_window` 中按路由首段匹配决定 - 新增需要自定义尺寸的路由时，扩展该 match 分支。

### 环境说明

- [src-tauri/.cargo/config.toml](src-tauri/.cargo/config.toml) 将 crates.io 重定向到 **rsproxy.cn** 镜像（国内）。若 Rust 依赖拉取失败，问题/解法均在此镜像配置 - 无充分理由不要移除。
- Vite 固定使用端口 1420 且 `strictPort: true`（Tauri 依赖此端口）；`src-tauri/**` 已被排除出 Vite 文件监听。
