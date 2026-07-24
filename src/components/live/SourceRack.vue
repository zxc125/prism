<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getBackend, type SessionMeta } from "../../composables/backend";
import { onWindowFocus } from "../../composables/tauri";
import { sourceOf } from "../common/format";
import StatusDot from "../common/StatusDot.vue";
import RecordControls from "./RecordControls.vue";

/** 源机架：本机 / web / tauri 三通道状态。
 *  从旧 MainView 的 rail 拆出，保留「源监控」签名元素，刷新为现代科技感。 */
type IngestStatus = {
  enabled: boolean;
  port: number;
  token: string;
  listening: boolean;
  addr: string | null;
};

const recording = ref(false);
const startedAt = ref<number | null>(null);
const elapsedMs = ref(0);
let tickHandle: number | null = null;

const sessions = ref<SessionMeta[]>([]);
const server = ref<IngestStatus | null>(null);

const webReady = computed(() => !!server.value?.listening && server.value?.enabled);
const webCount = computed(() => sessions.value.filter((s) => sourceOf(s) === "web").length);
const tauriCount = computed(() => sessions.value.filter((s) => sourceOf(s) === "tauri").length);

const webStateText = computed(() => {
  if (!server.value) return "读取中";
  if (!server.value.listening) return "未监听 · 端口占用？";
  if (!server.value.enabled) return "已停用 · 设置页开启";
  return `监听 ${server.value.addr} · ${webCount.value} 个会话`;
});
const tauriStateText = computed(() => {
  if (!server.value) return "读取中";
  if (!server.value.listening) return "未监听 · 端口占用？";
  if (!server.value.enabled) return "已停用 · 设置页开启";
  return `就绪 · ${tauriCount.value} 个会话`;
});
const serverStateText = computed(() => {
  if (!server.value) return "读取中";
  if (!server.value.listening) return "未监听";
  if (!server.value.enabled) return "已停用";
  return `监听 ${server.value.addr}`;
});

function startTick() {
  stopTick();
  tickHandle = window.setInterval(() => {
    if (startedAt.value) elapsedMs.value = Date.now() - startedAt.value;
  }, 200);
}
function stopTick() {
  if (tickHandle != null) {
    clearInterval(tickHandle);
    tickHandle = null;
  }
}

async function refreshSessions() {
  try {
    sessions.value = await getBackend().listSessions();
  } catch (e) {
    console.error("[live] list sessions failed", e);
  }
}
async function loadServer() {
  try {
    server.value = await invoke<IngestStatus>("get_ingest_config");
  } catch (e) {
    console.error("[live] get_ingest_config failed", e);
  }
}

function onRecStart(sid: number) {
  startedAt.value = sid;
  elapsedMs.value = 0;
  recording.value = true;
  startTick();
  ElMessage.success("已开始录制");
}
async function onRecStop() {
  stopTick();
  startedAt.value = null;
  elapsedMs.value = 0;
  recording.value = false;
  ElMessage.success("已停止录制");
  await refreshSessions();
}

let unlistenFocus: (() => void) | null = null;
onMounted(async () => {
  await Promise.all([refreshSessions(), loadServer()]);
  unlistenFocus = await onWindowFocus((focused) => {
    if (focused) {
      void refreshSessions();
      void loadServer();
    }
  });
});
onBeforeUnmount(() => {
  stopTick();
  unlistenFocus?.();
});

defineExpose({ refresh: () => Promise.all([refreshSessions(), loadServer()]) });
</script>

<template>
  <div class="source-rack">
    <div class="eyebrow rack-title">源监控 · source</div>

    <!-- 本机通道（录制控制） -->
    <div class="channel" :class="{ 'is-live': recording }">
      <div class="ch-head">
        <StatusDot
          :color="recording ? 'var(--oxblood-soft)' : 'var(--src-self)'"
          :glow="recording"
          :pulse="recording"
          :size="9"
        />
        <span class="ch-label mono">本机</span>
        <span class="ch-tag eyebrow">self-obs</span>
      </div>
      <RecordControls
        :recording="recording"
        :started-at="startedAt"
        :elapsed-ms="elapsedMs"
        @start="onRecStart"
        @stop="onRecStop"
      />
    </div>

    <!-- web 通道 -->
    <div class="channel" :class="{ 'is-pending': !webReady }">
      <div class="ch-head">
        <StatusDot
          color="var(--src-web)"
          :glow="webReady"
          :size="9"
        />
        <span class="ch-label mono">web</span>
        <span v-if="webReady && server?.addr" class="ch-addr mono">{{ server.addr }}</span>
      </div>
      <div class="ch-state eyebrow">{{ webStateText }}</div>
    </div>

    <!-- tauri 通道 -->
    <div class="channel" :class="{ 'is-pending': !webReady }">
      <div class="ch-head">
        <StatusDot color="var(--src-tauri)" :glow="webReady" :size="9" />
        <span class="ch-label mono">tauri</span>
      </div>
      <div class="ch-state eyebrow">{{ tauriStateText }}</div>
    </div>

    <!-- server 总状态 -->
    <div class="server-status">
      <StatusDot
        :color="webReady ? 'var(--teal)' : 'var(--ash-deep)'"
        :glow="webReady"
        :size="7"
      />
      <span class="mono">server</span>
      <span class="ss-state eyebrow">{{ serverStateText }}</span>
    </div>
  </div>
</template>

<style scoped>
.source-rack {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.rack-title {
  padding-bottom: 6px;
  border-bottom: 1px solid var(--hair-soft);
}
.channel {
  border: 1px solid var(--hair);
  border-radius: var(--radius);
  background: var(--slate);
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  transition: border-color 0.15s, box-shadow 0.15s;
}
.channel.is-live {
  border-color: var(--oxblood);
  box-shadow: 0 0 0 1px var(--oxblood-tint), 0 0 20px rgba(229, 72, 77, 0.10);
}
.channel.is-pending {
  border-style: dashed;
  border-color: var(--hair-soft);
  opacity: 0.78;
}
.ch-head {
  display: flex;
  align-items: center;
  gap: 8px;
}
.ch-label {
  font-size: var(--fs-sm);
  color: var(--bone-dim);
  letter-spacing: 0.04em;
}
.channel.is-pending .ch-label { color: var(--ash); }
.ch-tag {
  margin-left: auto;
  font-size: 10px;
  color: var(--ash-deep);
}
.ch-addr {
  margin-left: auto;
  font-size: var(--fs-xs);
  color: var(--src-web);
  opacity: 0.85;
}
.ch-state {
  font-size: 10px;
  color: var(--ash-deep);
}
.server-status {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  font-size: var(--fs-xs);
  color: var(--ash-deep);
  background: var(--ink-2);
  border-radius: var(--radius-sm);
}
.ss-state {
  margin-left: auto;
}
</style>
