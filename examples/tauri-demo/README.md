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

## 看点

- **多窗口**：应用内按钮开子窗口——不同 label = 不同轨道，console 回放时多轨时间轴对齐。
- **关闭子窗口 = 隐藏**：录制期间关子窗口，回放里该轨道在隐藏区间自动留空；重开续录新段。
- **上报地址热切**：配置 UI 切到云端 `observer-server` 地址，无需改代码。
