<script setup lang="ts">
import { ref, inject, computed, watch } from "vue";
import { PLAYER_CTX, type PlayerCtx } from "./context";
import type { Signal } from "../../composables/usePlayer";

/** 诊断侧栏：信号流 + 标注。可折叠，与时间轴共享播放头。 */
const ctx = inject<PlayerCtx>(PLAYER_CTX);
if (!ctx) throw new Error("DiagnosisPanel 必须在 PlayerShell 内使用");
const { player, annos } = ctx;

const diagTab = ref<"signal" | "note">("signal");
const signalFilter = ref<"all" | "console" | "network" | "error">("all");
const annoDraft = ref("");
const streamRef = ref<HTMLElement>();

const filterOptions = [
  { key: "all", label: "全部" },
  { key: "console", label: "console" },
  { key: "network", label: "network" },
  { key: "error", label: "error" },
] as const;

const SIG_COLOR: Record<Signal["plugin"], string> = {
  console: "var(--sig-log)",
  network: "var(--sig-net)",
  error: "var(--sig-err)",
};

function sigTimecode(ms: number) {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  const p = (n: number, l = 2) => n.toString().padStart(l, "0");
  return `+${p(m)}:${p(s % 60)}.${p(ms % 1000, 3)}`;
}

const filteredSignals = computed(() =>
  signalFilter.value === "all"
    ? player.signals.value
    : player.signals.value.filter((s) => s.plugin === signalFilter.value),
);

const activeSigIdx = computed(() => {
  const t = player.currentTime.value;
  let idx = -1;
  for (let i = 0; i < filteredSignals.value.length; i++) {
    if (filteredSignals.value[i].t <= t) idx = i;
    else break;
  }
  return idx;
});

function sigTag(s: Signal): string {
  if (s.plugin === "console") return s.payload.level;
  if (s.plugin === "network") return s.payload.kind;
  return s.payload.kind;
}
function formatArg(a: unknown): string {
  if (a === null) return "null";
  if (typeof a === "object") {
    try {
      return JSON.stringify(a);
    } catch {
      return String(a);
    }
  }
  return String(a);
}
function sigText(s: Signal): string {
  if (s.plugin === "console") return s.payload.args.map(formatArg).join(" ");
  if (s.plugin === "network")
    return `${s.payload.method} ${s.payload.url}  ${s.payload.status}  ${s.payload.duration}ms`;
  return s.payload.message;
}

function onSignalClick(t: number) {
  player.seek(t);
}

function addAnno() {
  const text = annoDraft.value.trim();
  if (!text) return;
  annos.add({
    t: player.currentTime.value,
    label: player.mainLabel.value ?? undefined,
    text,
    author: "local",
  });
  annoDraft.value = "";
}

function onAnnoClick(t: number) {
  player.seek(t);
}

function removeAnno(annoId: string) {
  annos.remove(annoId);
}

const activeAnnoId = computed(() => {
  const t = player.currentTime.value;
  let best: string | null = null;
  let bestDt = 250;
  for (const a of annos.annotations.value) {
    const dt = Math.abs(a.t - t);
    if (dt < bestDt) {
      bestDt = dt;
      best = a.id;
    }
  }
  return best;
});

watch(activeSigIdx, (idx) => {
  if (idx < 0 || !streamRef.value) return;
  const el = streamRef.value.querySelector<HTMLElement>(`[data-idx="${idx}"]`);
  el?.scrollIntoView({ block: "nearest" });
});
</script>

<template>
  <aside class="diagnosis">
    <header class="diag-head">
      <div class="diag-tabs">
        <button
          class="diag-tab mono"
          :class="{ 'is-active': diagTab === 'signal' }"
          @click="diagTab = 'signal'"
        >
          信号
        </button>
        <button
          class="diag-tab mono"
          :class="{ 'is-active': diagTab === 'note' }"
          @click="diagTab = 'note'"
        >
          标注<span v-if="annos.annotations.value.length" class="tab-count">
            {{ annos.annotations.value.length }}
          </span>
        </button>
      </div>
      <el-select
        v-if="diagTab === 'signal'"
        v-model="signalFilter"
        size="small"
        class="diag-filter"
      >
        <el-option v-for="o in filterOptions" :key="o.key" :label="o.label" :value="o.key" />
      </el-select>
    </header>

    <div v-show="diagTab === 'signal'" ref="streamRef" class="signal-stream">
      <div
        v-for="(s, i) in filteredSignals"
        :key="i"
        :data-idx="i"
        class="sig-row"
        :class="{ 'is-active': i === activeSigIdx }"
        @click="onSignalClick(s.t)"
      >
        <span class="sig-tc mono">{{ sigTimecode(s.t) }}</span>
        <span class="sig-tag mono" :style="{ '--c': SIG_COLOR[s.plugin] }">{{ sigTag(s) }}</span>
        <span class="sig-text" :title="sigText(s)">{{ sigText(s) }}</span>
      </div>
      <div v-if="!filteredSignals.length" class="sig-empty">无信号</div>
    </div>

    <div v-show="diagTab === 'note'" class="anno-pane">
      <div class="anno-add">
        <span class="anno-tc mono">{{ sigTimecode(player.currentTime.value) }}</span>
        <el-input
          v-model="annoDraft"
          size="small"
          placeholder="标注此处…"
          @keydown.enter="addAnno"
        />
        <el-button size="small" type="primary" @click="addAnno">标记</el-button>
      </div>
      <div class="anno-list">
        <div
          v-for="a in annos.annotations.value"
          :key="a.id"
          class="anno-row"
          :class="{ 'is-active': a.id === activeAnnoId }"
          @click="onAnnoClick(a.t)"
        >
          <span class="anno-tc mono">{{ sigTimecode(a.t) }}</span>
          <span class="anno-text" :title="a.text">{{ a.text }}</span>
          <button class="anno-del" aria-label="删除标注" @click.stop="removeAnno(a.id)">×</button>
        </div>
        <div v-if="!annos.annotations.value.length" class="sig-empty">
          无标注 · 移动到任意位置，输入文本后标记
        </div>
      </div>
    </div>
  </aside>
</template>

<style scoped>
.diagnosis {
  border-left: 1px solid var(--hair);
  background: var(--slate);
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}
.diag-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--hair);
  background: var(--slate-2);
}
.diag-filter { margin-left: auto; width: 110px; }
.diag-tabs { display: flex; gap: 2px; }
.diag-tab {
  appearance: none;
  border: 0;
  background: transparent;
  color: var(--ash);
  font-size: var(--fs-xs);
  padding: 4px 8px;
  cursor: pointer;
  letter-spacing: 0.06em;
  border-radius: var(--radius-sm);
  transition: color 0.12s, background 0.12s;
}
.diag-tab:hover { color: var(--bone-dim); }
.diag-tab.is-active { color: var(--bone); background: var(--slate-3); }
.tab-count { margin-left: 4px; color: var(--amber); }
.signal-stream { flex: 1; overflow-y: auto; padding: 4px 0; }
.sig-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px 12px;
  cursor: pointer;
  border-left: 2px solid transparent;
  transition: background 0.12s;
}
.sig-row:hover { background: var(--slate-2); }
.sig-row.is-active {
  background: var(--amber-tint);
  border-left-color: var(--amber);
}
.sig-tc { font-size: var(--fs-xs); color: var(--ash); flex-shrink: 0; padding-top: 1px; }
.sig-tag {
  font-size: 10px;
  color: var(--c);
  flex-shrink: 0;
  padding-top: 1px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  min-width: 44px;
}
.sig-text {
  font-size: var(--fs-xs);
  color: var(--bone-dim);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.sig-row.is-active .sig-text { color: var(--bone); }
.sig-empty {
  color: var(--ash-deep);
  font-size: var(--fs-xs);
  padding: 24px;
  text-align: center;
}
.anno-pane { flex: 1; display: flex; flex-direction: column; min-height: 0; }
.anno-add {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--hair);
  background: var(--slate-2);
}
.anno-add .anno-tc { font-size: var(--fs-xs); color: var(--amber); flex-shrink: 0; }
.anno-add :deep(.el-input) { flex: 1; min-width: 0; }
.anno-list { flex: 1; overflow-y: auto; padding: 4px 0; }
.anno-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px 12px;
  cursor: pointer;
  border-left: 2px solid transparent;
  transition: background 0.12s;
}
.anno-row:hover { background: var(--slate-2); }
.anno-row.is-active {
  background: var(--amber-tint);
  border-left-color: var(--amber);
}
.anno-row .anno-tc { font-size: var(--fs-xs); color: var(--ash); flex-shrink: 0; padding-top: 1px; }
.anno-row.is-active .anno-tc { color: var(--amber); }
.anno-text {
  font-size: var(--fs-xs);
  color: var(--bone-dim);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
}
.anno-row.is-active .anno-text { color: var(--bone); }
.anno-del {
  appearance: none;
  border: 0;
  background: transparent;
  color: var(--ash-deep);
  font-size: 15px;
  line-height: 1;
  cursor: pointer;
  padding: 0 2px;
  flex-shrink: 0;
  opacity: 0;
  transition: opacity 0.12s, color 0.12s;
}
.anno-row:hover .anno-del { opacity: 1; }
.anno-del:hover { color: var(--oxblood-soft); }
</style>
