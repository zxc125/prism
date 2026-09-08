# web-demo — 被观测样例应用

已接入 [`@prism-obs/observer-sdk`](https://www.npmjs.com/package/@prism-obs/observer-sdk) 的最小 Web 应用，两个用途：

1. **零代码跑通 Prism**：配合 console 验证「上报 → 会话 → 回放」全链路（见[快速开始](../../site/docs/quickstart.md)）。
2. **录制量控对比载体**（P14 场景 6）：内置行情模拟页，同页对比 full / balanced / minimal 三档 + 免录区的录制效果。

## 跑法

```sh
# 仓库根目录（示例依赖 workspace 里的 SDK，须先 install）
pnpm install

# 起 console（另开终端）：pnpm tauri dev

# 起本示例
pnpm dev:web-demo
```

浏览器打开 **http://localhost:1422**（固定端口）。

- 上报目标硬编码为 `http://127.0.0.1:1421`（`src/main.ts` 的 `ENDPOINT`，端口见 console 设置页，需要时改这里）。demo 的 `init()` 默认未传 `token`；console 开启鉴权时，请在 `init({ ... })` 里自行补 `token` 字段。
- **✅ 跑通的标志**：页面右上角状态徽标变绿显示「采集中」；console 会话浏览器出现 appId 为 `web-demo` 的会话。

## URL 参数

| 参数 | 取值 | 作用 |
| --- | --- | --- |
| `?sim=1` | — | 启动行情模拟：`.quote-table` 每 100ms 随机刷新，制造高频 DOM mutation |
| `?profile=` | `full`（缺省） / `balanced` / `minimal` | 录制档位 |
| `?block=` | CSS selector，逗号分隔多个 | 免录区，如 `.quote-table,.ad-banner` |

## 建议的对比实验

开两个浏览器窗口，边操作边回 console 看会话体积与回放流畅度：

```text
窗口 A（全量基线）：  http://localhost:1422/?sim=1
窗口 B（量控配置）：  http://localhost:1422/?sim=1&profile=minimal&block=.quote-table
```

预期：B 的高频行情 mutation 事件几乎消失（免录区生效），交互事件被 minimal 收紧；回放中行情表格显示为占位块。三档差异详见 [Web SDK · 录制量控](../../site/docs/web.md#录制量控-recording)。
