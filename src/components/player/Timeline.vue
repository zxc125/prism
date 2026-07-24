<script setup lang="ts">
import { inject } from "vue";
import { PLAYER_CTX, type PlayerCtx } from "./context";
import { LANE_COLORS } from "../../composables/usePlayer";

/** 多轨时间轴（签名元素保留）：色带 + error/focus/标注标记 + 播放头 + 滑块。 */
const ctx = inject<PlayerCtx>(PLAYER_CTX);
if (!ctx) throw new Error("Timeline 必须在 PlayerShell 内使用");
const { player, annos } = ctx;

const palette = LANE_COLORS;

function fmt(ms: number) {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  const r = s % 60;
  const p = (n: number) => n.toString().padStart(2, "0");
  return `${p(m)}:${p(r)}`;
}

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

function onSlider(v: number | number[]) {
  const ms = Array.isArray(v) ? v[0] : v;
  player.seek(ms);
}
</script>

<template>
  <div class="timeline-col">
    <div
      class="timeline-overview"
      :style="{ height: player.timeline.value.labels.length * 8 + 4 + 'px' }"
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
        v-for="a in annos.annotations.value"
        :key="'an' + a.id"
        class="tl-anno"
        :style="{ left: pct(a.t) + '%' }"
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
</template>

<style scoped>
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
.tl-anno {
  position: absolute;
  bottom: -3px;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--bone);
  transform: translateX(-50%);
  pointer-events: none;
  box-shadow: 0 0 0 1px var(--ink), 0 0 5px rgba(230, 234, 240, 0.45);
}
.tl-playhead {
  position: absolute;
  top: -2px;
  bottom: -2px;
  width: 2px;
  background: var(--amber);
  pointer-events: none;
  transform: translateX(-1px);
  box-shadow: 0 0 6px var(--amber-glow);
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
.slider { width: 100%; }
.slider :deep(.el-slider__runway) { margin: 8px 0; }
</style>
