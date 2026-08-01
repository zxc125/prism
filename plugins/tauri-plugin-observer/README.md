# tauri-plugin-observer

本地优先前端观测平台 · Tauri 2 多窗口录制协调插件。

## 两种模式

- **Local**（console 自录）：Rust 直接落盘到 `appDataDir/recordings/`
- **Remote**（外部 Tauri 应用）：Rust 只协调窗口 + 事件驱动，前端经 `HttpSink`（[`@prism-obs/observer-tauri`](https://www.npmjs.com/package/@prism-obs/observer-tauri)）上报到 console server

## 安装（外部 Tauri 应用）

`Cargo.toml`：

```toml
[dependencies]
tauri-plugin-observer = "0.1"
```

配合 JS 端（npm）：

```bash
pnpm add @prism-obs/observer-tauri @prism-obs/observer-sdk
```

## License

MIT
