# P10 console 2.0 重设计（最佳实践方案）

本文档是 [P10-浏览器版console](../阶段路径/P10-浏览器版console.md) 的可落地方案：给出信息架构、视觉方向、组件拆分、接口契约与分阶段实施，使开发者能据此开工。视觉像素级执行仍走 `/frontend-design` 流程，本文只定方向与结构。

## 1. 总览

P10 = **console 2.0 重设计 + 浏览器化**。三件事合一：

1. 重定信息架构（app shell + 导航 + 路由重组）
2. 刷新视觉语言（现代科技感）
3. 浏览器化（Tauri 抽象 + observer-server 托管 + 登录墙）

不是新功能堆叠，而是 P1-P9 功能积累的产品化收口。非目标：admin API/UI、OAuth/SSO、移动端原生、回放外部资源完全保真。

## 2. 现状盘点

### 2.1 Tauri 依赖面（决定浏览器化工作量）

| 模块 | Tauri 依赖 | 浏览器对策 |
|---|---|---|
| [usePlayer.ts](../../src/composables/usePlayer.ts) / [PlayerView.vue](../../src/views/PlayerView.vue) | **无** | 直接可用 |
| [useAnnotations.ts](../../src/composables/useAnnotations.ts) | 无（走 Backend） | 直接可用 |
| [backend.ts](../../src/composables/backend.ts) HttpBackend | 无（纯 fetch） | 直接可用 |
| router（hash 模式） | 无 | 直接可用 |
| [useRecorder.ts](../../src/composables/useRecorder.ts) / [sink.ts](../../src/composables/sink.ts) | event listen + webviewWindow + `invoke("plugin:observer|*")` | `isTauri()` 守卫，浏览器不挂载 |
| [App.vue](../../src/App.vue) | 挂载 useRecorder | 条件挂载 |
| [MainView.vue](../../src/views/MainView.vue) 窗口管理 | `invoke("open_window")` + `onFocusChanged` | dispatch: Tauri=invoke / 浏览器=window.open |
| [MainView.vue](../../src/views/MainView.vue) 文件导入 | `@tauri-apps/plugin-dialog` + `read_text_file` | `<input type="file">` + FileReader |
| [SettingsView.vue](../../src/views/SettingsView.vue) 接收/保留 | `invoke("get/set_ingest_config")` | `v-if=isTauri()` 隐藏 |
| [backend.ts](../../src/composables/backend.ts) TauriBackend | invoke | 浏览器强制 HttpBackend |

**关键发现**：回放/诊断核心（usePlayer/PlayerView）已 Tauri-clean。难的部分可移植，剩下是机械 dispatch。

### 2.2 视觉债务

- 视图巨石：MainView 921 行 / PlayerView 934 行 / SettingsView 274 行
- 无导航骨架：3 路由，rail 是源机架非导航
- 多租户不可见：P9 服务端能力无 console 出口
- player 独立窗口：浏览器要新标签 workaround
- 视觉偏复古：暖褐胶片底 + 装饰性 mono，偏 vintage 广播台

## 3. 信息架构重设计

### 3.1 新 IA

```
App Shell
├ 侧栏导航（图标 + 文字）
│   ├ 会话 Sessions       /            会话列表 + 筛选 + 搜索
│   ├ 实时 Live           /live        源机架 + 录制控制
│   ├ 租户 Tenants        /tenants     tenant 列表 + 详情（P9 感知）
│   └ 设置 Settings       /settings    分 tab：连接 / 采集 / 保留 / 关于
├ 顶栏
│   ├ Tenant 上下文       acme ▾        /whoami 驱动 + 配额余量条
│   ├ 连接指示器          ● acme @ host
│   ├ 搜索 / cmd+k        ⌘K
│   └ 账号 / key

路由
  /              Sessions 列表
  /live          Live 观测（源机架 + 录制）
  /s/:id         Player 全屏路由 + 面包屑 [取代 /player/:id 独立窗口]
  /tenants       Tenant 列表
  /tenants/:id   Tenant 详情（配额 / 会话数 / 保留 / redact，只读）
  /settings      分 tab 设置
  /login         浏览器登录墙（Tauri 桌面跳过）
```

### 3.2 关键变化

- **MainView 拆分**：Sessions（浏览）+ Live（观测控制）各自单一职责
- **Player in-app 路由**：`/s/:id` + 面包屑返回，Tauri/浏览器统一；新标签可选但非唯一
- **Tenants 路由**：给 P9 多租户一个产品出口（只读起步）
- **顶栏 tenant 上下文**：多租户感知从「指示器小点」升为「一等公民」

## 4. 视觉语言：现代科技感方向

D2 决策：告别复古暖褐，转向现代 dev tool 科技质感。**保留琥珀为信号主色**（品牌身份），但基底、用法、质感全面刷新。像素级执行走 `/frontend-design`。

### 4.1 方向

| 维度 | 现状（cutting room） | 演进（现代科技感） |
|---|---|---|
| 基底 | 暖褐 `#14110D`（偏黄褐、复古） | 中性偏冷暗（深空感，接近现代 dev tool） |
| 主色 | 琥珀 `--amber`（装饰 + 强调 + 激活，用得多） | 琥珀收敛为「信号主色」：播放头 / 激活 / 实时数据流 |
| 辅色 | 无明确辅色 | 引入冷青/teal 作「数据辅色」：网络 / 信息流 / 可视化，与琥珀暖冷对比 |
| 字体 | 大量 mono（标题/标签/状态/正文混用） | mono 严格限定时间码/ID/技术值（科技嗓音）；正文 sans，标题可稍紧凑几何感 sans |
| 质感 | 重面板 + 厚边框 | 轻量卡片 + 细 hairline + 微妙 elevation（shadow） |
| 效果 | 无 glow | 微妙发光（glow）用于激活态与实时数据；克制动效（信号脉冲） |
| 隐喻 | 胶片 / 广播台 / 剪辑室 | 示波器 / 终端 / 数据流（数字感） |
| 对标 | - | Vercel 克制 + Linear 细节 + Raycast 数字感；琥珀+青双色信号系统 |

### 4.2 保留与摒弃

- **保留**：琥珀信号主色、牛血红 `--oxblood` 用于 REC/危险、等宽字体用于技术值、暗色基底、session studio 概念定位
- **摒弃**：暖褐胶片底、复古广播台隐喻、装饰性 mono 滥用、厚边框重面板

### 4.3 token 草图（方向，非终稿）

```
--ink:        #0A0C10   /* 深空中性暗底（替代暖褐） */
--slate:      #11141A   /* 面板表面 */
--slate-2:    #181C24   /* 抬升表面 */
--hair:       #1F242E   /* 细 hairline */
--bone:       #E6EAF0   /* 主文字（偏冷白） */
--ash:        #8B94A3   /* 次文字 */
--amber:      #F0A83D   /* 信号主色（略提亮，数字感） */
--amber-glow: rgba(240,168,61,0.18)  /* 激活态发光 */
--teal:       #4DD0C8   /* 数据辅色（冷青） */
--oxblood:    #E5484D   /* REC/危险 */
```

> 上述色值仅示意方向，终稿由 `/frontend-design` 流程定。

## 5. 组件化拆分

把三个巨石 SFC 拆成组件系统：

```
src/components/
├ shell/                  app 骨架
│   ├ AppShell.vue        侧栏 + 顶栏 + 主内容区
│   ├ SideNav.vue         导航
│   ├ TopBar.vue          tenant 上下文 + 连接指示 + 搜索 + 账号
│   └ TenantSwitcher.vue  tenant 选择（/whoami 驱动）
├ sessions/               会话浏览
│   ├ SessionList.vue
│   ├ SessionCard.vue     单条会话卡（取代行，信息更丰富）
│   ├ SessionFilters.vue  筛选 + 搜索
│   └ EmptyState.vue      设计过的空状态
├ live/                   实时观测
│   ├ SourceRack.vue      源机架（从 MainView 拆出）
│   └ RecordControls.vue
├ player/                 回放
│   ├ PlayerShell.vue     /s/:id 容器 + 面包屑
│   ├ ReplayGrid.vue      rrweb 回放区
│   ├ Timeline.vue        多轨时间轴（签名元素保留）
│   └ DiagnosisPanel.vue  信号/标注侧栏（可折叠）
├ tenants/                [新]
│   ├ TenantList.vue
│   └ TenantDetail.vue
├ settings/               分 tab
│   ├ ConnectionTab.vue   云端连接 + tenant 信息
│   ├ CaptureTab.vue      采集开关（Tauri only）
│   ├ RetentionTab.vue    保留策略
│   └ AboutTab.vue
└ common/                 基础组件（按钮/卡片/标签/空态/骨架）
```

拆分原则：每组件 < 200 行，单一职责，props/inject 接收数据。composable（usePlayer/useAnnotations/useRecorder）保持，组件消费。

## 6. 多租户产品化

P9 服务端多租户在 P10 首次有 console 出口：

| 表面 | 内容 | 数据源 |
|---|---|---|
| 顶栏 Tenant 上下文 | `acme ▾` + 配额余量条 | `GET /whoami` |
| 连接指示器 | `● acme @ host`（多租户）/ `● host`（单租户） | /whoami |
| /tenants 列表 | 当前 key 可见 tenant（起步只 1 个） | /whoami |
| /tenants/:id 详情 | 配额用量 / 会话数 / 保留策略 / redact 配置（只读） | /whoami + /sessions |
| 设置 · 连接 tab | endpoint + key + 「当前租户」信息块 | /whoami |

## 7. 浏览器化

### 7.1 Tauri 抽象层

新增 `src/composables/tauri.ts`：

```ts
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
// 窗口：Tauri=invoke open_window / 浏览器=window.open 新标签
export async function openRoute(route: string): Promise<void>
// 文件选择：Tauri=dialog.open / 浏览器=<input type=file> + FileReader
export async function pickBundleFile(): Promise<{ content: string; name: string } | null>
```

- `@tauri-apps/api` 改动态 import（仅 isTauri() 时加载），浏览器构建不打包死代码。
- [App.vue](../../src/App.vue)：`if (isTauri()) useRecorder()`。
- [MainView](../../src/views/MainView.vue)（拆分后 Live/Sessions 组件）：openRoute/pickBundleFile 替换直接 invoke。
- [SettingsView](../../src/views/SettingsView.vue)（拆分后各 tab）：Capture/Retention tab `v-if=isTauri()`。
- [backend.ts](../../src/composables/backend.ts)：`getBackend()` 浏览器强制 HttpBackend。

### 7.2 observer-server 托管

- [lib.rs](../../crates/observer-server/src/lib.rs) 加静态文件服务：`GET /` -> `index.html`，`GET /assets/*` -> 构建产物。API 路由（`/ingest`/`/sessions`/`/whoami`）优先，其余 fallback 静态。
- [bin/observer_server.rs](../../crates/observer-server/src/bin/observer_server.rs) 加 `--web-dir` 参数（指向 console 构建产物）。
- tiny_http 加文件读取 + MIME 分派（html/js/css/json/svg）。
- 部署：`observer-server --bind 0.0.0.0:8080 --tenants-file tenants.json --web-dir ./web`，浏览器开 `http://host:8080`。

### 7.3 登录墙

- 新增 `src/views/LoginGate.vue`：无有效 backend 配置（endpoint+key）且非 Tauri 时，显示登录表单。提交 -> 调 `whoami()` 验证 -> 成功存 localStorage 进主应用；失败提示。
- Tauri 桌面：默认本地模式，无登录墙。
- 浏览器：必须登录（无本地 server）。
- key 存 localStorage，跨标签共享（player 新标签需要）。

### 7.4 player in-app 路由（浏览器化关键收益）

重设计把 player 从独立窗口变 `/s/:id` 路由后，浏览器天然支持（同源 localStorage 共享 key），不需要 window.open workaround。新标签可选（`window.open('#/s/<id>')`）但非唯一路径。

## 8. 后端接口

### 8.1 `GET /whoami`

```rust
// observer-server/src/routes.rs
("GET", ["whoami"]) => {
    // tenant 已在 handle_request 解析（bearer key -> TenantConfig）
    match tenant {
        Some(t) => Ok((200, Some(json!({
            "multiTenant": true,
            "tenantId": t.tenant_id,
            "appIds": t.app_ids,
            "quotaBytes": t.quota_bytes,
            "usageBytes": /* QuotaTracker 当前用量 */,
            "rateLimit": t.rate_limit,
            "retention": t.retention,
        }).to_string()))),
        None => Ok((200, Some(json!({ "multiTenant": false }).to_string()))),
    }
}
```

- 鉴权同 `/sessions`（多租户走 key，单租户走 auth_token）。
- `usageBytes` 让顶栏配额条实时可见，复用 P9 `QuotaTracker`（读 AtomicU64，零成本）。
- 单租户模式返回 `{ multiTenant: false }`，前端据此隐藏 tenant 上下文。

### 8.2 静态文件服务

```rust
// observer-server/src/lib.rs handle_request 末尾 fallback
// API 路由（/ingest, /sessions, /whoami）未命中 -> 尝试静态文件
if let Some(web_dir) = &config.web_dir {
    let path = /* url -> web_dir 下文件路径，防穿越 */;
    return serve_static(web_dir, path, req);
}
```

- 路径穿越防护：规范化请求路径，拒绝 `..`，仅允许 `assets/` 子目录或根 `index.html`。
- SPA fallback：未知路径返回 `index.html`（hash 路由客户端解析）。
- MIME 分派：`.html`/`.js`/`.css`/`.json`/`.svg`/`.woff2`。

### 8.3 ServerConfig 扩展

```rust
pub struct ServerConfig {
    // ... 现有字段
    pub web_dir: Option<PathBuf>,  // P10：console 静态托管目录
}
```

## 9. 分阶段实施

| 阶段 | 内容 | 产出 | 依赖 |
|---|---|---|---|
| 0 | 设计语言刷新 | 新 theme.css + common/ 组件库 | `/frontend-design` |
| 1 | app shell + 导航 + Sessions 拆分 | AppShell + /sessions 路由 | 0 |
| 2 | Live 视图 + 录制控制拆分 | /live 路由，源机架独立 | 1 |
| 3 | Player in-app 路由化 + 拆分 | /s/:id 路由，PlayerView 组件化 | 1 |
| 4 | 多租户产品化 | /whoami + 顶栏 tenant 上下文 + /tenants | 1 |
| 5 | 设置分 tab + 情境化 | /settings 分 tab，Tauri 守卫 | 1 |
| 6 | 浏览器化 | Tauri 抽象 + 静态托管 + LoginGate | 1-5 |
| 7 | 产品打磨 | 空状态 + skeleton + cmd+k + 响应式 | 6 |

每阶段独立可交付、可回归。阶段 0 必须先过 `/frontend-design`，定 token + 基础组件后，后续阶段消费。

## 10. 测试计划

### 10.1 后端

- `GET /whoami`：多租户返回 tenant 信息 + usageBytes；单租户返回 `{ multiTenant: false }`；无 key 401。
- 静态服务：`GET /` 返回 index.html；`GET /assets/x.js` 返回 JS + 正确 MIME；`/ingest`/`/sessions` API 不被静态拦截；`..` 路径穿越被拒。
- 回归：P8/P9 现有 13+45 测试全过。

### 10.2 前端

- Tauri 桌面：所有现有功能（录制/回放/导入导出/标注/设置）回归。
- 浏览器：observer-server 托管 -> 登录 -> Sessions/Live/Player/Tenants/Settings 全功能。
- `isTauri()` dispatch：窗口/文件/录制在两环境正确分派。
- player `/s/:id` 路由：面包屑返回、新标签可选、两端一致。

### 10.3 视觉

- 现代科技感落地：基底中性偏冷暗、琥珀信号主色、冷青辅色、glow/细线。
- 无退化回 Element Plus 默认或通用暗色模板。
- 空/错/加载态设计过（非一行字）。

## 11. 风险与开放问题

| # | 风险 | 处理 |
|---|---|---|
| R1 | 视觉方向调整破坏身份 | /frontend-design 流程；保留琥珀/牛血红/mono 锚点 |
| R2 | 拆分巨石 SFC 回归 | 渐进式，每阶段跑 P8/P9 测试流程；composable 不动只拆组件 |
| R3 | 视觉刷新跨所有表面工作量大 | 阶段 0 定 token + 基础组件，后续消费；不一处处手改 |
| R4 | player 路由化心智变化 | 面包屑 + 返回快捷键；保留新标签可选 |
| R5 | /whoami 暴露 usageBytes 性能 | QuotaTracker 已缓存，读 AtomicU64 零成本 |
| R6 | 浏览器 localStorage key XSS | 自托管私有云可控；文档明示；未来换 httpOnly cookie |
| R7 | admin UI 蔓延 | /tenants 起步只读；增删改留 P10.5/P11 |
| R8 | 工作量周期长 | 阶段化交付，1-3 即可见结构改善 |
| R9 | observer-server 静态服务拖累 API | tiny_http 串行；静态少且可缓存；高并发上反代 |
| R10 | type:6 信号浏览器回放 | usePlayer 已 Tauri-clean，主要靠测试验证 |

## 12. 与现有代码的关系

- [theme.css](../../src/styles/theme.css)：视觉 token 刷新（阶段 0）。
- [MainView.vue](../../src/views/MainView.vue)（921 行）：拆为 Sessions + Live + 组件。
- [PlayerView.vue](../../src/views/PlayerView.vue)（934 行）：拆为 PlayerShell + ReplayGrid + Timeline + DiagnosisPanel；路由 `/player/:id` -> `/s/:id`。
- [SettingsView.vue](../../src/views/SettingsView.vue)：拆为 settings/ 各 tab。
- [App.vue](../../src/App.vue)：挂 AppShell + LoginGate + 条件 useRecorder。
- [router/index.ts](../../src/router/index.ts)：路由重组（新增 /live /s/:id /tenants /login）。
- [backend.ts](../../src/composables/backend.ts)：加 `whoami()` 接口 + 浏览器强制 HttpBackend。
- 新增 `src/composables/tauri.ts`：isTauri + dispatch。
- 新增 `src/views/LoginGate.vue`。
- 新增 `src/components/{shell,sessions,live,player,tenants,settings,common}/`。
- [crates/observer-server/src/routes.rs](../../crates/observer-server/src/routes.rs)：`GET /whoami`。
- [crates/observer-server/src/lib.rs](../../crates/observer-server/src/lib.rs)：静态文件服务 + ServerConfig.web_dir。
- [crates/observer-server/src/bin/observer_server.rs](../../crates/observer-server/src/bin/observer_server.rs)：`--web-dir` 参数。

## 13. 不做什么（scope 纪律）

- admin API / 多租户管理 UI（/tenants 起步只读）。
- OAuth / SSO / 复杂组织模型。
- 移动端原生适配（响应式够用即可）。
- 回放外部资源完全保真（文档化 + 告警，B9）。
- 实时流式回放（仍按需点播）。
- 告警 / 生产 RUM（与锁定决策 #3 一致，永不做）。
