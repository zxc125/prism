# P10：浏览器版 console（可选）

> 阶段路径第 10 阶段（可选，规划中）。目标：零安装浏览器访问 console。

## 目标

仅当「不许装客户端」时才做：浏览器打开云端 console，回放/诊断/标注全功能。

## 范围

1. **web console**：复用 Vue views，`Backend` 用 `HttpBackend`；Tauri 专属 API（invoke/webviewWindow）全部走 HTTP/浏览器替代。
2. **replay 插件浏览器版**：type:6 信号 replay 在浏览器跑通。
3. **资源保真**：外部 CSS/字体/图片缺失告警（见方案 B9），可选内联关键样式表。

## 改动

- console 前端拆 Tauri 依赖，出 web 构建。
- replay 侧浏览器验证。

## 验收

- 浏览器打开云端 console，登录后回放/诊断/标注/导出全功能。

> 备注：P8 的 Tauri-as-cloud-client 已覆盖私有云场景（装一次 desktop client 连云端）。本阶段仅当明确需要「零安装浏览器访问」才做，优先级最低。详见 [离线导出与云端部署（方案）.md](../架构/离线导出与云端部署（方案）.md) Phase D。
