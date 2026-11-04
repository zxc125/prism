<script lang="ts">
/**
 * 单个导入文件的状态机行（供 SessionList 组织队列）。
 * 携带 PickedBundle 判别联合：Tauri 走 { path }（Rust 读），浏览器走 { content }。
 */
export type ImportJob = {
  /** 渲染 key（文件名可能重复）。 */
  key: string;
  name: string;
  status: "wait" | "busy" | "ok" | "fail";
  /** 成功时观测端分配的新会话 id。 */
  remoteId?: string;
  /** 失败时错误文案。 */
  error?: string;
} & ({ path: string } | { content: string });

export default {};
</script>

<script setup lang="ts">
import { computed } from "vue";

/**
 * 导入通道托盘（P24 D11）：批量导入的逐文件状态机反馈。
 *
 * 签名元素承袭 Live 源机架的「通道」DNA——每个待导入文件一条通道线，LED 表状态：
 * 等待（暗灰）→ 导入中（琥珀脉冲，REC DNA）→ 已导入（冷青）→ 失败（牛血）。
 * 文件名 / 会话 id 走等宽（技术值约定）。仅列表导入期间与结束后可见，可关闭。
 */

const props = defineProps<{ jobs: ImportJob[] }>();
const emit = defineEmits<{ close: []; replay: [id: string] }>();

const STATUS_LABEL: Record<ImportJob["status"], string> = {
  wait: "等待",
  busy: "导入中",
  ok: "已导入",
  fail: "失败",
};

const okCount = computed(() => props.jobs.filter((j) => j.status === "ok").length);
const failCount = computed(() => props.jobs.filter((j) => j.status === "fail").length);
const busyCount = computed(() => props.jobs.filter((j) => j.status === "busy").length);
const total = computed(() => props.jobs.length);
const done = computed(() => okCount.value + failCount.value === total.value);
const hasFail = computed(() => failCount.value > 0);
</script>

<template>
  <div class="import-tray">
    <div class="tray-head">
      <span class="tray-title">导入通道</span>
      <span class="tray-count mono">
        <template v-if="!done">进行中 {{ busyCount }}/{{ total }}</template>
        <template v-else>完成 · 成功 {{ okCount }} · 失败 {{ failCount }}</template>
      </span>
      <button
        class="tray-close"
        :disabled="!!busyCount"
        title="关闭导入通道"
        @click="emit('close')"
      >
        ✕
      </button>
    </div>
    <ul class="tray-lines">
      <li v-for="j in jobs" :key="j.key" class="tray-line" :class="`is-${j.status}`">
        <span class="led" aria-hidden="true"></span>
        <span class="line-name mono" :title="j.name">{{ j.name }}</span>
        <span class="line-status">
          <template v-if="j.status === 'ok'">
            已导入<span v-if="j.remoteId" class="mono line-id"> · {{ j.remoteId }}</span>
            <button
              v-if="j.remoteId"
              class="line-replay"
              @click="emit('replay', j.remoteId)"
            >
              回放
            </button>
          </template>
          <template v-else-if="j.status === 'fail'">
            <span class="line-error" :title="j.error">{{ j.error }}</span>
          </template>
          <template v-else>{{ STATUS_LABEL[j.status] }}</template>
        </span>
      </li>
    </ul>
    <div v-if="done && hasFail" class="tray-hint">
      大会话导入失败时，可在来源应用「导出 bundle 文件」后在此重新导入
    </div>
  </div>
</template>

<style scoped>
.import-tray {
  border: 1px solid var(--hair);
  background: var(--slate);
  border-radius: var(--radius-sm);
  padding: 8px 12px;
  margin-bottom: 10px;
}
.tray-head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 6px;
}
.tray-title {
  font-size: var(--fs-xs);
  letter-spacing: 0.08em;
  color: var(--bone-dim);
}
.tray-count {
  font-size: var(--fs-xs);
  color: var(--ash);
  flex: 1;
}
.tray-close {
  appearance: none;
  border: none;
  background: transparent;
  color: var(--ash);
  font-size: var(--fs-xs);
  cursor: pointer;
  padding: 2px 4px;
}
.tray-close:hover:not(:disabled) {
  color: var(--bone-dim);
}
.tray-close:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}
.tray-lines {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
}
.tray-line {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 3px 0;
  font-size: var(--fs-xs);
  min-width: 0;
}
.led {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
  background: var(--ash-deep);
}
.is-busy .led {
  background: var(--amber);
  box-shadow: 0 0 6px var(--amber-glow);
  animation: tray-pulse 1.1s ease-in-out infinite;
}
.is-ok .led {
  background: var(--teal);
}
.is-fail .led {
  background: var(--oxblood);
}
.is-fail .line-status {
  color: var(--oxblood-soft);
}
.line-name {
  color: var(--bone-dim);
  max-width: 320px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.line-status {
  color: var(--ash);
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  overflow: hidden;
}
.is-ok .line-status {
  color: var(--teal);
}
.line-id {
  color: var(--ash);
}
.line-error {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.line-replay {
  appearance: none;
  border: 1px solid var(--hair);
  background: transparent;
  color: var(--ash);
  font-size: var(--fs-xs);
  padding: 1px 7px;
  border-radius: var(--radius-sm);
  cursor: pointer;
}
.line-replay:hover {
  color: var(--bone-dim);
  border-color: var(--ash-deep);
}
.tray-hint {
  margin-top: 6px;
  font-size: var(--fs-xs);
  color: var(--ash-deep);
}
@keyframes tray-pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.35;
  }
}
@media (prefers-reduced-motion: reduce) {
  .is-busy .led {
    animation: none;
  }
}
</style>
