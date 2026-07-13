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
| 主窗口点「开始录制」 | `start_session`：置 active、建目录、写 session.json、补记初始 focus、广播 `recording-session{active:true}` | 各窗口 `useRecorder` 收到广播 -> `invoke("begin_segment")` -> 启动 rrweb |
| 新窗口首次创建 | `open_window` build | 新窗口 `useRecorder` 挂载时 `is_recording_active` 兜底为 true -> `begin_segment` |
| 已隐藏窗口再次打开 | `open_window`：`show()`+`focus()`，录制中则 `emit_to(label,"segment",{start})` | `useRecorder` 收到 start -> `begin_segment` 开新段 |
| 子窗口点 X 关闭（录制中） | `on_window_event` CloseRequested：`prevent_close`+`hide`、记 hidden、`emit_to` segment:stop | `useRecorder` 收到 stop -> 调 rrweb stop fn、flush |
| 窗口聚焦 | `on_window_event` Focused：记 focus（跳过 `player-*`） | - |
| 主窗口关闭 | 不拦截 -> 退出进程 | - |
| 点「停止录制」 | `stop_session`：置 inactive、关闭所有活跃段、广播 `recording-session{active:false}`、写 endedAt | 各窗口停录 |

### 关键设计决策与原因

- **hash 模式路由 + `index.html#{route}`**：新窗口靠 hash 路由定位视图，Tauri 走自定义协议/本地文件，history 模式刷新会 404。
- **segment 而非整窗单文件**：子窗口「关闭=隐藏、再开重录」导致同一窗口多次活跃；每段独立快照才能正确回放每次显示的初始状态。
- **绝对时间戳对齐**：所有窗口共享墙上时钟，rrweb 事件自带 `timestamp`，无需额外同步协议。主窗口 click 与子窗口 shown 落在同一时间轴，「连接感」自然成立。
- **子窗口关闭改隐藏**：保留 webview 与 Vue 状态，`useRecorder` 实例不销毁，靠 Rust 定向事件驱动 start/stop 段。
- **player-* 窗口跳过录制**：回放窗口本身不能被录进会话，否则污染数据。
- **底层 `Replayer` 而非 `rrweb-player`**：后者是 Vue 2 组件，不能用于 Vue 3；自建 transport（自定义播放按钮 + Element Plus slider/select/switch）+ 平铺网格槽位，磁贴内 rrweb 内容保留白底以还原被录页面。
- **JSONL 格式**：可流式追加、断电不丢整文件、回放按行 parse。
- **稳定槽位 + spotlight 主窗口**：回放时每个 label 占一个固定网格槽位（不随 show/hide reflow）；主窗口由 focus 时间线自动跟踪或手动选择，占大格，其余为侧槽。主槽/侧槽切换只改 CSS class（`is-main` + `grid-row:1/-1`），**不 reparent** Replayer 容器——rrweb 用 iframe 承载 mirror，reparent 会触发重新加载。
- **动态 tile 元素不可用 scoped CSS**：`usePlayer` 用 `document.createElement` 创建 `.tile-slot/.tile-header/.tile-root/.tile-placeholder`，不带 scoped 的 `data-v` 属性，`<style scoped>` 选择器全部失配（曾导致 spotlight 不跨行 + 横向溢出）。相关规则放 PlayerView 的非 scoped `<style>` 块（以 `.grid` 限定作用域），并配 `min-width:0` + `minmax(0,...)` 列模板防 rrweb 原始宽度撑爆网格。
- **tile 等比缩放（方案 E）**：`.replayer-wrapper` 固定为录制视口尺寸（Meta 事件的 width/height），`transform: translate() scale()` fit-contain 到 `.tile-root` 并居中、`transform-origin: top left`；`ResizeObserver` 监听各 tile-root，spotlight 切主 / 窗口缩放 / 段显隐时重算。
- **漂移阈值纠偏**：各 Replayer 独立 RAF 与主时钟（`setInterval(50)` + `performance.now()`）可能漂移；`tick` 内读 `replayer.getCurrentTime()` 与期望值对比，超 `DRIFT_THRESHOLD`(120ms) 则 `replayer.play(expect)` re-seek 拉回。rrweb 2.x `play(offset)` 内部先 PAUSE 再 PLAY(offset)，seek+续播一次完成。
- **focus 时间线**：`windows.jsonl` 的 focus 事件驱动自动主窗口；`start_session` 时补记初始 focus（遍历 `webview_windows().is_focused()`），避免 t=0 时间线为空；player-* 窗口的 focus 已过滤，不产生孤儿记录。

### 文件职责

| 文件 | 职责 |
| --- | --- |
| [src-tauri/src/lib.rs](src-tauri/src/lib.rs) | `Session` 状态、录制相关命令、`open_window`、`on_window_event` 生命周期拦截 |
| [src/composables/useRecorder.ts](src/composables/useRecorder.ts) | 每窗口录制器：监听会话广播与段事件、缓冲 flush、`player-*` 跳过 |
| [src/composables/usePlayer.ts](src/composables/usePlayer.ts) | 回放控制器：加载会话、按区间驱动各 `Replayer`、play/pause/seek/倍速、稳定槽位、spotlight 主窗口（auto focus + 手动）、漂移纠偏、时间轴色带数据、tile 等比缩放（`fitSegment` + `ResizeObserver`）；导出 `LANE_COLORS` 并为每个槽位标 `--lane-color`（磁贴头圆点与色带共用） |
| [src/App.vue](src/App.vue) | 挂载 `useRecorder`，使每个窗口都参与录制 |
| [src/styles/theme.css](src/styles/theme.css) | 全局设计系统：warm-dark 控制台色板（琥珀 `--amber` / 牛血 `--oxblood` / 等宽时间码 `--font-mono`）、Element Plus 全量深色变量覆写（`:root:root` 提权）、组件微调 |
| [src/views/MainView.vue](src/views/MainView.vue) | 控制轨 + 会话日志双栏：硬件式 REC 控件（胶片倒计环 + 实时 `T+` 计时 + 录制脉冲点）、开始/停止录制、会话列表（回放/删除） |
| [src/views/PlayerView.vue](src/views/PlayerView.vue) | 平铺编辑器布局：spotlight 回放网格 + 自建 transport（播放按钮 + 时间轴色带 + focus 标记 + playhead 三角 + 倍速 + 跟随焦点） |

### Rust 命令清单

`greet`、`open_window`、`start_session`、`stop_session`、`is_recording_active`、`begin_segment`、`append_events`、`list_sessions`、`read_session`、`delete_session`。自定义 command 已在 `generate_handler![]` 注册，无需在 capabilities 授权；`listen` 由 `core:default` 允许。

## 当前现状

### 已实现

- 多窗口录制：主窗口 + 任意子窗口，会话期间新开的窗口自动接入。
- 子窗口关闭=隐藏、再开重录（分段），主窗口关闭=退出。
- 事件流式落盘（每秒 flush 一次）。
- 会话列表、删除。
- 多窗口回放：播放/暂停/进度拖动/倍速，按 shown/hidden 区间切换可见 segment。
- 稳定槽位布局：每个 label 一个固定网格槽位，show/hide 不 reflow；隐藏窗口显示占位。
- spotlight 主窗口：focus 时间线自动跟踪 + 手动点击选主 + 自动跟随开关；主槽占大格、侧槽堆叠，CSS 切换不 reparent。
- 时间轴色带：主进度条上方叠加每窗口活跃区间色带 + focus 切换标记 + playhead，支持点击 seek。
- tile 等比缩放（方案 E）：`.replayer-wrapper` 固定为录制视口尺寸（Meta 事件的 width/height），`transform: scale()` fit-contain 到 tile-root 并居中；spotlight 切主 / 窗口缩放 / 段显隐由 `ResizeObserver` 自动重算。
- 漂移阈值纠偏：各 Replayer 与主时钟偏差超 120ms 时 re-seek 对齐。
- focus 数据补强：会话开始补记初始 focus、过滤 player-* focus 事件。

### 编译验证

- `cargo check`（src-tauri 内）通过。
- `pnpm exec vue-tsc --noEmit` 通过。
- `pnpm build`（含 vite 构建）通过。

### 运行时实测

已跑 `pnpm tauri dev` 实测确认（P0+P1 修复后）：

- ✅ spotlight 主槽/侧槽布局（`grid-row:1/-1` + 显式行数）、主窗口自动跟踪与手动切换；横向溢出已消除、播放按钮不再被滚出视口。
- ✅ tile 等比缩放（方案 E）生效，spotlight 主区放大 / 窗口缩放自适应。

仍待实测确认：

- 漂移纠偏的实际收敛效果、`play(offset)` 在长事件流上 re-seek 的延迟。
- 时间轴色带 / 焦点标记的渲染与点击 seek（P0 修复横向滚动后预期正常，未单独复核）。
- 录制中关闭子窗口的拦截时序、`emit_to` 定向事件是否被目标窗口稳定接收。

### 已知 MVP 限制

- 回放各 segment 仍各自跑独立 RAF，靠 120ms 阈值纠偏拉回（有界，非零漂移）；彻底零漂移需主时钟步进（方案 A，未做）。
- 回放布局为稳定槽位 + spotlight，但**未还原原始窗口位置/尺寸**（方案 C，未做）。
- 录制中关闭主窗口=直接退出，session.json 无 endedAt（不影响回放，但列表时长显示会以「现在」估算）。

## 未实现功能（TODO）

按优先级粗略排序：

1. **运行时测试与修 bug**：基本流程（开始录制 -> 开关子窗口 -> 停止 -> 回放）+ spotlight 布局 + 等比缩放（方案 E）已实测通过；仍待验证漂移纠偏收敛、色带点击 seek、录制中关闭子窗口时序（见「运行时实测」）。
2. **位置精确回放（方案 C）**：录制时记窗口 x/y/w/h（`outer_position`/`outer_size`），回放按包围盒缩放 fit 视口、按原位置摆放，还原多窗口空间关系。
3. **无漂移同步（方案 A）**：主 RAF 循环每帧 `replayer.pause(offset)` 步进驱动各 segment，取代独立 `play`，彻底零漂移。当前已落地阈值纠偏（方案 B，120ms re-seek），A 为可选增强。
4. **录制元信息编辑**：重命名、备注（需扩展 session.json 与列表 UI）。
5. **导出**：会话导出为单个 `.json` 文件分享/导入。
6. **console / 网络错误采集**：rrweb 的 `@rrweb/record` 插件，作为补充事件流。
7. **进度条精度与性能**：当前 50ms tick 更新 `currentTime`，长录制下可考虑节流或换 `requestAnimationFrame`。

未来可选：音频录制、屏幕流（超出 rrweb DOM 录制范畴，需额外方案）。

## 扩展指引

- **新增录制配置**（采样、遮罩等）：改 `useRecorder.ts` 的 `record({ emit, ...options })` 调用。注意：当前未做隐私遮罩（用户明确不需要）。
- **新增会话级元数据**：扩展 `session.json` 写入字段 + `list_sessions`/`read_session` 读取。
- **新增窗口生命周期事件**：在 `on_window_event` 增 match 分支，`append_lifecycle` 落 windows.jsonl；回放侧 `usePlayer` 解析新 type。
- **新增 Rust 命令**：在 `lib.rs` 加 `#[tauri::command]` 并注册到 `generate_handler![]`；自定义命令无需改 capabilities。
- **新增带新 label 模式的路由**：同步改 [src-tauri/capabilities/default.json](src-tauri/capabilities/default.json) 的 `windows` glob，否则窗口缺权限。
- **新增窗口轨道色**：在 `usePlayer.ts` 的 `LANE_COLORS` 数组追加色值，时间轴色带与对应磁贴头圆点同时生效；勿在视图里写临时色值。

## 约定

- UI 文案中文（zh-CN），与现有一致。
- 前端自动导入已开：`ref`/`reactive`/`ElMessage` 等无需手动 import（`.vue` 文件中）；`.ts` composable 内可显式 import vue API 以求清晰。
- 设计系统见 [src/styles/theme.css](src/styles/theme.css)：warm-dark 控制台主题，色板/字号/间距以 CSS 变量定义（`--ink`/`--slate`/`--amber`/`--oxblood`/`--font-mono` 等），Element Plus 经 `:root:root` 变量覆写为深色。视图用平铺布局，勿再用 `el-card shadow="hover"` 包裹；新色值走既有 token，勿临时写死。
- 类型声明 `src/auto-imports.d.ts`、`src/components.d.ts` 为构建产物，勿手改。
- Rust crates 走 rsproxy 镜像，勿删 `.cargo/config.toml`。
