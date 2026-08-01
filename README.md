<p align="center">
  <img src="site/public/logo.svg" width="120" alt="鉴 / Prism" />
</p>

<h1 align="center">鉴 / Prism</h1>

<p align="center">
  本地优先的前端观测平台。会话回放、诊断信号、多窗口对齐--数据留在你手里，不上云，不锁仓。
</p>

<p align="center">
  <em>Local-first frontend observation: session replay, interleaved diagnostic signals, multi-window alignment. Your data stays on your machine.</em>
</p>

---

## 这是什么

鉴（Jiàn）/ Prism 是一个**本地优先的前端观测平台**：基于 rrweb 2 的会话回放 + 诊断信号 + 多窗口对齐，闭环到「回放 + 诊断 + 导出/标注/分享」。

不是云 RUM，不是 APM，不做告警--只做诊断这一件事。数据默认留在本地，可选自托管私有云。没有 per-session 计费，没有厂商锁定，你的会话就是一份明文 JSON。

## 为什么

- **本地优先** -- 默认零云依赖，数据不经任何第三方服务器
- **多窗口对齐录制** -- Tauri 多窗口共享墙上时钟，回放多轨同步（独家）
- **交错事件模型** -- error / console / network 作为 `type:6` 交错进 DOM 事件流，共享同一条时间轴
- **诊断导向** -- 不做告警 / RUM 指标，专注复现 -> 看懂 -> 标注 -> 分享
- **单二进制自托管** -- `observer-server` 一个文件托管 API + 前端
- **开放 bundle 契约** -- `prism-session` 明文 JSON，三路共用，随时带走

## 快速开始

**嵌入你的应用**（Web SDK）：

```sh
pnpm add @prism-obs/observer-sdk
```

```ts
import { recordOffline } from "@prism-obs/observer-sdk";
recordOffline({ endpoint: "http://localhost:8080/ingest" });
```

**自托管 server**：

```sh
observer-server \
  --bind 0.0.0.0:8080 \
  --web-dir ./console \
  --tenants tenants.json
```

## 架构

```
┌─ Console / Player UI        Vue 3 · 多轨时间轴 · 诊断信号流
├─ bundle 契约                 prism-session（明文 JSON，跨进程迁移）
├─ Rust 核心                   observer-storage / observer-server
└─ rrweb 2 · 彽制基座           DOM 快照 + 增量 · type:6 交错诊断信号
```

## 仓库结构

```
prism/
├─ src/                      # console 应用（Vue 3 + Element Plus）
├─ src-tauri/                # Tauri 2 桌面壳
├─ crates/
│  ├─ observer-storage/      # 纯存储层（落盘 + bundle 契约 + redact）
│  └─ observer-server/       # HTTP server（ingest + 读 API + 多租户 + 静态托管）
├─ plugins/
│  └─ tauri-plugin-observer/ # 录制协调插件（Local + Remote 双模式）
├─ packages/
│  ├─ observer-sdk/          # @prism-obs/observer-sdk（Web SDK，HttpSink + IndexedDBSink）
│  └─ observer-tauri/        # Tauri App 接入驱动
├─ examples/
│  ├─ web-demo/              # Web SDK 样例
│  └─ tauri-demo/            # Tauri Plugin 样例
├─ site/                     # 官网（营销站，Vue 3 + Vite + Tailwind v4）
└─ docs/                     # 架构 / 阶段路径 / 品牌
```

## 开发

包管理器为 **pnpm**。

| 任务 | 命令 |
| --- | --- |
| console 前端 dev（端口 1420） | `pnpm dev` |
| 完整 Tauri 桌面应用 | `pnpm tauri dev` |
| 构建桌面安装包 | `pnpm tauri build` |
| 官网 dev（端口 4321） | `pnpm dev:site` |
| 官网构建 | `pnpm build:site` |
| Rust 检查 / 测试 | `cargo check` / `cargo test`（在 `src-tauri/` 内） |

架构与阶段规划见 [docs/](docs/)，命令速查与约定见 [CLAUDE.md](CLAUDE.md)。

## 阶段路径

P1-P10 已完成 console 2.0 + 浏览器化 + 多租户；P11 官网与品牌进行中。详见 [docs/阶段路径/](docs/阶段路径/)。

## 许可证

MIT License -- 见 [LICENSE](LICENSE)。

`site/src/components/Aurora.vue` 来自 [DavidHDev/vue-bits](https://github.com/DavidHDev/vue-bits)（MIT + Commons Clause），保留原始版权声明，详见 [site/THIRD_PARTY_NOTICES.md](site/THIRD_PARTY_NOTICES.md)。本项目建在 [rrweb](https://github.com/rrweb-io/rrweb) 与 [Tauri](https://tauri.app) 之上。
