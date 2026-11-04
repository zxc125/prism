# tauri-demo — 外部 Tauri 应用观测样例

独立的 Tauri 2 桌面应用，安装 [`tauri-plugin-observer`](https://crates.io/crates/tauri-plugin-observer)（**Remote 模式**）+ [`@prism-obs/observer-tauri`](https://www.npmjs.com/package/@prism-obs/observer-tauri)，演示「外部 Tauri 应用多窗口录制 → 上报 console」的完整接入（见 [Tauri Plugin 手册](../../site/docs/tauri.md)）。

## 跑法

> **必须在本仓库 checkout 内跑**：Rust 侧以 path 依赖引用 `../../../plugins/tauri-plugin-observer`，脱离仓库无法编译。依赖走 workspace，先在**仓库根** `pnpm install` 过。

```sh
# 1. console 先跑起来（另开终端）：仓库根 pnpm tauri dev

# 2. 起本示例
cd examples/tauri-demo
pnpm tauri dev
```

- 默认上报 `http://127.0.0.1:1421`；应用内置配置 UI 可改 endpoint / token（存 localStorage，改完自动 reload 重连），也支持 URL 参数 `?endpoint=...&token=...`。
- **✅ 跑通的标志**：窗口状态区显示「采集中 · 主窗口」（绿点）；console 会话浏览器出现 appId 为 `tauri-demo` 的会话。

## 一键上传（P24）

Local 模式下「上传到观测端（Local）」按钮：把本地全部落盘会话逐个打包成 bundle，串行 POST 到观测端 `/sessions/import`（逐会话回执：成功显观测端新 id，失败显原因）。

- 上传目标 = 「连接配置」里的 endpoint / token（Local 模式同样显示并持久化；不使用 URL 参数注入 token）。
- **重复点击会在观测端产生重复会话**（import 分配新 id，无去重）——重试安全，但会在 console 留副本。
- gzip 自动协商：探测 `/whoami` 的 `requestGzip` 标记，新版 server 压缩上传、老 server 自动回退明文。
- 超大会话上传失败时，可改走「导出 bundle（Local）」产文件，再在 console 会话页导入。
- **✅ 跑通的标志**：console 会话浏览器出现多个带「导入」标记的会话；回执区逐行 ✓/✗。
- 「**选择 bundle 文件上传**」按钮：任选一个 `.json` bundle 文件（不经本地会话列表，如别处导出的文件）→ `parseBundle` 预校验 → 单文件上传，同样的回执区展示。

## 手动档门控（P17）

环境变量开关（缺省 = 常录，会话活跃即持续录制）：

```sh
VITE_OBSERVER_GATING=manual pnpm tauri dev
```

- `gating: "manual"` 下，`recording-session{active}` 广播与挂载兜底**不再自动开段**；段边界由 demo 内置的最小宿主策略驱动：交互（pointerdown/keydown/wheel/touchstart）开段、30s 判闲停段、空闲期异常（`boom` 按钮）经 `controller.signal("error", …)` 开段注入。
- **✅ 跑通的标志**：状态区显示「手动档 · 等待交互开段」；不交互时 `segments/` 无新段文件，交互后开段、30s 后自动收段；空闲期点 `boom`，回放段头部能看到 error 信号。
- 机制说明见 [Tauri Plugin 手册](../../site/docs/tauri.md)；两种模式（`VITE_OBSERVER_MODE=local|remote`）均可叠加本开关。

## 看点

- **多窗口**：应用内按钮开子窗口——不同 label = 不同轨道，console 回放时多轨时间轴对齐。
- **关闭子窗口 = 隐藏**：录制期间关子窗口，回放里该轨道在隐藏区间自动留空；重开续录新段。
- **上报地址热切**：配置 UI 切到云端 `observer-server` 地址，无需改代码。
