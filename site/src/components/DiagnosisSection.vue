<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { Play, Pause } from "lucide-vue-next";

type EventType = "dom" | "network" | "console" | "error";
interface TimelineEvent {
  t: number;
  type: EventType;
  level?: "log" | "warn" | "error";
  status?: number;
  label: string;
  detail?: string;
}

// 一条真实故障故事：2.9s 点提交 -> 3.1s 500 -> 3.14s console.error -> 3.18s 未捕获异常
const events: TimelineEvent[] = [
  { t: 0.05, type: "dom", label: "DOMContentLoaded" },
  { t: 0.4, type: "network", status: 200, label: "GET /api/user", detail: "18ms" },
  { t: 0.8, type: "dom", label: "render <Dashboard />" },
  { t: 1.2, type: "console", level: "log", label: "dashboard mounted" },
  { t: 1.6, type: "network", status: 200, label: "GET /api/orders", detail: "142ms" },
  { t: 2.1, type: "dom", label: "render <OrderList />" },
  { t: 2.5, type: "console", level: "warn", label: "14 orders missing id" },
  { t: 2.9, type: "dom", label: 'click <button>提交订单</button>' },
  { t: 3.1, type: "network", status: 500, label: "POST /api/order", detail: "89ms" },
  { t: 3.14, type: "console", level: "error", label: 'TypeError: Cannot read "id" of undefined' },
  { t: 3.18, type: "error", label: "Uncaught TypeError", detail: "order.ts:42" },
  { t: 3.6, type: "dom", label: "render <ErrorToast />" },
  { t: 4.2, type: "console", level: "log", label: "error toast shown" },
];

const SESSION_MAX = 5;
const SWEEP_MS = 14000; // 14s 扫完 0-5s，慢到故障时刻有停留
const WINDOW = 0.35; // ±0.35s 内的事件进当前帧
const currentT = ref(3.14);
const playing = ref(true);
const reduced = ref(false);

onMounted(() => {
  reduced.value = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (reduced.value) {
    currentT.value = 3.14;
    playing.value = false;
    return;
  }
  let raf = 0;
  let last = 0;
  const step = (now: number) => {
    if (playing.value) {
      if (last > 0) {
        const dt = (now - last) / 1000;
        let next = currentT.value + (dt * SESSION_MAX) / (SWEEP_MS / 1000);
        if (next > SESSION_MAX) next = 0;
        currentT.value = next;
      }
      last = now;
    } else {
      last = 0;
    }
    raf = requestAnimationFrame(step);
  };
  raf = requestAnimationFrame(step);
  onUnmounted(() => cancelAnimationFrame(raf));
});

function eventColor(e: TimelineEvent): string {
  if (e.type === "dom") return "amber";
  if (e.type === "network") return e.status && e.status >= 400 ? "oxblood" : "teal";
  if (e.type === "console") {
    if (e.level === "error") return "oxblood";
    if (e.level === "warn") return "amber";
    return "ash";
  }
  return "oxblood";
}

function tickHeight(e: TimelineEvent): number {
  if (e.type === "error") return 38;
  if (e.type === "network") return 28;
  if (e.type === "dom") return 20;
  return 16;
}

function typeLabel(e: TimelineEvent): string {
  if (e.type === "dom") return "DOM";
  if (e.type === "network") return "NETWORK";
  if (e.type === "console") return "CONSOLE";
  return "ERROR";
}

function formatTc(s: number): string {
  const clamped = Math.max(0, s);
  const ms = Math.floor((clamped % 1) * 1000);
  const sec = Math.floor(clamped);
  return `00:00:${String(sec).padStart(2, "0")}.${String(ms).padStart(3, "0")}`;
}

function inFrame(e: TimelineEvent): boolean {
  return Math.abs(e.t - currentT.value) <= WINDOW;
}

const frameEvents = computed(() =>
  events
    .filter((e) => Math.abs(e.t - currentT.value) <= WINDOW)
    .sort((a, b) => a.t - b.t),
);

function jumpTo(t: number) {
  currentT.value = t;
  playing.value = false;
}

function scrubTo(ev: MouseEvent) {
  const track = ev.currentTarget as HTMLElement;
  const rect = track.getBoundingClientRect();
  const ratio = (ev.clientX - rect.left) / rect.width;
  currentT.value = Math.max(0, Math.min(SESSION_MAX, ratio * SESSION_MAX));
  playing.value = false;
}

function togglePlay() {
  playing.value = !playing.value;
}
</script>

<template>
  <section id="diagnosis" class="diagnosis">
    <div class="diag-inner">
      <header class="diag-head">
        <p class="eyebrow mono">诊断 · 交错事件模型</p>
        <h2 class="diag-h2">
          error / console / network，<br />
          交错在 <span class="accent-amber">DOM 同一条时间轴</span>。
        </h2>
        <p class="diag-sub">
          回放到第 3 秒，同时看到页面、那条 console.error、那个 500。传统 RUM
          把它们散在「网络」「日志」「错误」三个 tab--鉴 / Prism 让它们在同一条轴上自我说明。
        </p>
      </header>

      <div class="diag-stage">
        <!-- 单轴交错时间轴 -->
        <div class="timeline">
          <div class="tl-axis" @click="scrubTo">
            <div class="tl-line" />
            <button
              v-for="(e, i) in events"
              :key="i"
              class="tl-tick"
              :class="[`tick-${eventColor(e)}`, { active: inFrame(e) }]"
              :style="{
                left: (e.t / SESSION_MAX) * 100 + '%',
                height: tickHeight(e) + 'px',
              }"
              :title="`${formatTc(e.t)} · ${e.label}`"
              @click.stop="jumpTo(e.t)"
            />
            <div
              class="tl-playhead"
              :style="{ left: (currentT / SESSION_MAX) * 100 + '%' }"
            >
              <span class="ph-glow" />
            </div>
          </div>
          <div class="tl-ruler mono">
            <span>00:00:00</span>
            <span>00:00:01</span>
            <span>00:00:02</span>
            <span>00:00:03</span>
            <span>00:00:04</span>
            <span>00:00:05</span>
          </div>
          <div class="tl-controls">
            <button
              class="ctrl-play"
              @click="togglePlay"
              :aria-label="playing ? '暂停' : '播放'"
            >
              <component :is="playing ? Pause : Play" :size="13" />
            </button>
            <span class="ctrl-tc mono">{{ formatTc(currentT) }}</span>
            <div class="legend mono">
              <span class="leg leg-amber">DOM</span>
              <span class="leg leg-teal">network</span>
              <span class="leg leg-ash">console</span>
              <span class="leg leg-oxblood">error</span>
            </div>
          </div>
        </div>

        <!-- 当前帧面板：同一时刻多类信号并陈 -->
        <div class="frame">
          <div class="frame-head">
            <span class="frame-title mono">当前帧</span>
            <span class="frame-tc mono">{{ formatTc(currentT) }}</span>
            <span class="frame-window mono">±{{ WINDOW.toFixed(2) }}s 窗口</span>
          </div>
          <div class="frame-body">
            <div v-if="frameEvents.length === 0" class="frame-empty mono">
              等待事件进入窗口…
            </div>
            <div
              v-for="(e, i) in frameEvents"
              :key="i"
              class="frame-row"
              :class="`row-${eventColor(e)}`"
            >
              <span class="row-type mono">{{ typeLabel(e) }}</span>
              <span class="row-t mono">{{ formatTc(e.t) }}</span>
              <span class="row-label mono">{{ e.label }}</span>
              <span v-if="e.detail" class="row-detail mono">{{ e.detail }}</span>
            </div>
          </div>
        </div>
      </div>

      <p class="diag-foot mono">
        传统 RUM：三个 tab 来回跳，对不上时间 · 鉴 / Prism：同一条轴，自我说明
      </p>
    </div>
  </section>
</template>

<style scoped>
.diagnosis {
  position: relative;
  padding: 6rem 2rem 6.5rem;
  background:
    radial-gradient(ellipse 60% 40% at 50% 30%, rgba(240, 168, 61, 0.025), transparent),
    var(--color-ink);
  border-top: 1px solid var(--color-hair);
}
.diag-inner {
  max-width: 1180px;
  margin: 0 auto;
}

/* head */
.diag-head {
  max-width: 46rem;
  margin-bottom: 3.5rem;
}
.eyebrow {
  font-size: 0.75rem;
  letter-spacing: 0.18em;
  color: var(--color-ash);
  text-transform: uppercase;
  margin: 0 0 1.25rem;
}
.diag-h2 {
  font-size: clamp(1.875rem, 4vw, 2.75rem);
  line-height: 1.12;
  letter-spacing: -0.02em;
  font-weight: 700;
  color: var(--color-bone);
  margin: 0 0 1.25rem;
}
.accent-amber { color: var(--color-amber); }
.diag-sub {
  font-size: 1.0625rem;
  line-height: 1.65;
  color: var(--color-ash);
  margin: 0;
}

/* stage */
.diag-stage {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

/* timeline */
.timeline {
  background: var(--color-slate);
  border: 1px solid var(--color-hair);
  border-radius: 10px;
  padding: 2.5rem 1.75rem 1.5rem;
}
.tl-axis {
  position: relative;
  height: 56px;
  cursor: pointer;
}
.tl-line {
  position: absolute;
  top: 50%;
  left: 0;
  right: 0;
  height: 1px;
  background: var(--color-hair);
  transform: translateY(-0.5px);
}
.tl-tick {
  position: absolute;
  bottom: 50%;
  width: 2px;
  border: none;
  border-radius: 1px;
  padding: 0;
  cursor: pointer;
  opacity: 0.5;
  transform: translate(-1px, 50%);
  transition: opacity 0.15s;
}
.tl-tick:hover { opacity: 0.85; }
.tl-tick.active { opacity: 1; }
.tick-amber { background: var(--color-amber); }
.tick-amber.active { box-shadow: 0 0 8px rgba(240, 168, 61, 0.7); }
.tick-teal { background: var(--color-teal); }
.tick-teal.active { box-shadow: 0 0 8px rgba(77, 208, 200, 0.7); }
.tick-ash { background: var(--color-ash); }
.tick-ash.active { box-shadow: 0 0 6px rgba(139, 148, 163, 0.6); }
.tick-oxblood { background: var(--color-oxblood); }
.tick-oxblood.active { box-shadow: 0 0 9px rgba(229, 72, 77, 0.75); }

.tl-playhead {
  position: absolute;
  top: -8px;
  bottom: -8px;
  width: 2px;
  background: var(--color-amber);
  box-shadow: 0 0 10px rgba(240, 168, 61, 0.8), 0 0 20px rgba(240, 168, 61, 0.35);
  transform: translateX(-1px);
  pointer-events: none;
}
.ph-glow {
  position: absolute;
  top: -4px;
  left: 50%;
  transform: translateX(-50%);
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--color-amber);
  box-shadow: 0 0 12px rgba(240, 168, 61, 0.9);
}

.tl-ruler {
  display: flex;
  justify-content: space-between;
  margin-top: 1.25rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--color-hair);
  font-size: 0.6875rem;
  color: var(--color-ash-deep);
}

.tl-controls {
  display: flex;
  align-items: center;
  gap: 1rem;
  margin-top: 1.25rem;
}
.ctrl-play {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  border: 1px solid var(--color-hair);
  background: var(--color-slate-2);
  color: var(--color-bone);
  cursor: pointer;
  transition: all 0.15s;
}
.ctrl-play:hover {
  border-color: var(--color-amber);
  color: var(--color-amber);
}
.ctrl-tc {
  font-size: 0.8125rem;
  color: var(--color-amber);
  min-width: 7.5rem;
}
.legend {
  margin-left: auto;
  display: flex;
  gap: 1.25rem;
  font-size: 0.6875rem;
  letter-spacing: 0.08em;
  color: var(--color-ash);
}
.leg::before {
  content: "";
  display: inline-block;
  width: 8px;
  height: 8px;
  margin-right: 0.4rem;
  vertical-align: middle;
  border-radius: 1px;
}
.leg-amber::before { background: var(--color-amber); }
.leg-teal::before { background: var(--color-teal); }
.leg-ash::before { background: var(--color-ash); }
.leg-oxblood::before { background: var(--color-oxblood); }

/* frame panel */
.frame {
  background: var(--color-slate);
  border: 1px solid var(--color-hair);
  border-radius: 10px;
  overflow: hidden;
}
.frame-head {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.875rem 1.25rem;
  border-bottom: 1px solid var(--color-hair);
  background: var(--color-ink-2);
}
.frame-title {
  font-size: 0.75rem;
  letter-spacing: 0.12em;
  color: var(--color-ash);
  text-transform: uppercase;
}
.frame-tc {
  font-size: 0.9375rem;
  color: var(--color-amber);
}
.frame-window {
  margin-left: auto;
  font-size: 0.6875rem;
  color: var(--color-ash-deep);
}
.frame-body {
  padding: 0.5rem 0.75rem;
  min-height: 4rem;
}
.frame-empty {
  padding: 1.5rem 0.75rem;
  text-align: center;
  font-size: 0.8125rem;
  color: var(--color-ash-deep);
}
.frame-row {
  display: flex;
  align-items: baseline;
  gap: 0.875rem;
  padding: 0.625rem 0.875rem;
  border-left: 2px solid;
  border-radius: 0 4px 4px 0;
  margin: 0.125rem 0;
}
.row-amber {
  border-color: var(--color-amber);
  background: rgba(240, 168, 61, 0.06);
}
.row-teal {
  border-color: var(--color-teal);
  background: rgba(77, 208, 200, 0.06);
}
.row-ash {
  border-color: var(--color-ash);
  background: rgba(139, 148, 163, 0.05);
}
.row-oxblood {
  border-color: var(--color-oxblood);
  background: rgba(229, 72, 77, 0.07);
}
.row-type {
  width: 64px;
  flex-shrink: 0;
  font-size: 0.6875rem;
  letter-spacing: 0.08em;
  font-weight: 600;
}
.row-amber .row-type { color: var(--color-amber); }
.row-teal .row-type { color: var(--color-teal); }
.row-ash .row-type { color: var(--color-ash); }
.row-oxblood .row-type { color: var(--color-oxblood); }
.row-t {
  width: 92px;
  flex-shrink: 0;
  font-size: 0.75rem;
  color: var(--color-ash);
}
.row-label {
  flex: 1;
  font-size: 0.8125rem;
  color: var(--color-bone);
  word-break: break-word;
}
.row-detail {
  flex-shrink: 0;
  font-size: 0.75rem;
  color: var(--color-ash-deep);
}

/* foot */
.diag-foot {
  margin: 2.5rem 0 0;
  font-size: 0.8125rem;
  color: var(--color-ash-deep);
  text-align: center;
  letter-spacing: 0.02em;
}

.mono { font-family: var(--font-mono); }
</style>
