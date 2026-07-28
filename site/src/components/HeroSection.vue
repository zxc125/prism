<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import PrismLogo from "./PrismLogo.vue";
import Aurora from "./Aurora.vue";

// DOM 轨事件（rrweb snapshot/incremental）
const domEvents = [
  { t: 0.08, kind: "snap" },
  { t: 0.22, kind: "incr" },
  { t: 0.35, kind: "incr" },
  { t: 0.5, kind: "incr" },
  { t: 0.68, kind: "snap" },
  { t: 0.82, kind: "incr" },
  { t: 0.92, kind: "incr" },
];
// 信号轨事件（type:6 交错：network/console/error）
const signalEvents = [
  { t: 0.05, kind: "network" },
  { t: 0.15, kind: "console" },
  { t: 0.28, kind: "network" },
  { t: 0.38, kind: "error" },
  { t: 0.44, kind: "console" },
  { t: 0.58, kind: "network" },
  { t: 0.72, kind: "error" },
  { t: 0.85, kind: "console" },
  { t: 0.95, kind: "network" },
];

function dotClass(kind: string) {
  return `dot dot-${kind}`;
}

// 播放头时间码 - rAF 同步横扫循环（session 0~4s 映射到 7s 真实扫程）
const SWEEP_MS = 7000;
const SESSION_MAX = 4; // 秒，对应 ruler 00:00:00 -> 00:00:04
const tc = ref("00:00:00.000");
const reduced = ref(false);

onMounted(() => {
  reduced.value = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (reduced.value) {
    tc.value = "00:00:01.520";
    return;
  }
  const start = performance.now();
  let raf = 0;
  const tick = (now: number) => {
    const p = ((now - start) % SWEEP_MS) / SWEEP_MS;
    const s = p * SESSION_MAX;
    tc.value = formatTc(s);
    raf = requestAnimationFrame(tick);
  };
  raf = requestAnimationFrame(tick);
  onUnmounted(() => cancelAnimationFrame(raf));
});

function formatTc(s: number): string {
  const ms = Math.floor((s % 1) * 1000);
  const sec = Math.floor(s);
  return `00:00:${String(sec).padStart(2, "0")}.${String(ms).padStart(3, "0")}`;
}
</script>

<template>
  <section class="hero">
    <!-- 棱镜分光背景流：ink -> amber -> teal 双光谱（vue-bits Aurora） -->
    <div class="aurora-bg" aria-hidden="true">
      <Aurora
        v-if="!reduced"
        :color-stops="['#0A0C10', '#F0A83D', '#4DD0C8']"
        :amplitude="1.1"
        :blend="0.7"
        :speed="0.45"
      />
    </div>
    <!-- top bar -->
    <header class="topbar">
      <div class="wordmark">
        <PrismLogo :height="28" />
        <span class="wordmark-text">鉴 / Prism</span>
      </div>
      <nav class="topnav">
        <a class="navlink" href="#diagnosis">诊断</a>
        <a class="navlink" href="#deploy">部署</a>
        <a class="navlink" href="https://github.com/zxc125/prism" rel="noreferrer">GitHub</a>
        <a class="btn btn-primary" href="#quickstart">自托管 →</a>
      </nav>
    </header>

    <!-- hero body -->
    <div class="hero-body">
      <!-- 左：文案 -->
      <div class="hero-copy">
        <p class="eyebrow">本地优先 · 不上云 · 不锁仓</p>
        <h1 class="hero-h1">
          你的会话，<br />
          不必<span class="accent-amber">离开你的机器</span>。
        </h1>
        <p class="hero-sub">
          鉴 / Prism
          是本地优先的前端观测平台。会话回放、诊断信号、多窗口对齐--error /
          console / network 交错进同一条时间轴，回放到第 3
          秒，同时看到页面、那条 console.error、那个 500。
        </p>
        <div class="hero-cta">
          <a class="btn btn-primary lg" href="#quickstart">自托管 →</a>
          <a class="btn btn-ghost lg" href="#diagnosis">看诊断演示</a>
        </div>
        <p class="hero-foot mono">
          <span class="rec-dot" /> REC · observer-sdk 一行接入 · 单二进制自托管
        </p>
      </div>

      <!-- 右：棱镜分光可视化 -->
      <div class="prism-visual" :class="{ reduced }">
        <div class="prism-head">
          <PrismLogo :height="72" glow />
        </div>
        <div class="lanes-wrap">
          <div class="lane lane-dom">
            <span class="lane-label mono">DOM</span>
            <div class="lane-track">
              <span
                v-for="(e, i) in domEvents"
                :key="'d' + i"
                :class="dotClass(e.kind)"
                :style="{ left: e.t * 100 + '%' }"
              />
            </div>
          </div>
          <div class="lane lane-sig">
            <span class="lane-label mono">SIGNAL</span>
            <div class="lane-track">
              <span
                v-for="(e, i) in signalEvents"
                :key="'s' + i"
                :class="dotClass(e.kind)"
                :style="{ left: e.t * 100 + '%' }"
              />
            </div>
          </div>

          <!-- 播放头 -->
          <div class="playhead">
            <span class="playhead-tc mono">{{ tc }}</span>
          </div>

          <!-- 时间码 ruler -->
          <div class="ruler mono">
            <span>00:00:00</span>
            <span>00:00:02</span>
            <span>00:00:04</span>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.hero {
  position: relative;
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--color-ink);
  overflow: hidden;
}
/* 棱镜分光背景 - vue-bits Aurora，低透明 + 径向 mask 渐隐到 ink */
.aurora-bg {
  position: absolute;
  inset: 0;
  z-index: 0;
  pointer-events: none;
  opacity: 0.6;
  mask-image: radial-gradient(ellipse 75% 65% at 72% 35%, #000 12%, transparent 78%);
  -webkit-mask-image: radial-gradient(ellipse 75% 65% at 72% 35%, #000 12%, transparent 78%);
}
.topbar,
.hero-body {
  position: relative;
  z-index: 1;
}

/* top bar */
.topbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1.5rem 2rem;
}
.wordmark {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}
.wordmark-text {
  font-size: 0.95rem;
  letter-spacing: 0.02em;
  color: var(--color-bone);
  font-weight: 600;
}
.topnav {
  display: flex;
  align-items: center;
  gap: 1.5rem;
}
.navlink {
  font-size: 0.875rem;
  color: var(--color-ash);
  text-decoration: none;
  transition: color 0.15s;
}
.navlink:hover {
  color: var(--color-bone);
}

/* hero body */
.hero-body {
  flex: 1;
  display: grid;
  grid-template-columns: 1fr;
  gap: 3rem;
  align-items: center;
  padding: 2rem 2rem 4rem;
  max-width: 1280px;
  margin: 0 auto;
  width: 100%;
}
@media (min-width: 1024px) {
  .hero-body {
    grid-template-columns: 0.85fr 1.15fr;
    gap: 4rem;
    padding: 2rem 2.5rem 5rem;
  }
}

/* copy */
.eyebrow {
  font-size: 0.75rem;
  letter-spacing: 0.18em;
  color: var(--color-ash);
  text-transform: uppercase;
  margin: 0 0 1.5rem;
}
.hero-h1 {
  font-size: clamp(2.5rem, 6vw, 4rem);
  line-height: 1.05;
  letter-spacing: -0.025em;
  font-weight: 700;
  color: var(--color-bone);
  margin: 0 0 1.5rem;
}
.accent-amber {
  color: var(--color-amber);
}
.hero-sub {
  font-size: 1.0625rem;
  line-height: 1.65;
  color: var(--color-ash);
  max-width: 32rem;
  margin: 0 0 2rem;
}
.hero-cta {
  display: flex;
  gap: 0.875rem;
  flex-wrap: wrap;
  margin-bottom: 2rem;
}
.hero-foot {
  font-size: 0.75rem;
  color: var(--color-ash-deep);
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.rec-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--color-oxblood);
  box-shadow: 0 0 6px rgba(229, 72, 77, 0.6);
  animation: pulse 1.8s ease-in-out infinite;
}
@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.35;
  }
}

/* buttons */
.btn {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.875rem;
  font-weight: 500;
  padding: 0.625rem 1.125rem;
  border-radius: 6px;
  text-decoration: none;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all 0.15s;
}
.btn.lg {
  font-size: 0.9375rem;
  padding: 0.75rem 1.375rem;
}
.btn-primary {
  background: var(--color-amber);
  color: var(--color-ink);
}
.btn-primary:hover {
  background: var(--color-amber-soft);
  box-shadow: 0 0 16px rgba(240, 168, 61, 0.35);
}
.btn-ghost {
  background: transparent;
  color: var(--color-bone);
  border-color: var(--color-hair);
}
.btn-ghost:hover {
  border-color: var(--color-ash);
  background: var(--color-slate);
}

/* prism visual */
.prism-visual {
  display: flex;
  align-items: stretch;
  gap: 0;
  padding: 2rem 1.5rem;
  background: linear-gradient(
    180deg,
    var(--color-slate) 0%,
    var(--color-ink-2) 100%
  );
  border: 1px solid var(--color-hair);
  border-radius: 10px;
  position: relative;
}
.prism-head {
  display: flex;
  align-items: center;
  padding-right: 1.25rem;
  border-right: 1px solid var(--color-hair);
}
.lanes-wrap {
  flex: 1;
  padding-left: 1.5rem;
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}
.lane {
  position: relative;
  height: 2.5rem;
  display: flex;
  align-items: center;
}
.lane-label {
  position: absolute;
  left: 0;
  top: -1.25rem;
  font-size: 0.625rem;
  letter-spacing: 0.15em;
  color: var(--color-ash-deep);
}
.lane-track {
  position: relative;
  width: 100%;
  height: 2px;
  background: var(--color-hair);
  border-radius: 1px;
}
.lane-dom .lane-track {
  background: rgba(240, 168, 61, 0.18);
}
.lane-sig .lane-track {
  background: rgba(77, 208, 200, 0.18);
}

/* dots */
.dot {
  position: absolute;
  top: 50%;
  transform: translate(-50%, -50%);
  border-radius: 50%;
}
.dot-snap {
  width: 9px;
  height: 9px;
  background: var(--color-amber);
  box-shadow: 0 0 6px rgba(240, 168, 61, 0.5);
}
.dot-incr {
  width: 4px;
  height: 4px;
  background: var(--color-amber);
  opacity: 0.75;
}
.dot-network {
  width: 6px;
  height: 6px;
  background: var(--color-teal);
  box-shadow: 0 0 5px rgba(77, 208, 200, 0.45);
}
.dot-console {
  width: 5px;
  height: 5px;
  background: var(--color-ash);
}
.dot-error {
  width: 8px;
  height: 8px;
  background: var(--color-oxblood);
  box-shadow: 0 0 7px rgba(229, 72, 77, 0.6);
  border-radius: 1px; /* error 用方块区分 */
}

/* playhead */
.playhead {
  position: absolute;
  top: 0;
  bottom: 2rem; /* 留 ruler 空间 */
  left: 1.5rem; /* 对齐 lanes-wrap padding */
  width: 2px;
  background: var(--color-amber);
  box-shadow:
    0 0 8px rgba(240, 168, 61, 0.7),
    0 0 16px rgba(240, 168, 61, 0.3);
  animation: sweep 7s linear infinite;
  pointer-events: none;
}
.playhead-tc {
  position: absolute;
  top: -1.5rem;
  left: 50%;
  transform: translateX(-50%);
  font-size: 0.625rem;
  color: var(--color-amber);
  white-space: nowrap;
  background: var(--color-ink-2);
  padding: 0.1rem 0.35rem;
  border-radius: 3px;
  border: 1px solid rgba(240, 168, 61, 0.3);
}
@keyframes sweep {
  0% {
    left: calc(1.5rem + 0%);
  }
  100% {
    left: calc(1.5rem + 100%);
  }
}

/* ruler */
.ruler {
  margin-top: 0.5rem;
  display: flex;
  justify-content: space-between;
  font-size: 0.625rem;
  color: var(--color-ash-deep);
  padding-top: 0.5rem;
  border-top: 1px solid var(--color-hair);
}

/* reduced motion */
.prism-visual.reduced .playhead {
  animation: none;
  left: calc(1.5rem + 38%);
}
.prism-visual.reduced .rec-dot {
  animation: none;
}

@media (prefers-reduced-motion: reduce) {
  .playhead {
    animation: none;
    left: calc(1.5rem + 38%);
  }
  .rec-dot {
    animation: none;
  }
}

.mono {
  font-family: var(--font-mono);
}
</style>
