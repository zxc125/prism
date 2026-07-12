---
name: rrweb-recording
description: Guidance for the rrweb-based multi-window recording & replay system in this Tauri 2 + Vue 3 project. Covers implementation plan, current status, and unimplemented features. Use this skill whenever the user works on anything related to recording, replay, 回放, 录制, session/segment files, the useRecorder/usePlayer composables, PlayerView/MainView recording controls, the Rust session commands (start_session, stop_session, begin_segment, append_events, list_sessions, read_session, delete_session), the recordings/ data directory, or asks about the recording architecture, current state, or planned features. Trigger even when the user doesn't say "rrweb" explicitly—any mention of window recording, session replay, multi-window playback, or close-to-hide window behavior should activate this skill.
---

# rrweb 录制与回放系统

本 skill 记录该项目中 rrweb 录制/回放功能的**实现方案**、**当前现状**与**未实现功能**。在修改录制相关代码、排查回放问题或规划新功能前，先通读本 skill，避免破坏已有的跨窗口对齐与分段机制。

## 实现方案

### 核心思路

rrweb 在**前端 webview** 里跑（JS），录制它所在窗口的 DOM 变更与用户交互。Tauri 每个窗口 = 独立 webview = 独立 JS 上下文，所以**每个窗口各跑一个 `record()` 实例**，事件按 segment 流式落盘到本地文件。回放时用每窗口一个 `Replayer`，靠**绝对时间戳**对齐到同一主时间轴。

### 数据布局

录制产物存放在 Tauri `appDataDir/recordings/<sessionId>/`：

```
recordings/<sessionId>/
  session.json              # { id, startedAt, endedAt? }
  windows.jsonl             # 窗口生命周期，每行一个 JSON
  segments/<label>#<n>.jsonl  # 每段 rrweb 事件流，每行一个事件
```

- `windows.jsonl` 每行：`{ type: "shown"|"hidden"|"focus", label, segmentId?, t }`，`t` 为 epoch 毫秒。
- segmentId 格式 `<label>#<n>`，同一窗口每次重新显示 `n` 自增。一次 show~hide = 一段，每段自带一次全量快照，可独立回放。

### 会话与段的生命周期

| 事件 | Rust 动作 | 前端动作 |
| --- | --- | --- |
| 主窗口点「开始录制」 | `start_session`：置 active、建目录、写 session.json、广播 `recording-session{active:true}` | 各窗口 `useRecorder` 收到广播 -> `invoke("begin_segment")` -> 启动 rrweb |
| 新窗口首次创建 | `open_window` build | 新窗口 `useRecorder` 挂载时 `is_recording_active` 兜底为 true -> `begin_segment` |
| 已隐藏窗口再次打开 | `open_window`：`show()`+`focus()`，录制中则 `emit_to(label,"segment",{start})` | `useRecorder` 收到 start -> `begin_segment` 开新段 |
| 子窗口点 X 关闭（录制中） | `on_window_event` CloseRequested：`prevent_close`+`hide`、记 hidden、`emit_to` segment:stop | `useRecorder` 收到 stop -> 调 rrweb stop fn、flush |
| 窗口聚焦 | `on_window_event` Focused：记 focus | - |
| 主窗口关闭 | 不拦截 -> 退出进程 | - |
| 点「停止录制」 | `stop_session`：置 inactive、关闭所有活跃段、广播 `recording-session{active:false}`、写 endedAt | 各窗口停录 |

### 关键设计决策与原因

- **hash 模式路由 + `index.html#{route}`**：新窗口靠 hash 路由定位视图，Tauri 走自定义协议/本地文件，history 模式刷新会 404。
- **segment 而非整窗单文件**：子窗口「关闭=隐藏、再开重录」导致同一窗口多次活跃；每段独立快照才能正确回放每次显示的初始状态。
- **绝对时间戳对齐**：所有窗口共享墙上时钟，rrweb 事件自带 `timestamp`，无需额外同步协议。主窗口 click 与子窗口 shown 落在同一时间轴，「连接感」自然成立。
- **子窗口关闭改隐藏**：保留 webview 与 Vue 状态，`useRecorder` 实例不销毁，靠 Rust 定向事件驱动 start/stop 段。
- **player-* 窗口跳过录制**：回放窗口本身不能被录进会话，否则污染数据。
- **底层 `Replayer` 而非 `rrweb-player`**：后者是 Vue 2 组件，不能用于 Vue 3；自建 Element Plus 控制条 + 平铺 iframe。
- **JSONL 格式**：可流式追加、断电不丢整文件、回放按行 parse。

### 文件职责

| 文件 | 职责 |
| --- | --- |
| [src-tauri/src/lib.rs](src-tauri/src/lib.rs) | `Session` 状态、录制相关命令、`open_window`、`on_window_event` 生命周期拦截 |
| [src/composables/useRecorder.ts](src/composables/useRecorder.ts) | 每窗口录制器：监听会话广播与段事件、缓冲 flush、`player-*` 跳过 |
| [src/composables/usePlayer.ts](src/composables/usePlayer.ts) | 回放控制器：加载会话、按区间驱动各 `Replayer`、play/pause/seek/倍速 |
| [src/App.vue](src/App.vue) | 挂载 `useRecorder`，使每个窗口都参与录制 |
| [src/views/MainView.vue](src/views/MainView.vue) | 开始/停止录制 + 会话列表（回放/删除） |
| [src/views/PlayerView.vue](src/views/PlayerView.vue) | 平铺回放 UI + 控制条 |

### Rust 命令清单

`greet`、`open_window`、`start_session`、`stop_session`、`is_recording_active`、`begin_segment`、`append_events`、`list_sessions`、`read_session`、`delete_session`。自定义 command 已在 `generate_handler![]` 注册，无需在 capabilities 授权；`listen` 由 `core:default` 允许。

## 当前现状

### 已实现

- 多窗口录制：主窗口 + 任意子窗口，会话期间新开的窗口自动接入。
- 子窗口关闭=隐藏、再开重录（分段），主窗口关闭=退出。
- 事件流式落盘（每秒 flush 一次）。
- 会话列表、删除。
- 多窗口平铺回放：播放/暂停/进度拖动/倍速，按 shown/hidden 区间切换可见 segment。

### 编译验证

- `cargo check`（src-tauri 内）通过。
- `pnpm exec vue-tsc --noEmit` 通过。
- `pnpm build`（含 vite 构建）通过。

### 未实测

**运行时未在桌面环境实测**——需 `pnpm tauri dev` 验证。以下点尤其需要确认：

- rrweb `Replayer` 在平铺 tile 内的缩放/尺寸是否正常。
- `pause(offset)` 是否真正 seek（rrweb 2.x 无 `goto`，seek 依赖 `pause(offset)`/`play(offset)`）。
- 各 segment `Replayer` 独立 RAF 的漂移程度。
- 录制中关闭子窗口的拦截时序、`emit_to` 定向事件是否被目标窗口稳定接收。

### 已知 MVP 限制

- 回放各 segment 各自跑 RAF，长录制可能轻微漂移。
- 平铺布局为 `auto-fit` 网格，未还原原始窗口位置/尺寸。
- 录制中关闭主窗口=直接退出，session.json 无 endedAt（不影响回放，但列表时长显示会以「现在」估算）。

## 未实现功能（TODO）

按优先级粗略排序：

1. **运行时测试与修 bug**——最高优先级，当前仅编译通过。先跑通基本流程（开始录制 -> 开关子窗口 -> 停止 -> 回放），再逐项修。
2. **位置精确回放**：录制时记窗口 x/y/w/h，回放按包围盒缩放 fit 视口、按原位置摆放，还原多窗口空间关系。
3. **无漂移同步**：用主 RAF 循环按帧 `goto` 驱动各 segment，取代各 Replayer 独立 `play`。
4. **录制元信息编辑**：重命名、备注（需扩展 session.json 与列表 UI）。
5. **导出**：会话导出为单个 `.json` 文件分享/导入。
6. **console / 网络错误采集**：rrweb 的 `@rrweb/record` 插件，作为补充事件流。
7. **seek 准确性验证**：确认 `pause(offset)` 行为，必要时改用 `play(offset)`+`pause()` 组合。
8. **进度条精度与性能**：当前 50ms tick 更新 `currentTime`，长录制下可考虑节流。

未来可选：音频录制、屏幕流（超出 rrweb DOM 录制范畴，需额外方案）。

## 扩展指引

- **新增录制配置**（采样、遮罩等）：改 `useRecorder.ts` 的 `record({ emit, ...options })` 调用。注意：当前未做隐私遮罩（用户明确不需要）。
- **新增会话级元数据**：扩展 `session.json` 写入字段 + `list_sessions`/`read_session` 读取。
- **新增窗口生命周期事件**：在 `on_window_event` 增 match 分支，`append_lifecycle` 落 windows.jsonl；回放侧 `usePlayer` 解析新 type。
- **新增 Rust 命令**：在 `lib.rs` 加 `#[tauri::command]` 并注册到 `generate_handler![]`；自定义命令无需改 capabilities。
- **新增带新 label 模式的路由**：同步改 [src-tauri/capabilities/default.json](src-tauri/capabilities/default.json) 的 `windows` glob，否则窗口缺权限。

## 约定

- UI 文案中文（zh-CN），与现有一致。
- 前端自动导入已开：`ref`/`reactive`/`ElMessage` 等无需手动 import（`.vue` 文件中）；`.ts` composable 内可显式 import vue API 以求清晰。
- 类型声明 `src/auto-imports.d.ts`、`src/components.d.ts` 为构建产物，勿手改。
- Rust crates 走 rsproxy 镜像，勿删 `.cargo/config.toml`。
