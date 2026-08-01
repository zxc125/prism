# observer-storage

本地优先前端观测平台 · 纯存储层（落盘 + annotations + bundle 契约）。

无 Tauri 依赖，全部以 `&Path` 操作文件系统，被 `tauri-plugin-observer`（console 自录）与 `observer-server`（自托管 HTTP server）共用。

## 功能

- **会话/段落落盘**：`recordings/<sessionId>/` 结构（`session.json` / `windows.jsonl` / `segments/*.jsonl` / `annotations.jsonl`）
- **bundle 契约**：`prism-session` 单文件序列化（跨进程/跨机迁移唯一契约），含 segmentId 校验防路径穿越 + 原子写
- **annotations**：session 级标注读写
- **读/列举/导入/导出会话**

## License

MIT
