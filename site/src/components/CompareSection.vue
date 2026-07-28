<script setup lang="ts">
// 竞品对比 · 对标矩阵（方案 §6.8 第 9 节 · §5.1）
// 签名元素：对标表，我方列琥珀高亮 + ✓/✗ 色编码
import { computed } from "vue";
import { Check, X } from "lucide-vue-next";
import Reveal from "./Reveal.vue";
import { useLang } from "../composables/useLang";

const { t } = useLang();

const products = computed(() => [
  t("compare.products.p1"),
  "FullStory / LogRocket",
  "Sentry Replay",
  "Highlight.io",
  "PostHog",
  "rrweb",
]);

type Cell = { text: string; kind: "yes" | "no" | "text"; ours?: boolean };
const rows = computed(() => {
  const c = (key: string, kind: Cell["kind"], ours?: boolean): Cell => ({
    text: t(key),
    kind,
    ours,
  });
  return [
    {
      dim: t("compare.rows.form.cells.0"),
      cells: [
        c("compare.rows.form.cells.1", "text", true),
        c("compare.rows.form.cells.2", "text"),
        c("compare.rows.form.cells.3", "text"),
        c("compare.rows.form.cells.4", "text"),
        c("compare.rows.form.cells.5", "text"),
        c("compare.rows.form.cells.6", "text"),
      ],
    },
    {
      dim: t("compare.rows.sovereignty.cells.0"),
      cells: [
        c("compare.rows.sovereignty.cells.1", "yes", true),
        c("compare.rows.sovereignty.cells.2", "no"),
        c("compare.rows.sovereignty.cells.3", "no"),
        c("compare.rows.sovereignty.cells.4", "no"),
        c("compare.rows.sovereignty.cells.5", "no"),
        c("compare.rows.sovereignty.cells.6", "yes"),
      ],
    },
    {
      dim: t("compare.rows.pricing.cells.0"),
      cells: [
        c("compare.rows.pricing.cells.1", "yes", true),
        c("compare.rows.pricing.cells.2", "no"),
        c("compare.rows.pricing.cells.3", "no"),
        c("compare.rows.pricing.cells.4", "no"),
        c("compare.rows.pricing.cells.5", "no"),
        c("compare.rows.pricing.cells.6", "yes"),
      ],
    },
    {
      dim: t("compare.rows.multiwindow.cells.0"),
      cells: [
        c("compare.rows.multiwindow.cells.1", "yes", true),
        c("compare.rows.multiwindow.cells.2", "no"),
        c("compare.rows.multiwindow.cells.3", "no"),
        c("compare.rows.multiwindow.cells.4", "no"),
        c("compare.rows.multiwindow.cells.5", "no"),
        c("compare.rows.multiwindow.cells.6", "no"),
      ],
    },
    {
      dim: t("compare.rows.diagnosis.cells.0"),
      cells: [
        c("compare.rows.diagnosis.cells.1", "yes", true),
        c("compare.rows.diagnosis.cells.2", "no"),
        c("compare.rows.diagnosis.cells.3", "no"),
        c("compare.rows.diagnosis.cells.4", "no"),
        c("compare.rows.diagnosis.cells.5", "no"),
        c("compare.rows.diagnosis.cells.6", "no"),
      ],
    },
    {
      dim: t("compare.rows.positioning.cells.0"),
      cells: [
        c("compare.rows.positioning.cells.1", "text", true),
        c("compare.rows.positioning.cells.2", "text"),
        c("compare.rows.positioning.cells.3", "text"),
        c("compare.rows.positioning.cells.4", "text"),
        c("compare.rows.positioning.cells.5", "text"),
        c("compare.rows.positioning.cells.6", "text"),
      ],
    },
    {
      dim: t("compare.rows.alerts.cells.0"),
      cells: [
        c("compare.rows.alerts.cells.1", "text", true),
        c("compare.rows.alerts.cells.2", "text"),
        c("compare.rows.alerts.cells.3", "text"),
        c("compare.rows.alerts.cells.4", "text"),
        c("compare.rows.alerts.cells.5", "text"),
        c("compare.rows.alerts.cells.6", "no"),
      ],
    },
    {
      dim: t("compare.rows.desktop.cells.0"),
      cells: [
        c("compare.rows.desktop.cells.1", "yes", true),
        c("compare.rows.desktop.cells.2", "no"),
        c("compare.rows.desktop.cells.3", "no"),
        c("compare.rows.desktop.cells.4", "no"),
        c("compare.rows.desktop.cells.5", "no"),
        c("compare.rows.desktop.cells.6", "no"),
      ],
    },
    {
      dim: t("compare.rows.contract.cells.0"),
      cells: [
        c("compare.rows.contract.cells.1", "yes", true),
        c("compare.rows.contract.cells.2", "no"),
        c("compare.rows.contract.cells.3", "no"),
        c("compare.rows.contract.cells.4", "text"),
        c("compare.rows.contract.cells.5", "no"),
        c("compare.rows.contract.cells.6", "text"),
      ],
    },
  ] as { dim: string; cells: Cell[] }[];
});
</script>

<template>
  <section id="compare" class="section">
    <div class="section-inner">
      <Reveal>
        <header class="section-head">
          <p class="eyebrow mono">{{ t("compare.eyebrow") }}</p>
          <h2 class="section-h2">
            {{ t("compare.h2_pre") }}<br />
            {{ t("compare.h2_mid") }}<span class="accent-amber">{{ t("compare.h2_accent") }}</span>{{ t("compare.h2_suf") }}
          </h2>
          <p class="section-sub">
            {{ t("compare.sub") }}
          </p>
        </header>
      </Reveal>

      <Reveal>
        <div class="cmp-scroll">
          <table class="cmp-table">
            <thead>
              <tr>
                <th class="dim-col">{{ t("compare.dimCol") }}</th>
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
