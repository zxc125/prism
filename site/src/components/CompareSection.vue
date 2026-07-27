<script setup lang="ts">
// 竞品对比 · 对标矩阵（方案 §6.8 第 9 节 · §5.1）
// 签名元素：对标表，我方列琥珀高亮 + ✓/✗ 色编码
import { Check, X } from "lucide-vue-next";
import Reveal from "./Reveal.vue";

const products = [
  "鉴 / Prism",
  "FullStory / LogRocket",
  "Sentry Replay",
  "Highlight.io",
  "PostHog",
  "rrweb",
];

type Cell = { text: string; kind: "yes" | "no" | "text"; ours?: boolean };
const rows: { dim: string; cells: Cell[] }[] = [
  {
    dim: "默认形态",
    cells: [
      { text: "本地 / 自托管", kind: "text", ours: true },
      { text: "云 SaaS", kind: "text" },
      { text: "云", kind: "text" },
      { text: "云优先", kind: "text" },
      { text: "云", kind: "text" },
      { text: "库", kind: "text" },
    ],
  },
  {
    dim: "数据主权",
    cells: [
      { text: "完全自有", kind: "yes", ours: true },
      { text: "厂商托管", kind: "no" },
      { text: "厂商托管", kind: "no" },
      { text: "厂商托管", kind: "no" },
      { text: "厂商托管", kind: "no" },
      { text: "自有", kind: "yes" },
    ],
  },
  {
    dim: "计费",
    cells: [
      { text: "无 / 自托管成本", kind: "yes", ours: true },
      { text: "per-session", kind: "no" },
      { text: "套餐内", kind: "no" },
      { text: "per-session", kind: "no" },
      { text: "per-session", kind: "no" },
      { text: "免费", kind: "yes" },
    ],
  },
  {
    dim: "多窗口对齐",
    cells: [
      { text: "独家", kind: "yes", ours: true },
      { text: "", kind: "no" },
      { text: "", kind: "no" },
      { text: "", kind: "no" },
      { text: "", kind: "no" },
      { text: "", kind: "no" },
    ],
  },
  {
    dim: "诊断信号交错",
    cells: [
      { text: "同一时间轴", kind: "yes", ours: true },
      { text: "独立面板", kind: "no" },
      { text: "偏 error", kind: "no" },
      { text: "独立面板", kind: "no" },
      { text: "独立面板", kind: "no" },
      { text: "需自建", kind: "no" },
    ],
  },
  {
    dim: "定位",
    cells: [
      { text: "诊断工具", kind: "text", ours: true },
      { text: "RUM / 分析", kind: "text" },
      { text: "错误追踪", kind: "text" },
      { text: "RUM", kind: "text" },
      { text: "产品分析", kind: "text" },
      { text: "录回放库", kind: "text" },
    ],
  },
  {
    dim: "告警 / RUM",
    cells: [
      { text: "不做", kind: "text", ours: true },
      { text: "做", kind: "text" },
      { text: "做", kind: "text" },
      { text: "做", kind: "text" },
      { text: "做", kind: "text" },
      { text: "", kind: "no" },
    ],
  },
  {
    dim: "桌面 App",
    cells: [
      { text: "有", kind: "yes", ours: true },
      { text: "", kind: "no" },
      { text: "", kind: "no" },
      { text: "", kind: "no" },
      { text: "", kind: "no" },
      { text: "", kind: "no" },
    ],
  },
  {
    dim: "开放数据契约",
    cells: [
      { text: "有", kind: "yes", ours: true },
      { text: "", kind: "no" },
      { text: "", kind: "no" },
      { text: "部分", kind: "text" },
      { text: "", kind: "no" },
      { text: "N/A", kind: "text" },
    ],
  },
];
</script>

<template>
  <section id="compare" class="section">
    <div class="section-inner">
      <Reveal>
        <header class="section-head">
          <p class="eyebrow mono">竞品对比 · 对标矩阵</p>
          <h2 class="section-h2">
            同行做 RUM，<br />
            我们做<span class="accent-amber">诊断闭环</span>。
          </h2>
          <p class="section-sub">
            不是功能多寡的对比，是定位的分野。云 RUM 把数据搬走按 session 收费；鉴 / Prism 把数据留下，只做诊断这一件事。
          </p>
        </header>
      </Reveal>

      <Reveal>
        <div class="cmp-scroll">
          <table class="cmp-table">
            <thead>
              <tr>
                <th class="dim-col">维度</th>
                <th
                  v-for="(p, i) in products"
                  :key="p"
                  class="prod-col"
                  :class="{ ours: i === 0 }"
                >
                  {{ p }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in rows" :key="row.dim">
                <td class="dim-cell mono">{{ row.dim }}</td>
                <td
                  v-for="(c, i) in row.cells"
                  :key="i"
                  class="cell"
                  :class="{ ours: c.ours }"
                >
                  <span class="cell-inner" :class="`kind-${c.kind}`">
                    <Check
                      v-if="c.kind === 'yes'"
                      :size="13"
                      class="mark mark-yes"
                    />
                    <X
                      v-else-if="c.kind === 'no' && !c.text"
                      :size="13"
                      class="mark mark-no"
                    />
                    <span class="cell-text">{{ c.text }}</span>
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </Reveal>
    </div>
  </section>
</template>

<style scoped>
.cmp-scroll {
  overflow-x: auto;
  border: 1px solid var(--color-hair);
  border-radius: 10px;
  background: var(--color-slate);
}
.cmp-table {
  width: 100%;
  border-collapse: collapse;
  min-width: 760px;
}
.cmp-table th,
.cmp-table td {
  padding: 0.875rem 1rem;
  text-align: left;
  border-bottom: 1px solid var(--color-hair);
  vertical-align: middle;
}
.cmp-table thead th {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--color-ash);
  background: var(--color-ink-2);
  letter-spacing: 0.02em;
  position: sticky;
  top: 0;
}
.cmp-table th.dim-col {
  width: 130px;
}
.cmp-table th.prod-col.ours {
  color: var(--color-amber);
  background: rgba(240, 168, 61, 0.08);
  border-left: 1px solid rgba(240, 168, 61, 0.25);
  border-right: 1px solid rgba(240, 168, 61, 0.25);
}
.cmp-table tbody tr:last-child td {
  border-bottom: none;
}
.dim-cell {
  font-size: 0.8125rem;
  color: var(--color-ash);
  letter-spacing: 0.02em;
}
.cell.ours {
  background: rgba(240, 168, 61, 0.06);
  border-left: 1px solid rgba(240, 168, 61, 0.25);
  border-right: 1px solid rgba(240, 168, 61, 0.25);
}
.cell-inner {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.8125rem;
}
.cell-text {
  color: var(--color-bone-dim);
}
.kind-yes .cell-text {
  color: var(--color-bone);
}
.cell.ours .kind-yes .cell-text {
  color: var(--color-amber);
}
.kind-no .cell-text {
  color: var(--color-ash-deep);
}
.mark {
  flex-shrink: 0;
}
.mark-yes {
  color: var(--color-teal);
}
.cell.ours .mark-yes {
  color: var(--color-amber);
}
.mark-no {
  color: var(--color-ash-deep);
  opacity: 0.5;
}
</style>
