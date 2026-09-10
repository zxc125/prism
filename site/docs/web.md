# Web SDK

`@prism-obs/observer-sdk` —— 嵌入任意 Web 应用，录制 DOM + 诊断信号，上报到 console。

> **适用版本**：`@prism-obs/observer-sdk` **0.3.x**。API 有变更时本页会同步更新并改此标注。

**本页你将完成**：安装 SDK → 最小接入跑通 → 按需配置录制量控 / 离线采集 / 脱敏。前置：已按 [快速开始](./quickstart) 跑通 console。

## 安装

```sh
pnpm add @prism-obs/observer-sdk
```

## 快速接入：`init()`

在被观测应用入口调用一次，启动 rrweb 录制 + 诊断信号 hook，经 HTTP 上报到 console：

```ts
import { init } from "@prism-obs/observer-sdk";

const ctrl = await init({
  appId: "my-app",                       // 必填：应用标识，console 区分来源用
  endpoint: "http://127.0.0.1:1421",     // 必填：console 本地 server（设置页可查）
});

// ctrl.stop() 可显式停止；页面卸载会自动 sendBeacon 兜底
```

**✅ 验证**：打开被测页的 DevTools → Network，能看到发往 endpoint 的 `/ingest` 请求；console 会话列表出现你的应用。

### `init()` 选项全集

| 选项 | 类型 | 必填 | 默认 | 说明 |
| --- | --- | --- | --- | --- |
| `appId` | `string` | ✅ | — | 应用标识，随会话上报，console 侧区分来源 |
| `endpoint` | `string` | ✅ | — | console 本地 HTTP server 地址，如 `http://127.0.0.1:1421` |
| `token` | `string` | ➖ | — | 本地鉴权 token；console 开启鉴权时必传（设置页可查） |
| `env` | `string` | ➖ | — | 环境标记（`dev` / `staging` / `prod`，自定） |
| `release` | `string` | ➖ | — | 版本标记，回放排查「哪个版本出的问题」 |
| `label` | `string` | ➖ | `"web"` | 段标签。SPA 路由连续算同段，整页刷新开新段 |
| `signals` | `SignalSet` | ➖ | `"all"` | 诊断信号开关：`"all"`，或按需部分开，如 `{ error: true, console: true, network: false }` |
| `recording` | `RecordingOptions` | ➖ | — | 录制量控（见下节）；缺省 = 全量录制 |
| `meta` | `object` | ➖ | — | 透传到 session meta 的额外字段（如用户 id、订单号） |

行为要点：

- **会话 = 一次页面访问**；SPA 路由连续，整页刷新开新段。
- 页面卸载（`beforeunload`）自动用 `sendBeacon` 兜底 flush 已缓冲事件，会话结束标记 best-effort。

## 录制量控 recording

高频页面（行情推送、轮询渲染、密集动画）会产生海量事件。`recording` 配置两件事：**档位**（收紧哪些采集通道）与**免录区**（哪些 DOM 根本不录）。

```ts
init({
  appId: "my-app",
  endpoint: "http://127.0.0.1:1421",
  recording: {
    profile: "minimal",                     // 档位，缺省 "full"
    domBlocks: [".quote-table", ".ad-banner"], // 免录区 CSS selector，缺省不设
  },
});
```

`RecordingOptions`：

| 选项 | 类型 | 说明 |
| --- | --- | --- |
| `profile` | `"full" \| "balanced" \| "minimal"` | 录制档位，缺省 `full` |
| `domBlocks` | `string[]` | 免录区域：CSS selector 列表。命中元素及其子树的 DOM 变更**不进事件流**，回放时显示占位块 |

三档行为差异：

| 档位 | mousemove | scroll | 输入框 | 其他鼠标/触摸交互 | 次要节点（script/注释/head 杂项等） | 适用 |
| --- | --- | --- | --- | --- | --- | --- |
| `full`（默认） | 全量（50ms 聚合） | 全量 | 全量 | 全量 | 保留 | 默认，与旧版零差异 |
| `balanced` | 100ms 聚合 | 200ms 节流 | 只记最终值 | 全量 | 剪枝 | 一般业务页降量 |
| `minimal` | 关闭 | 500ms 节流 | 只记最终值 | 仅 Click / TouchStart / TouchEnd | 全面剪枝 | 只关心「用户点了什么」 |

::: warning 档位管不住 mutation
三档收紧的都是**交互通道与次要节点**，**任何档位都不降低 DOM mutation 事件量**。行情推送、秒级轮询渲染这类高频 DOM 场景，请搭配 `domBlocks` 免录区——把高频刷新的区域排除出录制（回放中该区域显示占位），事件量与回放体积才会真正降下来。
:::

想直观对比三档差异？仓库示例 [examples/web-demo](https://github.com/zxc125/prism/tree/main/examples/web-demo) 支持 URL 参数：`?sim=1` 启动行情模拟，`?profile=minimal&block=.quote-table` 一键套用量控配置。

### 高频渲染场景

录制本身有主线程成本：rrweb 的 mutation 序列化跑在主线程，**任何档位都绕不开**。高频渲染页（虚拟滚动表格、行情推送、秒级轮询）按两步把成本压到可用：

1. **免录区罩住「行容器」，不只是单元格**。虚拟滚动列表滚动时整行挂载/卸载，只挡个别列（如价格 cell）挡不住行级增删——把行容器纳入免录区（如 `domBlocks: [".vtable .row"]`），滚动期的 mutation 才不会逐帧进流。代价是回放中这些区域显示占位块。
2. **让段边界避开高频期**。Tauri 应用可配 `gating: "manual"` 手动档：交互开段、空闲停段，空闲期零录制成本（见 [Tauri Plugin 手册](./tauri#手动档段门控-p17)）；开段头部是一次全量快照，尽量让开段时机落在交互间隙而非渲染洪峰中。

## 离线采集：`recordOffline()`

不依赖 console 在线：事件实时落浏览器 IndexedDB，之后导出 `prism-session` bundle，下载或上传。

```ts
import { recordOffline } from "@prism-obs/observer-sdk";

const ctrl = await recordOffline({ appId: "my-app", release: "1.0.0" });

// ... 用户操作被录下 ...

// 导出当前会话为 bundle（会自动 stop）
const bundle = await ctrl.export();
// 或直接触发浏览器下载
await ctrl.download();

// 列出本机所有离线会话
const sessions = await ctrl.list();
// 清理
await ctrl.clear();
```

选项与 `init()` 基本一致（`appId` 必填；`env` / `release` / `label` / `signals` / `recording` / `meta` 可选），但**没有** `endpoint` / `token`——离线不联网。

`OfflineController` API：

| 方法 | 作用 |
| --- | --- |
| `sessionId` | 当前会话 id（只读属性） |
| `stop()` | 显式停止，flush 残留事件，返回会话 id |
| `export(id?, redactOpts?)` | 序列化为 bundle（默认当前会话；导出当前会话会自动 stop） |
| `download(id?, filename?, redactOpts?)` | export + 触发浏览器下载 |
| `list()` | 列出本机所有离线会话 meta（按开始时间倒序） |
| `clear(id?)` | 删除指定会话；不传 id = 清空全部 |
| `destroy()` | 销毁控制器：移除 unload 钩子并停止录制（保留已录数据） |

> rrweb 事件经缓冲（默认 1s flush），页面突然关闭可能丢失末尾 <1s 的事件；正常 `stop()` 收尾可避免。已落盘的会话可经 `list()` 找回再导出。

## 脱敏：`redact()`

导出 / 分享 bundle 前，剥离或 scrub 掉 PII（network body、headers、token、邮箱等）。

```ts
import { redact } from "@prism-obs/observer-sdk";

const clean = redact(data, {
  stripNetworkBody: true,      // 默认 true，PII 压力最大
  stripNetworkHeaders: true,   // 默认 true
  dropNetwork: false,          // 完全丢弃 network 信号事件
  dropConsole: false,          // 完全丢弃 console 信号事件
  scrubbers: [                 // 正则 scrubber，匹配并替换为 [REDACTED]
    /Bearer\s+[\w.-]+/g,
    /[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+/g,
  ],
});
```

`recordOffline().download()` / `export()` 可直接传 `redactOpts`，导出即脱敏。上云 / 分享前务必过一遍。

## 诊断信号

三类信号交错进事件流（`type: 6`），与 DOM 共享时间轴：

| 信号 | hook 点 | 语义 |
| --- | --- | --- |
| `error` | `window.onerror` + `unhandledrejection` | 同步抛错与未捕获 Promise |
| `console` | `console.{log,warn,error,info,debug}` | 参数经序列化（Error 转结构、Node 转简述、循环引用截断） |
| `network` | `fetch` + `XMLHttpRequest` | url / method / status / duration，失败时带 error |

## 被观测页需显式设置背景色

::: warning
rrweb 只录 DOM 样式，**不录浏览器画布默认色**。若页面仅靠 `color-scheme: light dark` 取默认背景与文字色，回放时 iframe 画布透明会透出播放器底色——深色系统下浅色文字可能落在白底上看不见。
:::

给 `html, body` 显式写 `background` 与 `color`，录制即可忠实还原：

```css
html,
body {
  background: #fff;
  color: #111;
}
```

## 框架集成

没有现成项目？从零起一个：

```sh
npm create vite@latest my-app -- --template vue-ts   # 或 react-ts / vanilla-ts
cd my-app
pnpm install
pnpm add @prism-obs/observer-sdk
```

`init()` 放应用入口，保证只调用一次：

::: code-group

```ts [Vue — main.ts]
import { init } from "@prism-obs/observer-sdk";
init({ appId: "my-app", endpoint: "http://127.0.0.1:1421" });
// createApp(App).mount('#app')
```

```ts [React — index.tsx]
import { init } from "@prism-obs/observer-sdk";
init({ appId: "my-app", endpoint: "http://127.0.0.1:1421" });
// ReactDOM.createRoot(...).render(...)
```

:::

::: warning SSR 环境
`init()` 内部访问 `window` / `navigator` / `document`，**只能在浏览器端调用**。Nuxt / Next 等 SSR 框架请放在客户端生命周期里（Vue 的 `onMounted`、React 的 `useEffect`），或用 `import.meta.client` / `typeof window !== "undefined"` 守卫包住。
:::

## 故障排查

| 现象 | 多半是 | 解法 |
| --- | --- | --- |
| `init()` 报错 / 页面显示连接失败 | endpoint 不对或 console 没在跑 | 确认 console 主窗口开着、地址端口与设置页一致；401 = token 不匹配 |
| 回放画面空白 / 文字看不见 | 页面没显式设置背景色 | 见上文[背景色须知](#被观测页需显式设置背景色) |
| 事件量 / 会话体积太大 | 高频交互或高频 DOM 渲染 | 上 `recording` 量控；高频 DOM 场景配 `domBlocks`（见[录制量控](#录制量控-recording)） |
| 录制开启后滚动 / 切页卡顿 | mutation 序列化的主线程成本 | 免录区罩住虚拟表行容器 + 段边界避开高频期（见[高频渲染场景](#高频渲染场景)） |
| SSR 构建报 `window is not defined` | `init()` 在服务端被调用 | 见上文 SSR 警示，挪到客户端生命周期或加守卫 |

## 完整示例

完整可跑样例见仓库 [`examples/web-demo`](https://github.com/zxc125/prism/tree/main/examples/web-demo)（含行情模拟 + 三档量控对比，跑法见其 README）。
