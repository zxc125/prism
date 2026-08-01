# Web SDK

`@prism-obs/observer-sdk` —— 嵌入任意 Web 应用，录制 DOM + 诊断信号，上报到 console。

## 安装

```sh
pnpm add @prism-obs/observer-sdk
```

## 实时上报：`init()`

在被观测应用调用一次，启动 rrweb 录制 + 信号 hook，经 `HttpSink` 上报到 console 本地 server。

```ts
import { init } from "@prism-obs/observer-sdk";

const ctrl = await init({
  appId: "my-app",                          // 应用标识，console 区分来源用
  endpoint: "http://127.0.0.1:1421",        // console 本地 server（设置页可查）
  token: "<可选 token>",                     // console 开启鉴权时必传
  env: "dev",                               // 可选，环境标记
  release: "1.0.0",                         // 可选，版本标记
  label: "web",                             // 可选，段标签，默认 "web"
  signals: "all",                           // 可选，诊断信号开关，默认全开
  meta: {},                                 // 可选，透传到 session meta 的额外字段
});

await ctrl.stop(); // 显式停止（可选）
```

行为要点：

- **会话 = 一次页面访问**；SPA 路由连续，整页刷新开新段。
- 页面卸载（`beforeunload`）自动用 `sendBeacon` 兜底 flush 已缓冲事件，会话结束标记 best-effort。
- `signals` 可按需只开部分：`{ error: true, console: true, network: false }`。

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

`OfflineController` API：

| 方法 | 作用 |
| --- | --- |
| `stop()` | 显式停止，flush 残留事件，返回会话 id |
| `export(id?)` | 序列化为 bundle（默认当前会话；导出当前会话会自动 stop） |
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

## ⚠️ 被观测页需显式设置背景色

rrweb 只录 DOM 样式，**不录浏览器画布默认色**。若页面仅靠 `color-scheme: light dark` 取默认背景与文字色，回放时 iframe 画布透明会透出播放器底色，深色系统下浅色文字可能落在白底上看不见。

请给 `html, body` 显式写 `background` 与 `color`，录制即可忠实还原：

```css
html,
body {
  background: #fff;
  color: #111;
}
```

## 框架集成

入口放应用根，保证只调用一次：

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

完整可跑样例见仓库 [`examples/web-demo`](https://github.com/zxc125/prism/tree/main/examples/web-demo)。
