<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "../../composables/tauri";
import { fmtDur } from "../common/format";
import StatusDot from "../common/StatusDot.vue";

/** 录制控制：本机通道的 REC 按钮 + 计时 + 状态。
 *  仅 Tauri 可用（浏览器无法自录）。 */
const props = defineProps<{
  recording: boolean;
  startedAt: number | null;
  elapsedMs: number;
}>();

const emit = defineEmits<{
  (e: "start", startedAtMs: number): void;
  (e: "stop"): void;
}>();

const tauri = isTauri();
const elapsedStr = computed(() => fmtDur(props.elapsedMs));

async function toggle() {
  if (props.recording) {
    try {
      await invoke("plugin:observer|stop_session");
      emit("stop");
    } catch (e) {
      ElMessage.error(`停止失败: ${e}`);
    }
  } else {
    try {
      const sid = await invoke<string>("plugin:observer|start_session");
      emit("start", Number(sid));
    } catch (e) {
      ElMessage.error(`开始失败: ${e}`);
    }
  }
}
</script>

<template>
  <div v-if="tauri" class="rec-ctrl" :class="{ 'is-live': recording }">
    <button class="rec-btn" :class="{ 'is-rec': recording }" @click="toggle">
      <span class="rec-dot" aria-hidden="true" />
      <span class="rec-label">{{ recording ? "停止录制" : "开始录制" }}</span>
    </button>
    <div class="rec-info">
      <div class="rec-time mono">{{ recording ? elapsedStr : "待机" }}</div>
      <div class="rec-state">
        <StatusDot
          v-if="recording"
          color="var(--oxblood-soft)"
          :pulse="true"
          :size="7"
        />
        <span class="rec-state-text eyebrow">
          {{ recording ? "REC · 多窗口同步" : "就绪" }}
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.rec-ctrl {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 16px 18px;
  background: var(--slate);
  border: 1px solid var(--hair);
  border-radius: var(--radius);
  transition: border-color 0.15s, box-shadow 0.15s;
}
.rec-ctrl.is-live {
  border-color: var(--oxblood);
  box-shadow: 0 0 0 1px var(--oxblood-tint), 0 0 20px rgba(229, 72, 77, 0.12);
}
.rec-btn {
  appearance: none;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 18px;
  background: var(--slate-2);
  border: 1px solid var(--hair);
  border-radius: var(--radius);
  color: var(--bone-dim);
  font-family: var(--font-sans);
  font-size: var(--fs-sm);
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}
.rec-btn:hover {
  border-color: var(--oxblood-soft);
  color: var(--oxblood-soft);
}
.rec-btn.is-rec {
  background: var(--oxblood);
  border-color: var(--oxblood);
  color: #fff;
}
.rec-btn.is-rec:hover {
  background: var(--oxblood-soft);
  border-color: var(--oxblood-soft);
}
.rec-dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: var(--oxblood-soft);
  flex-shrink: 0;
}
.rec-btn.is-rec .rec-dot {
  background: #fff;
  animation: rec-pulse 1.1s ease-in-out infinite;
}
@keyframes rec-pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.3; transform: scale(0.7); }
}
.rec-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.rec-time {
  font-size: var(--fs-lg);
  color: var(--bone);
  letter-spacing: 0.04em;
}
.rec-ctrl.is-live .rec-time { color: var(--oxblood-soft); }
.rec-state {
  display: flex;
  align-items: center;
  gap: 6px;
}
.rec-state-text {
  font-size: 10px;
  color: var(--ash-deep);
}
</style>
