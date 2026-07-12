<script setup lang="ts">
import "rrweb/dist/style.css";
import { useRoute } from "vue-router";
import { usePlayer } from "../composables/usePlayer";

const route = useRoute();
const id = route.params.id as string;

const gridRef = ref<HTMLElement>();
const player = usePlayer(id);

const speeds = [0.5, 1, 2, 4];

function fmt(ms: number) {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  const r = s % 60;
  return `${m}:${r.toString().padStart(2, "0")}`;
}

function onSlider(v: number | number[]) {
  const ms = Array.isArray(v) ? v[0] : v;
  player.seek(ms);
}

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
  <main class="page">
    <el-card class="box" shadow="hover">
      <template #header>
        <h2>回放 - {{ id }}</h2>
      </template>

      <div v-if="!player.ready.value" v-loading="true" class="loading" />

      <template v-else>
        <div ref="gridRef" class="grid" />

        <div class="controls">
          <el-button
            :type="player.playing.value ? 'warning' : 'primary'"
            @click="player.playing.value ? player.pause() : player.play()"
          >
            {{ player.playing.value ? "暂停" : "播放" }}
          </el-button>

          <el-slider
            class="slider"
            :min="0"
            :max="player.totalTime.value"
            :model-value="player.currentTime.value"
            :step="100"
            :format-tooltip="fmt"
            @input="onSlider"
          />

          <span class="time">
            {{ fmt(player.currentTime.value) }} / {{ fmt(player.totalTime.value) }}
          </span>

          <el-select
            :model-value="player.speed.value"
            style="width: 90px"
            @update:model-value="(v) => player.setSpeed(v as number)"
          >
            <el-option
              v-for="s in speeds"
              :key="s"
              :label="`${s}x`"
              :value="s"
            />
          </el-select>
        </div>
      </template>
    </el-card>
  </main>
</template>

<style scoped>
.page {
  display: flex;
  justify-content: center;
  padding: 16px;
}

.box {
  width: 100%;
  max-width: 920px;
}

h2 {
  margin: 0;
}

.loading {
  height: 420px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 8px;
  min-height: 420px;
  background: var(--el-fill-color-light);
  padding: 8px;
  border-radius: 4px;
}

.tile {
  display: flex;
  flex-direction: column;
  background: #fff;
  border: 1px solid var(--el-border-color);
  border-radius: 4px;
  overflow: hidden;
  min-height: 200px;
}

.tile-header {
  padding: 4px 8px;
  font-size: 12px;
  background: var(--el-color-primary-light-9);
  border-bottom: 1px solid var(--el-border-color);
}

.tile-root {
  flex: 1;
  overflow: hidden;
}

.tile-root :deep(.replayer-wrapper) {
  width: 100%;
  height: 100%;
}

.controls {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 12px;
}

.slider {
  flex: 1;
}

.time {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  white-space: nowrap;
}
</style>
