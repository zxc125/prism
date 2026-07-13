<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";

const recording = ref(false);
const startedAt = ref<number | null>(null);
const elapsedMs = ref(0);
let tickHandle: number | null = null;

const sessions = ref<
  Array<{ id: string; startedAt: number; endedAt?: number }>
>([]);

type Session = { id: string; startedAt: number; endedAt?: number };

async function refreshSessions() {
  try {
    sessions.value = await invoke("list_sessions");
  } catch (e) {
    ElMessage.error(`读取列表失败: ${e}`);
  }
}

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

async function startSession() {
  try {
    // start_session 返回的 id 即起始毫秒时间戳，用作计时基准
    const sid = await invoke<string>("start_session");
    startedAt.value = Number(sid);
    elapsedMs.value = 0;
    recording.value = true;
    startTick();
    ElMessage.success("已开始录制");
  } catch (e) {
    ElMessage.error(`开始失败: ${e}`);
  }
}

async function stopSession() {
  try {
    await invoke("stop_session");
    stopTick();
    startedAt.value = null;
    elapsedMs.value = 0;
    recording.value = false;
    ElMessage.success("已停止录制");
    await refreshSessions();
  } catch (e) {
    ElMessage.error(`停止失败: ${e}`);
  }
}

async function openSettings() {
  try {
    await invoke("open_window", { route: "/settings" });
  } catch (e) {
    ElMessage.error(`打开失败: ${e}`);
  }
}

async function openPlayer(id: string) {
  try {
    await invoke("open_window", { route: `/player/${id}` });
  } catch (e) {
    ElMessage.error(`打开失败: ${e}`);
  }
}

async function deleteSession(id: string) {
  try {
    await ElMessageBox.confirm("删除该录制后无法恢复。", "删除录制", {
      type: "warning",
      confirmButtonText: "删除",
      cancelButtonText: "取消",
    });
  } catch {
    return;
  }
  try {
    await invoke("delete_session", { id });
    await refreshSessions();
    ElMessage.success("已删除");
  } catch (e) {
    ElMessage.error(`删除失败: ${e}`);
  }
}

function fmtClock(ts?: number) {
  if (!ts) return "-";
  const d = new Date(ts);
  const p = (n: number) => n.toString().padStart(2, "0");
  return `${p(d.getMonth() + 1)}/${p(d.getDate())} ${p(d.getHours())}:${p(
    d.getMinutes(),
  )}`;
}
function fmtDur(ms: number) {
  const s = Math.max(0, Math.floor(ms / 1000));
  const m = Math.floor(s / 60);
  const h = Math.floor(m / 60);
  const p = (n: number) => n.toString().padStart(2, "0");
  return h > 0 ? `${h}:${p(m % 60)}:${p(s % 60)}` : `${m}:${p(s % 60)}`;
}
function sessionDur(s: { startedAt: number; endedAt?: number }) {
  return fmtDur((s.endedAt ?? Date.now()) - s.startedAt);
}

const elapsedStr = computed(() => fmtDur(elapsedMs.value));
const logTitle = computed(() =>
  sessions.value.length ? `${sessions.value.length} 个会话` : "尚未录制",
);

onMounted(refreshSessions);
onBeforeUnmount(stopTick);
</script>

<template>
  <main class="deck">
    <!-- control rail -->
    <aside class="rail">
      <div class="brand">
        <span class="brand-mark" aria-hidden="true">
          <svg viewBox="0 0 24 24" width="22" height="22" fill="none">
            <rect x="1.5" y="3" width="21" height="18" rx="2" stroke="currentColor" stroke-width="1.5" />
            <circle cx="5" cy="6.5" r="1" fill="currentColor" />
            <circle cx="5" cy="12" r="1" fill="currentColor" />
            <circle cx="5" cy="17.5" r="1" fill="currentColor" />
            <circle cx="19" cy="6.5" r="1" fill="currentColor" />
            <circle cx="19" cy="12" r="1" fill="currentColor" />
            <circle cx="19" cy="17.5" r="1" fill="currentColor" />
          </svg>
        </span>
        <span class="brand-name mono">replay</span>
        <span class="brand-tag eyebrow">session&nbsp;studio</span>
      </div>

      <div class="rec-block">
        <button
          class="rec"
          :class="{ 'is-rec': recording }"
          :aria-label="recording ? '停止录制' : '开始录制'"
          @click="recording ? stopSession() : startSession()"
        >
          <span class="rec-ring" aria-hidden="true">
            <svg viewBox="0 0 100 100" width="104" height="104">
              <circle
                cx="50"
                cy="50"
                r="47"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-dasharray="1.5 4.2"
              />
            </svg>
          </span>
          <span class="rec-core">
            <span v-if="!recording" class="rec-dot" aria-hidden="true" />
            <span v-else class="rec-stop" aria-hidden="true" />
            <span class="rec-label mono">{{ recording ? "STOP" : "REC" }}</span>
          </span>
        </button>

        <div class="rec-time mono">{{ recording ? elapsedStr : "T+ 0:00" }}</div>
        <div class="rec-state eyebrow">
          <span v-if="recording" class="live-dot" aria-hidden="true" />
          {{ recording ? "录制中 · 多窗口同步" : "待机" }}
        </div>
      </div>

      <div class="rail-foot">
        <p class="rail-note">录制时关闭子窗口 = 暂停该窗口，再次打开续录。</p>
        <div class="rail-links">
          <button class="link" @click="refreshSessions">
            <span class="mono">↻</span> 刷新列表
          </button>
          <button class="link" @click="openSettings">
            <span class="mono">⚙</span> 设置
          </button>
        </div>
      </div>
    </aside>

    <!-- session log -->
    <section class="log">
      <header class="log-head">
        <div>
          <div class="eyebrow">会话日志</div>
          <h1 class="log-title">{{ logTitle }}</h1>
        </div>
        <p class="log-hint mono">按时间倒序 · 选择会话回放</p>
      </header>

      <el-table
        :data="sessions"
        size="small"
        class="log-table"
        empty-text="暂无录制 · 按下 REC 开始第一段"
      >
        <el-table-column label="开始" width="150">
          <template #default="{ row }">
            <span class="mono cell-time">{{ fmtClock(row.startedAt) }}</span>
          </template>
        </el-table-column>
        <el-table-column label="时长" width="80">
          <template #default="{ row }">
            <span class="mono cell-dur">{{ sessionDur(row as Session) }}</span>
          </template>
        </el-table-column>
        <el-table-column label="会话 ID">
          <template #default="{ row }">
            <span class="mono cell-id">{{ row.id }}</span>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="150" align="right">
          <template #default="{ row }">
            <el-button size="small" type="primary" @click="openPlayer(row.id)">
              回放
            </el-button>
            <el-button
              size="small"
              type="danger"
              plain
              @click="deleteSession(row.id)"
            >
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </section>
  </main>
</template>

<style scoped>
.deck {
  display: grid;
  grid-template-columns: 268px 1fr;
  height: 100vh;
  background: var(--ink);
}

/* ---- control rail ---- */
.rail {
  display: flex;
  flex-direction: column;
  gap: 24px;
  padding: 22px 20px;
  background: var(--slate);
  border-right: 1px solid var(--hair);
}

.brand {
  display: flex;
  align-items: center;
  gap: 8px;
}
.brand-mark {
  color: var(--amber);
  display: flex;
}
.brand-name {
  font-size: var(--fs-md);
  font-weight: 600;
  letter-spacing: 0.04em;
  color: var(--bone);
}
.brand-tag {
  margin-left: auto;
  font-size: 10px;
}

/* ---- REC control (signature) ---- */
.rec-block {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  padding: 12px 0 4px;
}
.rec {
  position: relative;
  width: 108px;
  height: 108px;
  border: 0;
  background: transparent;
  cursor: pointer;
  color: var(--amber);
  padding: 0;
  border-radius: 50%;
}
.rec-ring {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--ash-deep);
  transition: color 0.2s;
}
.rec:hover .rec-ring {
  color: var(--amber-deep);
}
.rec-core {
  position: absolute;
  inset: 20px;
  border-radius: 50%;
  background: var(--ink);
  border: 2px solid var(--amber);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 5px;
  transition: background 0.2s, border-color 0.2s;
}
.rec-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--amber);
  box-shadow: 0 0 10px var(--amber);
}
.rec-stop {
  width: 14px;
  height: 14px;
  background: #fff;
  border-radius: 2px;
}
.rec-label {
  font-size: 11px;
  letter-spacing: 0.16em;
  color: var(--bone);
  font-weight: 600;
}
.rec-time {
  font-size: var(--fs-lg);
  color: var(--bone);
  letter-spacing: 0.06em;
}
.rec-state {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 10px;
}
.live-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--oxblood-soft);
  box-shadow: 0 0 8px var(--oxblood);
  animation: rec-pulse 1.1s ease-in-out infinite;
}
.rec.is-rec {
  color: var(--oxblood-soft);
}
.rec.is-rec .rec-ring {
  color: var(--oxblood);
  opacity: 0.55;
}
.rec.is-rec .rec-core {
  border-color: var(--oxblood-soft);
  background: var(--oxblood);
}
.rec.is-rec .rec-label {
  color: #fff;
}
@keyframes rec-pulse {
  0%,
  100% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.3;
    transform: scale(0.75);
  }
}

/* ---- rail foot ---- */
.rail-foot {
  margin-top: auto;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.rail-note {
  margin: 0;
  font-size: var(--fs-xs);
  line-height: 1.55;
  color: var(--ash-deep);
  border-top: 1px solid var(--hair-soft);
  padding-top: 12px;
}
.rail-links {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.link {
  appearance: none;
  border: 0;
  background: transparent;
  color: var(--ash);
  font-family: var(--font-sans);
  font-size: var(--fs-sm);
  text-align: left;
  padding: 8px 6px;
  border-radius: var(--radius);
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 10px;
  transition: background 0.15s, color 0.15s;
}
.link .mono {
  color: var(--amber);
  width: 14px;
  text-align: center;
}
.link:hover {
  background: var(--slate-2);
  color: var(--bone);
}

/* ---- session log ---- */
.log {
  display: flex;
  flex-direction: column;
  padding: 22px 24px;
  min-width: 0;
  overflow: auto;
}
.log-head {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 16px;
}
.log-title {
  margin: 4px 0 0;
  font-size: var(--fs-xl);
  font-weight: 600;
  color: var(--bone);
  letter-spacing: -0.01em;
}
.log-hint {
  color: var(--ash-deep);
  font-size: var(--fs-xs);
}

.log-table {
  --el-table-bg-color: transparent;
  --el-table-tr-bg-color: transparent;
  --el-table-header-bg-color: transparent;
}
.cell-time,
.cell-dur {
  color: var(--bone-dim);
}
.cell-id {
  color: var(--ash);
  font-size: var(--fs-xs);
}
</style>
