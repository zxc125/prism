# 导出 to-file 变体决策

> P18 /cycle 产物（2026-09-10 spec）。背景：bond/frontend 0910 提案（`docs/bugs/0910-prism-导出命令async化与to-file变体提案.md`，bond 仓）指出导出链路三热点；改动 A（`export_session` async 化，crates 0.2.2）已随 P17 修复记录落地，本决策覆盖改动 B——插件代执行落盘的 `export_session_to_file` 变体。实现流水账见 [docs/阶段路径/P18-导出to-file变体.md](../阶段路径/P18-导出to-file变体.md)。

## 范围决策

- **归属**：升格新 P18 走 /cycle，非 /fix。改动 B 新增对外命令（新 API）+ 扩展 P16 D5 决策，命中 `/fix` vs `/cycle` 升格准则两条（改对外 API + 动锁定决策）。
- **只加变体不改本体**：`export_session`（0.2.2 已 async 化）保持原样。Remote 宿主与浏览器消费方仍走「返回数据」形态；to-file 仅 Local 模式新增选项。
- **不做 bond §4 兜底对账**：上游 B 落地后 bond 侧兜底命令自行删除（其本仓决策），上游不跟踪。

## 技术路线决策

- **D5 扩展而非推翻**：D5 原文「落盘**目标**由宿主决定（fs/dialog 插件各异）」——变体仍由宿主传 `path`，插件只代执行「大 JSON 序列化 + 文件写入」两个高开销动作，落盘目标决定权不变。动机：几十 MB bundle 经 JS stringify + 字节数组 + IPC 三重放大（提案②③），Rust 直写盘根治。
- **执行模型用 `async fn` + `spawn_blocking`，不用 `#[tauri::command(async)]` 属性**：区别于 P17 D3（既有命令最小改动）——新命令无兼容包袱，且 build→serialize→write 是**秒级**阻塞，须丢到阻塞线程池而非占用 async runtime worker。
- **原子写**：同级 `.tmp` 写入成功后 rename 到目标路径（P7 导入侧惯例）。磁盘满/中断不留残缺 bundle。
- **键序与逐字节一致性**：本仓未开 serde_json `preserve_order`（Map = BTreeMap 排序键），`export_session` 响应与 to-file 落盘走同一份 serde_json，键序天然一致；唯一差异是 `exportedAt: now_ms()` 每次调用必变——**验收为「除 `exportedAt` 外逐字节一致」**（修正 bond 提案 §5 原文「逐字节一致」，按原文必失败）。

## 边界决策

- **任意写盘能力**：不做路径白名单、不做父目录限制（导出目标天然任意）。能力面论证：`observer:default` 已含 `allow-export-session`——全量数据本就回到宿主 JS、JS 本可写任意位置，to-file **无实质能力升级**；叠加宿主 path 通常经系统 save 对话框取得（用户授权语义）。
- **session_id 校验不豁免**：`path` 任意是设计使然，但 `session_id` 仍走 `validate_session_id` 防路径穿越（P7 防护复用），与 `export_session` 同门禁（`ensure_local`）。
- **父目录不存在不自动创建**：`fs::write` 报错即返回（save 对话框给出的路径父目录必然存在；自动建目录反而扩大副作用面）。

## 默认值 / 取值

- 临时文件名 = `<path>.tmp`（同级）：同分区 rename 原子性保证。
- 权限标识 = `allow-export-session-to-file` / `deny-export-session-to-file`（三件套自动生成惯例）。
- 发版 = crates 0.3.0（additive minor，0.x 先例）+ npm `@prism-obs/observer-tauri` 0.5.0（加 TS 包装）；`observer-storage` 不动。

## 未做（留后续）

- console 前端导出切 to-file + dialog：现有 Blob 下载交互不变，切走需引 dialog 插件且改变体验，留独立小改（console 也命中②③，收益真实但非本 P）。
- `read_session` 同款大载荷问题（console 查看会话全量事件走 IPC）：形态不同（非文件落盘），留 P19+ 候选。
- 手册选项全集表补 to-file：随实施在 site/docs/tauri.md zh/en 同步。
