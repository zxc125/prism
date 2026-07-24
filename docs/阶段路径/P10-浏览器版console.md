# P10：console 2.0 重设计 + 浏览器化

> 阶段路径第 10 阶段（✅ 已落地 2026-07-25）。目标：重设计分析端平台 + 浏览器零安装访问。详细可落地方案见 [P10-console2.0重设计（方案）.md](../架构/P10-console2.0重设计（方案）.md)。

## 目标

P1-P9 把功能堆满了，但 console 形态仍停在 P1 的「源机架 + 会话浏览器」单页骨架。P10 借浏览器化契机做一次 **console 2.0 重设计**：

1. **重定信息架构**：app shell + 持久侧栏 + 顶栏上下文，支撑多租户等新概念
2. **刷新视觉语言**：转向**现代科技感**（现代 dev tool 质感，琥珀保留为信号主色）
3. **浏览器化**：Tauri 抽象 + observer-server 托管 + 登录墙，同一份构建两端可跑

不是「把 console 搬进浏览器」，而是让 P1-P9 的功能积累以更现代、更可扩展、更易访问的形态交付。

## 现状诊断（设计债务）

| 债务 | 现状 | 现代产品做法 |
|---|---|---|
| 视图巨石 | MainView 921 行、PlayerView 934 行、SettingsView 274 行 | 组件化拆分，单一职责 |
| 无导航骨架 | 仅 3 路由，左侧 rail 是源机架非导航 | app shell：侧栏导航 + 顶栏上下文 |
| 设置是垃圾场 | 6 个 form 分组平铺一页 | 分 tab / 情境化 |
| 多租户不可见 | P9 tenant/quota/redact 服务端，console 无感 | 顶栏 tenant 上下文 + 配额可见 + tenant 管理入口 |
| player 独立窗口 | Tauri 新开 webview | in-app 全屏路由 + 面包屑（两端通用） |
| 空/错/加载态凑合 | 空状态一行字 | 设计过的空状态 + CTA + skeleton |
| 视觉偏复古 | "cutting room" 暖暗 + 装饰性 mono，偏复古广播台 | 现代 dev tool：中性偏冷暗 + 克制用色 + 留白 |
| 无 cmd+k | - | 命令面板已是标配 |

## 范围

1. **信息架构重设计**：app shell（侧栏 + 顶栏）+ 路由重组。MainView 拆为 Sessions（浏览）+ Live（观测控制）；Player 从独立窗口改 in-app 路由 `/s/:id`；新增 Tenants 路由。
2. **视觉语言刷新**：转向现代科技感--基底中性偏冷暗、琥珀保留为信号主色、引入冷青辅色、强化等宽字体科技嗓音、glow/细线取代厚边框。**视觉执行走 `/frontend-design` 流程**。
3. **组件化拆分**：三个巨石 SFC 拆成 `src/components/{shell,sessions,live,player,tenants,settings,common}/` 组件系统，每组件 < 200 行。
4. **多租户产品化**：`GET /whoami` 端点 + 顶栏 tenant 上下文 + `/tenants` 路由，P9 服务端能力首次在 console 可见。
5. **浏览器化**：`isTauri()` 运行时检测 + dispatch（窗口/文件/录制）、observer-server 内嵌静态托管、LoginGate 登录墙。
6. **产品打磨**：空状态 + skeleton + cmd+k + 响应式。

## 设计决策（已拍）

| # | 决策点 | 选择 | 理由 |
|---|---|---|---|
| D1 | P10 范围 | console 2.0 重设计 + 浏览器化（不只浏览器） | 功能已堆满，单纯搬浏览器没解决结构债 |
| D2 | 视觉方向 | **现代科技感**（中性偏冷暗 + 琥珀信号主色 + 冷青辅色 + glow/细线） | 用户决策；告别复古暖褐，转向现代 dev tool 质感 |
| D3 | player 形态 | in-app 全屏路由 `/s/:id` + 面包屑 | 两端统一；浏览器无新标签 workaround |
| D4 | 导航模型 | 持久侧栏 + 顶栏 | 可扩展（tenants/admin 未来入口）；现代 dev tool 标配 |
| D5 | tenant 上下文位置 | 顶栏左（最显眼） | 多租户是一等公民，不藏在指示器小点里 |
| D6 | 自录在浏览器 | 隐藏 Live 的本机通道 | 浏览器无法自录；保留 web/tauri 通道观测 |
| D7 | 登录模型 | endpoint + key -> /whoami 校验 | 复用多租户 key，无 OAuth |
| D8 | web 托管 | observer-server 内嵌静态服务 | 单二进制零安装 |
| D9 | 拆分节奏 | 渐进式（边拆边跑） | 避免大爆炸；每阶段可回归 P8/P9 测试流程 |

## 实施顺序

| 阶段 | 内容 | 产出 | 依赖 |
|---|---|---|---|
| 0 | 设计语言刷新（token + 基础组件） | 新 theme.css + common/ 组件库 | `/frontend-design` 驱动 |
| 1 | app shell + 导航 + Sessions 视图拆分 | AppShell + /sessions 路由 | 0 |
| 2 | Live 视图 + 录制控制拆分 | /live 路由，源机架独立 | 1 |
| 3 | Player in-app 路由化 + 拆分 | /s/:id 路由，PlayerView 组件化，面包屑 | 1 |
| 4 | 多租户产品化 | /whoami + 顶栏 tenant 上下文 + /tenants 路由 | 1 |
| 5 | 设置分 tab + 情境化 | /settings 分 tab，Tauri 专属 tab 守卫 | 1 |
| 6 | 浏览器化 | Tauri 抽象层 + observer-server 静态托管 + LoginGate | 1-5 |
| 7 | 产品打磨 | 空状态 + skeleton + cmd+k + 响应式 | 6 |

每阶段独立可交付、可回归。阶段 0 是视觉地基（必须 /frontend-design 先行），1-3 结构重组，4 多租户闭环，5-6 设置 + 浏览器，7 打磨。

## 改动

- **前端**：新增 `src/components/{shell,sessions,live,player,tenants,settings,common}/`；新增 `src/composables/tauri.ts`（isTauri + dispatch）；新增 `src/views/LoginGate.vue`；[router/index.ts](src/router/index.ts) 路由重组；[theme.css](src/styles/theme.css) 视觉 token 刷新；拆解 MainView/PlayerView/SettingsView。
- **后端**：[observer-server](crates/observer-server) 加 `GET /whoami` + 静态文件服务（`GET /` + `/assets/*`）；[bin/observer_server.rs](crates/observer-server/src/bin/observer_server.rs) 加 `--web-dir` 参数。
- **构建**：`pnpm build` 产物拷到 `crates/observer-server/web/`。

## 验收

- app shell + 侧栏导航 + 顶栏 tenant 上下文可用。
- Sessions / Live / Player / Tenants / Settings 五大视图组件化、各自 < 200 行。
- Player 走 in-app 路由 `/s/:id`，面包屑返回，两端一致。
- 多租户：顶栏显示 `acme` + 配额余量；`/tenants` 路由只读详情。
- 浏览器：`observer-server --web-dir` 启动，浏览器开 `http://host:port` -> 登录 -> 全功能（回放/诊断/标注/导出）。
- 视觉：现代科技感落地，告别复古暖褐；琥珀作信号主色、冷青辅色、glow/细线。
- 回归：P8/P9 测试流程全过；Tauri 桌面行为不变。

## 风险与边界

- **视觉身份**：D2 转现代科技感是较大方向调整，须 `/frontend-design` 流程保意图落地，避免退化回 Element Plus 默认或通用暗色模板。
- **拆分回归**：900+ 行巨石 SFC 拆分有回归风险，渐进式 + 每阶段跑 P8/P9 测试流程。
- **player 路由化心智变化**：从多窗口变 in-app，面包屑 + 返回快捷键补偿；保留「新标签打开」可选。
- **浏览器 localStorage key XSS**：自托管私有云威胁模型可控；未来可换 httpOnly cookie + session。
- **admin UI 蔓延**：`/tenants` 起步只读；增删改 key/tenant 留 P10.5 或 P11。
- **工作量**：8 阶段周期长，阶段 1-3 即可见结构改善，不必等全部完成。

> 备注：P8 的 Tauri-as-cloud-client 已覆盖私有云场景（装一次 desktop client 连云端）。P10 进一步到零安装浏览器访问，同时完成 console 产品化重设计。详见 [P10-console2.0重设计（方案）.md](../架构/P10-console2.0重设计（方案）.md)。
