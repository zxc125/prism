<script setup lang="ts">
import "rrweb/dist/style.css";
import { useRoute } from "vue-router";
import { usePlayer, LANE_COLORS, type Signal } from "../composables/usePlayer";

const route = useRoute();
const id = route.params.id as string;

const gridRef = ref<HTMLElement>();
const player = usePlayer(id);

const speeds = [0.5, 1, 2, 4];

const diagOpen = ref(true);
const signalFilter = ref<"all" | "console" | "network" | "error">("all");

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

const streamRef = ref<HTMLElement>();

function fmt(ms: number) {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  const r = s % 60;
  const p = (n: number) => n.toString().padStart(2, "0");
  return `${p(m)}:${p(r)}`;
}

// 信号时间码：相对会话起点 +MM:SS.mmm
function sigTimecode(ms: number) {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  const p = (n: number, l = 2) => n.toString().padStart(l, "0");
  return `+${p(m)}:${p(s % 60)}.${p(ms % 1000, 3)}`;
}

function onSlider(v: number | number[]) {
  const ms = Array.isArray(v) ? v[0] : v;
  player.seek(ms);
}

const palette = LANE_COLORS;

function pct(ms: number) {
  const total = player.totalTime.value;
  return total > 0 ? (ms / total) * 100 : 0;
}

function bandStyle(b: { labelIdx: number; start: number; end: number }) {
  return {
    left: `${pct(b.start)}%`,
    width: `${Math.max(0, pct(b.end - b.start))}%`,
    top: `${b.labelIdx * 8}px`,
    background: palette[b.labelIdx % palette.length],
  };
}

function onOverviewClick(e: MouseEvent) {
  const el = e.currentTarget as HTMLElement;
  const rect = el.getBoundingClientRect();
  const ratio = Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width));
  player.seek(ratio * player.totalTime.value);
}

const filteredSignals = computed(() =>
  signalFilter.value === "all"
    ? player.signals.value
    : player.signals.value.filter((s) => s.plugin === signalFilter.value),
);

// 当前播放头对应的信号索引（最后一个 t <= currentTime），用于高亮 + 自动滚动
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

// 活跃信号行滚入视口
watch(activeSigIdx, (idx) => {
  if (idx < 0 || !streamRef.value) return;
  const el = streamRef.value.querySelector<HTMLElement>(`[data-idx="${idx}"]`);
  el?.scrollIntoView({ block: "nearest" });
});

onMounted(async () => {
  try {
    await player.load();
    if (gridRef.value) player.attachGrid(gridRef.value);
  } catch (e) {
    ElMessage.error(`加载录制失败: ${e}`);
  }
});

onBeforeUnmount(() => player.destroy());
</script>

<template>
  <main class="player" :class="{ 'diag-collapsed': !diagOpen }">
    <header class="bar">
      <div class="bar-id">
        <span class="eyebrow">回放</span>
        <span class="bar-sess mono">{{ id }}</span>
      </div>
      <div class="bar-meta mono">
        <span>{{ player.timeline.value.labels.length }} 窗口</span>
        <span class="dot-sep">·</span>
        <span>{{ fmt(player.totalTime.value) }}</span>
        <span v-if="player.errorMarks.value.length" class="dot-sep">·</span>
        <span v-if="player.errorMarks.value.length" class="bar-err">
          ⚠{{ player.errorMarks.value.length }}
        </span>
      </div>
      <button
        class="bar-toggle mono"
        :class="{ 'is-on': diagOpen }"
        @click="diagOpen = !diagOpen"
      >
        诊断
      </button>
    </header>

    <div v-if="!player.ready.value" v-loading="true" class="loading" />

    <template v-else>
      <div class="body">
        <div ref="gridRef" class="grid" />

        <aside class="diagnosis">
          <header class="diag-head">
            <span class="eyebrow">诊断 · signal</span>
            <el-select
              v-model="signalFilter"
              size="small"
              class="diag-filter"
            >
              <el-option
                v-for="o in filterOptions"
                :key="o.key"
                :label="o.label"
                :value="o.key"
              />
            </el-select>
          </header>
          <div ref="streamRef" class="signal-stream">
            <div
              v-for="(s, i) in filteredSignals"
              :key="i"
              :data-idx="i"
              class="sig-row"
              :class="{ 'is-active': i === activeSigIdx }"
              @click="onSignalClick(s.t)"
            >
              <span class="sig-tc mono">{{ sigTimecode(s.t) }}</span>
              <span
                class="sig-tag mono"
                :style="{ '--c': SIG_COLOR[s.plugin] }"
                >{{ sigTag(s) }}</span
              >
              <span class="sig-text" :title="sigText(s)">{{ sigText(s) }}</span>
            </div>
            <div v-if="!filteredSignals.length" class="sig-empty">无信号</div>
          </div>
        </aside>
      </div>

      <footer class="transport">
        <button
          class="transport-play"
          :class="{ 'is-playing': player.playing.value }"
          :aria-label="player.playing.value ? '暂停' : '播放'"
          @click="player.playing.value ? player.pause() : player.play()"
        >
          <svg
            v-if="!player.playing.value"
            viewBox="0 0 24 24"
            width="16"
            height="16"
            fill="currentColor"
          >
            <path d="M8 5v14l11-7z" />
          </svg>
          <svg
            v-else
            viewBox="0 0 24 24"
            width="16"
            height="16"
            fill="currentColor"
          >
            <rect x="6" y="5" width="4" height="14" rx="1" />
            <rect x="14" y="5" width="4" height="14" rx="1" />
          </svg>
        </button>

        <div class="timeline-col">
          <div
            class="timeline-overview"
            :style="{
              height: player.timeline.value.labels.length * 8 + 4 + 'px',
            }"
            @click="onOverviewClick"
          >
            <div
              v-for="(b, i) in player.timeline.value.bands"
              :key="'b' + i"
              class="tl-band"
              :style="bandStyle(b)"
            />
            <div
              v-for="(t, i) in player.errorMarks.value"
              :key="'e' + i"
              class="tl-err"
              :style="{ left: pct(t) + '%' }"
            />
            <div
              v-for="(t, i) in player.timeline.value.focusMarks"
              :key="'f' + i"
              class="tl-focus"
              :style="{ left: pct(t) + '%' }"
            />
            <div
              class="tl-playhead"
              :style="{ left: pct(player.currentTime.value) + '%' }"
            >
              <span class="tl-head" />
            </div>
          </div>

          <el-slider
            class="slider"
            :min="0"
            :max="player.totalTime.value"
            :model-value="player.currentTime.value"
            :step="100"
            :format-tooltip="fmt"
            @input="onSlider"
          />
        </div>

        <span class="timecode mono">
          {{ fmt(player.currentTime.value)
          }}<span class="tc-sep">/</span>{{ fmt(player.totalTime.value) }}
        </span>

        <el-select
          :model-value="player.speed.value"
          class="speed"
          @update:model-value="(v) => player.setSpeed(v as number)"
        >
          <el-option
            v-for="s in speeds"
            :key="s"
            :label="s + 'x'"
            :value="s"
          />
        </el-select>

        <span class="sep" />

        <label class="follow">
          <el-switch
            :model-value="player.autoFollow.value"
            @update:model-value="(v) => player.setAutoFollow(v as boolean)"
          />
          <span>跟随焦点</span>
        </label>
      </footer>
    </template>
  </main>
</template>

<style scoped>
.player {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--ink);
}

/* ---- top bar ---- */
.bar {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--hair);
  background: var(--slate);
}
.bar-id {
  display: flex;
  align-items: baseline;
  gap: 10px;
}
.bar-sess {
  color: var(--bone);
  font-size: var(--fs-md);
  letter-spacing: 0.04em;
}
.bar-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--ash);
  font-size: var(--fs-xs);
  margin-left: auto;
}
.bar-err {
  color: var(--oxblood-soft);
}
.dot-sep {
  color: var(--ash-deep);
}
.bar-toggle {
  appearance: none;
  border: 1px solid var(--hair);
  background: transparent;
  color: var(--ash);
  font-size: var(--fs-xs);
  padding: 4px 11px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  letter-spacing: 0.06em;
  transition: color 0.15s, border-color 0.15s, background 0.15s;
}
.bar-toggle:hover {
  color: var(--bone-dim);
  border-color: var(--ash-deep);
}
.bar-toggle.is-on {
  color: var(--ink);
  background: var(--amber);
  border-color: var(--amber);
}

.loading {
  flex: 1;
}

/* ---- body: replay grid + diagnosis rail ---- */
.body {
  flex: 1;
  display: grid;
  grid-template-columns: minmax(0, 1fr) 340px;
  min-height: 0;
}
.player.diag-collapsed .body {
  grid-template-columns: minmax(0, 1fr);
}
.player.diag-collapsed .diagnosis {
  display: none;
}

/* ---- tile grid (slots rendered by usePlayer) ---- */
.grid {
  min-height: 0;
  display: grid;
  gap: 8px;
  padding: 12px;
  background: var(--ink);
}
/* tile-slot / tile-header / tile-root / tile-placeholder 由 usePlayer 用
   document.createElement 动态创建，不带 scoped 的 data-v 属性，scoped 选择器
   不命中。样式见文件末尾的非 scoped 块（以 .grid 限定作用域）。*/

/* ---- diagnosis rail ---- */
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
.diag-filter {
  margin-left: auto;
  width: 110px;
}
.signal-stream {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}
.sig-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px 12px;
  cursor: pointer;
  border-left: 2px solid transparent;
  transition: background 0.12s;
}
.sig-row:hover {
  background: var(--slate-2);
}
.sig-row.is-active {
  background: var(--amber-tint);
  border-left-color: var(--amber);
}
.sig-tc {
  font-size: var(--fs-xs);
  color: var(--ash);
  flex-shrink: 0;
  padding-top: 1px;
}
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
.sig-row.is-active .sig-text {
  color: var(--bone);
}
.sig-empty {
  color: var(--ash-deep);
  font-size: var(--fs-xs);
  padding: 24px;
  text-align: center;
}

/* ---- transport ---- */
.transport {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 12px 16px;
  border-top: 1px solid var(--hair);
  background: var(--slate);
}
.transport-play {
  width: 38px;
  height: 38px;
  border-radius: 50%;
  border: 1px solid var(--amber);
  background: var(--amber);
  color: var(--ink);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  flex-shrink: 0;
  transition: background 0.15s, transform 0.1s;
}
.transport-play:hover {
  background: var(--amber-soft);
}
.transport-play:active {
  transform: scale(0.94);
}
.transport-play.is-playing {
  background: transparent;
  color: var(--amber);
}

.timeline-col {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}
.timeline-overview {
  position: relative;
  background: var(--hair-soft);
  border-radius: var(--radius-sm);
  cursor: pointer;
  padding: 2px 0;
}
.tl-band {
  position: absolute;
  height: 6px;
  border-radius: 2px;
  opacity: 0.9;
  pointer-events: none;
}
.tl-err {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 2px;
  background: var(--oxblood-soft);
  opacity: 0.9;
  pointer-events: none;
  transform: translateX(-1px);
}
.tl-focus {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 1px;
  background: var(--bone);
  opacity: 0.32;
  pointer-events: none;
}
.tl-playhead {
  position: absolute;
  top: -2px;
  bottom: -2px;
  width: 2px;
  background: var(--amber);
  pointer-events: none;
  transform: translateX(-1px);
}
.tl-head {
  position: absolute;
  top: -5px;
  left: 50%;
  transform: translateX(-50%);
  width: 0;
  height: 0;
  border-left: 5px solid transparent;
  border-right: 5px solid transparent;
  border-top: 5px solid var(--amber);
}

.slider {
  width: 100%;
}
.slider :deep(.el-slider__runway) {
  margin: 8px 0;
}

.timecode {
  font-size: var(--fs-sm);
  color: var(--bone);
  white-space: nowrap;
  letter-spacing: 0.06em;
}
.tc-sep {
  color: var(--ash-deep);
  margin: 0 4px;
}

.speed {
  width: 84px;
}

.sep {
  width: 1px;
  height: 22px;
  background: var(--hair);
}

.follow {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: var(--fs-xs);
  color: var(--ash);
  cursor: pointer;
  white-space: nowrap;
}
.follow:hover {
  color: var(--bone-dim);
}
</style>

<style>
/* tile 元素由 usePlayer 用 document.createElement 动态创建，不带 scoped 的
   data-v 属性，故这些规则放在非 scoped 块里；以 .grid 限定作用域避免泄漏。
   min-width:0 + minmax(0,...) 列模板防止 rrweb 原始宽度撑爆网格列。 */
.grid .tile-slot {
  display: flex;
  flex-direction: column;
  background: var(--slate);
  border: 1px solid var(--hair);
  border-radius: var(--radius);
  overflow: hidden;
  min-width: 0;
  min-height: 0;
}
.grid .tile-slot.is-main {
  grid-column: 1;
  grid-row: 1 / -1;
  border-color: var(--amber);
  box-shadow: 0 0 0 1px var(--amber-deep) inset,
    0 0 28px rgba(232, 163, 61, 0.08);
}
.grid .tile-slot:not(.is-main) {
  grid-column: 2;
}
.grid .tile-slot.is-empty {
  background: repeating-linear-gradient(
    135deg,
    var(--slate) 0 10px,
    #1b1712 10px 20px
  );
  border-style: dashed;
  border-color: var(--hair-soft);
}
.grid .tile-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 10px;
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
  letter-spacing: 0.06em;
  color: var(--bone-dim);
  background: var(--slate-2);
  border-bottom: 1px solid var(--hair);
  cursor: pointer;
  user-select: none;
}
.grid .tile-header::before {
  content: "";
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--lane-color, var(--ash));
  flex-shrink: 0;
}
.grid .tile-slot.is-main .tile-header {
  color: var(--bone);
  background: var(--amber-tint);
}
.grid .tile-slot.is-main .tile-header::after {
  content: "主";
  margin-left: auto;
  font-size: 10px;
  letter-spacing: 0.1em;
  color: var(--ink);
  background: var(--amber);
  padding: 1px 6px;
  border-radius: 2px;
  font-weight: 600;
}
.grid .tile-placeholder {
  display: none;
  flex: 1;
  align-items: center;
  justify-content: center;
  color: var(--ash-deep);
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
  letter-spacing: 0.1em;
}
.grid .tile-slot.is-empty .tile-placeholder {
  display: flex;
}
.grid .tile-root {
  flex: 1;
  overflow: hidden;
  background: #fff;
}
/* .replayer-wrapper 的 width/height/transform 由 usePlayer.fitSegment 按录制
   视口尺寸等比缩放设置，此处不覆写。 */
</style>
