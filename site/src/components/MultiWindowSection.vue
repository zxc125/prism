<script setup lang="ts">
// 独家：多窗口对齐（方案 §6.8 第 5 节）
// 签名元素：多轨窗口生命周期时间轴 - 3 个窗口 lane（shown/hidden 段）+ 信号轨 + 共享播放头
// 对齐产品真实机制：shown/hidden 区间在主时间轴同步驱动各 segment
import { ref, onMounted, onUnmounted } from "vue";
import { Play, Pause } from "lucide-vue-next";
import Reveal from "./Reveal.vue";

interface WinSeg {
  label: string;
  start: number;
  end: number;
  color: "amber" | "teal" | "oxblood";
}
const windows: WinSeg[] = [
  { label: "主窗口", start: 0, end: 5, color: "amber" },
  { label: "设置面板", start: 1.2, end: 3.8, color: "teal" },
  { label: "确认模态", start: 2.5, end: 4.2, color: "oxblood" },
];

interface Sig {
  t: number;
  kind: "network" | "console" | "error";
}
const signals: Sig[] = [
  { t: 0.4, kind: "network" },
  { t: 1.5, kind: "console" },
  { t: 2.1, kind: "network" },
  { t: 2.8, kind: "error" },
  { t: 3.3, kind: "console" },
  { t: 3.9, kind: "network" },
  { t: 4.5, kind: "error" },
];

const SESSION_MAX = 5;
const SWEEP_MS = 12000;
const currentT = ref(0);
const playing = ref(true);
const reduced = ref(false);
let raf = 0;
let last = 0;

onMounted(() => {
  reduced.value = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (reduced.value) {
    currentT.value = 2.8;
    playing.value = false;
    return;
  }
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
});
onUnmounted(() => cancelAnimationFrame(raf));

function fmt(s: number): string {
  const c = Math.max(0, s);
  return `00:00:${String(Math.floor(c)).padStart(2, "0")}.${String(
    Math.floor((c % 1) * 1000),
  ).padStart(3, "0")}`;
}
function isActive(seg: WinSeg): boolean {
  return currentT.value >= seg.start && currentT.value <= seg.end;
}
function scrubTo(ev: MouseEvent) {
  const rail = ev.currentTarget as HTMLElement;
  const rect = rail.getBoundingClientRect();
  // 轨道区在 label + gap 之后，scrub 比例相对轨道区而非整条 rail
  const labelW = window.innerWidth <= 640 ? 48 + 12 : 64 + 16;
  const trackW = Math.max(1, rect.width - labelW);
  const ratio = (ev.clientX - rect.left - labelW) / trackW;
  currentT.value = Math.max(0, Math.min(SESSION_MAX, ratio * SESSION_MAX));
  playing.value = false;
}
function togglePlay() {
  playing.value = !playing.value;
}
</script>

<template>
  <section id="multiwindow" class="section mw">
    <div class="section-inner">
      <Reveal>
        <header class="section-head">
          <p class="eyebrow mono">独家 · 多窗口对齐录制</p>
          <h2 class="section-h2">
            三扇窗口，<br />
            <span class="accent-amber">同一面墙上时钟</span>。
          </h2>
          <p class="section-sub">
            Tauri 桌面应用多窗口各自一个 rrweb 录制实例，事件带绝对 timestamp 共享时钟。回放时按 shown/hidden 区间在主时间轴同步驱动各 segment--主窗口、设置面板、模态框，三轨对齐，别家做不到。
          </p>
        </header>
      </Reveal>

      <Reveal>
        <div class="mw-stage">
          <div class="stage-rail" @click="scrubTo">
            <!-- 窗口生命周期 lanes -->
            <div
              v-for="(w, i) in windows"
              :key="i"
              class="rail-lane"
            >
              <span class="lane-label mono">{{ w.label }}</span>
              <div class="lane-track">
                <div
                  class="lane-seg"
                  :class="[`seg-${w.color}`, { active: isActive(w) }]"
                  :style="{
                    left: (w.start / SESSION_MAX) * 100 + '%',
                    width: ((w.end - w.start) / SESSION_MAX) * 100 + '%',
                  }"
                />
              </div>
            </div>

            <!-- 信号轨 -->
            <div class="rail-lane rail-sig">
              <span class="lane-label mono">SIGNAL</span>
              <div class="lane-track">
                <span
                  v-for="(s, i) in signals"
                  :key="i"
                  class="sig-dot"
                  :class="`sig-${s.kind}`"
                  :style="{ left: (s.t / SESSION_MAX) * 100 + '%' }"
                />
              </div>
            </div>

            <!-- 共享播放头 - left 由 --p 计算，对齐 label 偏移后的轨道区 -->
            <div
              class="rail-playhead"
              :style="{ '--p': currentT / SESSION_MAX }"
            >
              <span class="ph-glow" />
            </div>
          </div>

          <div class="stage-ruler mono">
            <span>00:00:00</span>
            <span>00:00:01</span>
            <span>00:00:02</span>
            <span>00:00:03</span>
            <span>00:00:04</span>
            <span>00:00:05</span>
          </div>

          <div class="stage-ctrl">
            <button
              class="ctrl-play"
              @click="togglePlay"
              :aria-label="playing ? '暂停' : '播放'"
            >
              <component :is="playing ? Pause : Play" :size="13" />
            </button>
            <span class="ctrl-tc mono">{{ fmt(currentT) }}</span>
            <span class="ctrl-hint mono">点击轨道跳转 · 多窗口共享播放头</span>
          </div>

          <p class="stage-foot mono">
            传统回放：单窗口单流 · 鉴 / Prism：shown/hidden 区间同步驱动，多轨对齐
          </p>
        </div>
      </Reveal>
    </div>
  </section>
</template>

<style scoped>
.mw {
  background:
    radial-gradient(
      ellipse 55% 40% at 70% 30%,
      rgba(77, 208, 200, 0.03),
      transparent
    ),
    var(--color-ink);
}
.mw-stage {
  background: var(--color-slate);
  border: 1px solid var(--color-hair);
  border-radius: 10px;
  padding: 2.5rem 1.75rem 1.5rem;
}
.stage-rail {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  cursor: pointer;
  padding: 0.5rem 0 1rem;
}
.rail-lane {
  position: relative;
  display: flex;
  align-items: center;
  gap: 1rem;
}
.lane-label {
  width: 64px;
  flex-shrink: 0;
  font-size: 0.6875rem;
  letter-spacing: 0.1em;
  color: var(--color-ash-deep);
}
.lane-track {
  position: relative;
  flex: 1;
  height: 14px;
  background: var(--color-ink-2);
  border-radius: 3px;
  border: 1px solid var(--color-hair);
}
.lane-seg {
  position: absolute;
  top: 1px;
  bottom: 1px;
  border-radius: 2px;
  opacity: 0.55;
  transition: opacity 0.15s;
}
.lane-seg.active {
  opacity: 1;
}
.seg-amber {
  background: var(--color-amber);
}
.seg-amber.active {
  box-shadow: 0 0 10px rgba(240, 168, 61, 0.6);
}
.seg-teal {
  background: var(--color-teal);
}
.seg-teal.active {
  box-shadow: 0 0 10px rgba(77, 208, 200, 0.6);
}
.seg-oxblood {
  background: var(--color-oxblood);
}
.seg-oxblood.active {
  box-shadow: 0 0 10px rgba(229, 72, 77, 0.6);
}
.rail-sig .lane-track {
  height: 8px;
  background: transparent;
  border: none;
}
.sig-dot {
  position: absolute;
  top: 50%;
  transform: translate(-50%, -50%);
  border-radius: 50%;
}
.sig-network {
  width: 6px;
  height: 6px;
  background: var(--color-teal);
  box-shadow: 0 0 5px rgba(77, 208, 200, 0.5);
}
.sig-console {
  width: 5px;
  height: 5px;
  background: var(--color-ash);
}
.sig-error {
  width: 7px;
  height: 7px;
  background: var(--color-oxblood);
  box-shadow: 0 0 6px rgba(229, 72, 77, 0.6);
  border-radius: 1px;
}

.rail-playhead {
  position: absolute;
  top: 0;
  bottom: 1rem;
  /* label(64px) + gap(1rem) + p × 轨道宽(100% - 64px - 1rem) */
  left: calc(64px + 1rem + var(--p, 0) * (100% - 64px - 1rem));
  width: 2px;
  background: var(--color-amber);
  box-shadow:
    0 0 8px rgba(240, 168, 61, 0.7),
    0 0 16px rgba(240, 168, 61, 0.3);
  transform: translateX(-1px);
  pointer-events: none;
}
.ph-glow {
  position: absolute;
  top: -5px;
  left: 50%;
  transform: translateX(-50%);
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--color-amber);
  box-shadow: 0 0 12px rgba(240, 168, 61, 0.9);
}

.stage-ruler {
  display: flex;
  justify-content: space-between;
  margin-left: calc(64px + 1rem);
  padding-top: 0.75rem;
  border-top: 1px solid var(--color-hair);
  font-size: 0.6875rem;
  color: var(--color-ash-deep);
}
.stage-ctrl {
  display: flex;
  align-items: center;
  gap: 1rem;
  margin-top: 1.25rem;
  margin-left: calc(64px + 1rem);
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
.ctrl-hint {
  font-size: 0.6875rem;
  color: var(--color-ash-deep);
}
.stage-foot {
  margin: 1.75rem 0 0;
  font-size: 0.8125rem;
  color: var(--color-ash-deep);
  text-align: center;
}

@media (max-width: 640px) {
  .lane-label {
    width: 48px;
    font-size: 0.625rem;
  }
  .stage-ruler,
  .stage-ctrl {
    margin-left: calc(48px + 0.75rem);
  }
  .rail-playhead {
    left: calc(48px + 0.75rem + var(--p, 0) * (100% - 48px - 0.75rem));
  }
}
</style>
