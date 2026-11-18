# tauri-plugin-observer

本地优先前端观测平台 · Tauri 2 多窗口录制协调插件。

## 两种模式

- **Local**（console 自录 / 外部应用 opt-in 本地落盘）：Rust 直接落盘到 `<dir_base>/<dir_name>`——默认 `appDataDir/recordings/`；`ObserverConfig.dir_base`（`AppData`/`ResourceDir`）+ `dir_name`（默认 `"recordings"`）可配（P20，切根不迁移历史会话）；`retention` 保留策略自动清理可配（P23，缺省不清扫，活跃会话豁免）
- **Remote**（外部 Tauri 应用）：Rust 只协调窗口 + 事件驱动，前端经 `HttpSink`（[`@prism-obs/observer-tauri`](https://www.npmjs.com/package/@prism-obs/observer-tauri)）上报到 console server

## 安装（外部 Tauri 应用）

`Cargo.toml`：

```toml
[dependencies]
tauri-plugin-observer = "0.4"
```

配合 JS 端（npm）：

```bash
pnpm add @prism-obs/observer-tauri @prism-obs/observer-sdk
```

## 依赖披露（0.7.0 起）

自 0.7.0 起本 crate 含 **C 编译依赖 `zstd-sys`**（`zstd = "0.13"`，仅 `export_session_to_file` 的 `compress = true` 路径使用）：交叉编译需目标平台 C 工具链（cc）。仅需流式导出（内存恒定）不需要压缩交付的消费方，可钉 `tauri-plugin-observer = "=0.6.1"`（P25 论证改判前的 M1 里程碑版，按需补发——见 docs/阶段路径/P25-导出流式化与压缩交付.md 论证改判记录）。

## License

MIT
