<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

type Source = "self" | "web" | "tauri";
type Session = {
  id: string;
  startedAt: number;
  endedAt?: number;
  source?: Source;
  appId?: string;
};
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

const sessions = ref<Session[]>([]);
const server = ref<IngestStatus | null>(null);

const srcFilter = ref<"all" | Source>("all");
const search = ref("");

const filterChips = [
  { key: "all", label: "全部" },
  { key: "self", label: "本机" },
  { key: "web", label: "web" },
  { key: "tauri", label: "tauri" },
] as const;

const srcColor: Record<Source, string> = {
  self: "var(--src-self)",
  web: "var(--src-web)",
  tauri: "var(--src-tauri)",
};
const srcLabel: Record<Source, string> = {
  self: "本机",
  web: "web",
  tauri: "tauri",
};

// 来源取自 session.json 的 source 字段：self-obs 写 "self"，web SDK 写 "web"
function sourceOf(s: Session): Source {
  return (s.source as Source) ?? "self";
}
// 错误计数：list_sessions 仅返回元信息，不含事件计数；P4 暂留 0（回放时信号流呈现）
function errCount(_s: Session): number {
  return 0;
}

async function refreshSessions() {
  try {
    sessions.value = await invoke("list_sessions");
  } catch (e) {
    ElMessage.error(`读取列表失败: ${e}`);
  }
}

async function loadServer() {
  try {
    server.value = await invoke<IngestStatus>("get_ingest_config");
  } catch (e) {
    console.error("[main] get_ingest_config failed", e);
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
function sessionDur(s: Session) {
  return fmtDur((s.endedAt ?? Date.now()) - s.startedAt);
}

const elapsedStr = computed(() => fmtDur(elapsedMs.value));

const filteredSessions = computed(() => {
  let list = sessions.value;
  if (srcFilter.value !== "all") {
    list = list.filter((s) => sourceOf(s) === srcFilter.value);
  }
  const q = search.value.trim().toLowerCase();
  if (q) list = list.filter((s) => s.id.toLowerCase().includes(q));
  return list;
});

const listTitle = computed(() =>
  sessions.value.length ? `${sessions.value.length} 个会话` : "会话观测台",
);

// web 通道接入指示：server 已监听即点亮，并显示已收到的 web 会话数
const webReady = computed(
  () => !!server.value?.listening && server.value?.enabled,
);
const webCount = computed(
  () => sessions.value.filter((s) => sourceOf(s) === "web").length,
);
const webStateText = computed(() => {
  if (!server.value) return "读取中";
  if (!server.value.listening) return "未监听 · 端口占用？";
  if (!server.value.enabled) return "已停用 · 设置页开启";
  return `监听 ${server.value.addr} · ${webCount.value} 个会话`;
});
const serverStateText = computed(() => {
  if (!server.value) return "读取中";
  if (!server.value.listening) return "未监听";
  if (!server.value.enabled) return "已停用";
  return `监听 ${server.value.addr}`;
});

let unlistenFocus: (() => void) | null = null;
onMounted(async () => {
  await Promise.all([refreshSessions(), loadServer()]);
  // 切回 console 窗口时自动刷新（外部 web SDK 上报后切回即可见）
  unlistenFocus = await getCurrentWebviewWindow().onFocusChanged(
    ({ payload: focused }) => {
      if (focused) {
        void refreshSessions();
        void loadServer();
      }
    },
  );
});
onBeforeUnmount(() => {
  stopTick();
  unlistenFocus?.();
});
</script>

<template>
  <main class="deck">
    <!-- 控制轨：源监控机架 -->
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
        <span class="brand-tag eyebrow">session studio</span>
      </div>

      <div class="rack">
        <div class="eyebrow rack-title">源监控 · source</div>

        <!-- 本机通道（功能性） -->
        <div class="channel" :class="{ 'is-live': recording }" style="--c: var(--src-self)">
          <div class="ch-head">
            <span class="ch-dot" aria-hidden="true" />
            <span class="ch-label mono">本机</span>
            <button
              class="ch-act"
              :aria-label="recording ? '停止录制' : '开始录制'"
              @click="recording ? stopSession() : startSession()"
            >
              {{ recording ? "■ 停止" : "● 录制" }}
            </button>
          </div>
          <div class="ch-time mono">{{ recording ? elapsedStr : "待机" }}</div>
          <div class="ch-state eyebrow">
            <span v-if="recording" class="live-dot" aria-hidden="true" />
            {{ recording ? "REC · 多窗口同步" : "就绪" }}
          </div>
        </div>

        <!-- web 通道：server 监听即点亮 -->
        <div
          class="channel"
          :class="{ 'is-pending': !webReady }"
          style="--c: var(--src-web)"
        >
          <div class="ch-head">
            <span class="ch-dot" :class="{ 'is-up': webReady }" aria-hidden="true" />
            <span class="ch-label mono">web</span>
            <span v-if="webReady" class="ch-addr mono">{{ server?.addr }}</span>
          </div>
          <div class="ch-state eyebrow">{{ webStateText }}</div>
        </div>

        <!-- tauri 通道（待接入） -->
        <div class="channel is-pending" style="--c: var(--src-tauri)">
          <div class="ch-head">
            <span class="ch-dot" aria-hidden="true" />
            <span class="ch-label mono">tauri</span>
          </div>
          <div class="ch-state eyebrow">待接入 · P5</div>
        </div>
      </div>

      <div class="rail-foot">
        <div class="server-status">
          <span
            class="ss-dot"
            :class="{ 'is-up': webReady }"
            aria-hidden="true"
          />
          <span class="mono">server</span>
          <span class="ss-state eyebrow">{{ serverStateText }}</span>
        </div>
        <button class="link" @click="refreshSessions">
          <span class="mono">↻</span> 刷新列表
        </button>
        <button class="link" @click="openSettings">
          <span class="mono">⚙</span> 设置
        </button>
      </div>
    </aside>

    <!-- 会话浏览器 -->
    <section class="browser">
      <header class="browser-head">
        <div>
          <div class="eyebrow">会话观测</div>
          <h1 class="browser-title">{{ listTitle }}</h1>
        </div>
        <div class="filters">
          <button
            v-for="c in filterChips"
            :key="c.key"
            class="chip mono"
            :class="{ 'is-active': srcFilter === c.key }"
            @click="srcFilter = c.key"
          >
            {{ c.label }}
          </button>
          <el-input
            v-model="search"
            class="search"
            placeholder="搜索会话 ID"
            size="small"
            clearable
          />
        </div>
      </header>

      <div class="session-list">
        <div v-if="!filteredSessions.length" class="empty">
          {{
            sessions.length ? "无匹配会话" : "暂无会话 · 在左侧本机通道开始录制，或用 web SDK 上报"
          }}
        </div>
        <div v-for="s in filteredSessions" :key="s.id" class="row">
          <span class="row-dot" :style="{ '--c': srcColor[sourceOf(s)] }" aria-hidden="true" />
          <span class="row-src mono">{{ srcLabel[sourceOf(s)] }}</span>
          <span class="row-time mono">{{ fmtClock(s.startedAt) }}</span>
          <span class="row-dur mono">{{ sessionDur(s) }}</span>
          <span v-if="errCount(s) > 0" class="row-err mono">⚠{{ errCount(s) }}</span>
          <span class="row-id mono">{{ s.id }}</span>
          <span class="row-spacer" />
          <el-button size="small" type="primary" @click="openPlayer(s.id)">回放</el-button>
          <el-dropdown trigger="click" @command="(cmd: string) => cmd === 'delete' && deleteSession(s.id)">
            <el-button size="small" class="more" @click.stop>⋯</el-button>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item command="delete">删除</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </div>
    </section>
  </main>
</template>

<style scoped>
.deck {
  display: grid;
  grid-template-columns: 280px 1fr;
  height: 100vh;
  background: var(--ink);
}

/* ---- rail / source rack ---- */
.rail {
  display: flex;
  flex-direction: column;
  gap: 20px;
  padding: 20px 18px;
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

.rack {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.rack-title {
  padding-bottom: 6px;
  border-bottom: 1px solid var(--hair-soft);
}

.channel {
  border: 1px solid var(--hair);
  border-radius: var(--radius);
  background: var(--ink-2);
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.channel.is-live {
  border-color: var(--oxblood);
  background: linear-gradient(180deg, rgba(181, 56, 58, 0.14), transparent);
}
.channel.is-pending {
  border-style: dashed;
  border-color: var(--hair-soft);
  opacity: 0.72;
}
.ch-head {
  display: flex;
  align-items: center;
  gap: 8px;
}
.ch-dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: var(--c, var(--ash));
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--c, var(--ash)) 22%, transparent);
  flex-shrink: 0;
}
.ch-dot.is-up {
  box-shadow: 0 0 8px color-mix(in srgb, var(--c, var(--ash)) 60%, transparent);
}
.channel.is-live .ch-dot {
  background: var(--oxblood-soft);
  box-shadow: 0 0 8px var(--oxblood);
  animation: rec-pulse 1.1s ease-in-out infinite;
}
.ch-label {
  font-size: var(--fs-sm);
  color: var(--bone-dim);
  letter-spacing: 0.04em;
}
.channel.is-pending .ch-label {
  color: var(--ash);
}
.ch-addr {
  margin-left: auto;
  font-size: var(--fs-xs);
  color: var(--src-web);
  opacity: 0.85;
}
.ch-act {
  margin-left: auto;
  appearance: none;
  border: 1px solid var(--hair);
  background: transparent;
  color: var(--bone-dim);
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  padding: 3px 9px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  letter-spacing: 0.04em;
  transition: border-color 0.15s, color 0.15s;
}
.ch-act:hover {
  border-color: var(--ash-deep);
  color: var(--bone);
}
.channel.is-live .ch-act {
  border-color: var(--oxblood-soft);
  color: var(--oxblood-soft);
}
.ch-time {
  font-size: var(--fs-lg);
  color: var(--bone);
  letter-spacing: 0.06em;
}
.channel.is-live .ch-time {
  color: var(--oxblood-soft);
}
.channel.is-pending .ch-time {
  color: var(--ash-deep);
}
.ch-state {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 10px;
  color: var(--ash-deep);
}
.live-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--oxblood-soft);
  box-shadow: 0 0 8px var(--oxblood);
  animation: rec-pulse 1.1s ease-in-out infinite;
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
  gap: 2px;
  padding-top: 12px;
  border-top: 1px solid var(--hair-soft);
}
.server-status {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 6px 10px;
  font-size: var(--fs-xs);
  color: var(--ash-deep);
}
.ss-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--ash-deep);
}
.ss-dot.is-up {
  background: var(--src-web);
  box-shadow: 0 0 8px color-mix(in srgb, var(--src-web) 60%, transparent);
}
.ss-state {
  margin-left: auto;
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

/* ---- session browser ---- */
.browser {
  display: flex;
  flex-direction: column;
  padding: 22px 24px;
  min-width: 0;
  overflow: auto;
}
.browser-head {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 18px;
}
.browser-title {
  margin: 4px 0 0;
  font-size: var(--fs-xl);
  font-weight: 600;
  color: var(--bone);
  letter-spacing: -0.01em;
}
.filters {
  display: flex;
  align-items: center;
  gap: 6px;
}
.chip {
  appearance: none;
  border: 1px solid var(--hair);
  background: transparent;
  color: var(--ash);
  font-size: var(--fs-xs);
  padding: 5px 11px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  letter-spacing: 0.04em;
  transition: color 0.15s, border-color 0.15s, background 0.15s;
}
.chip:hover {
  color: var(--bone-dim);
  border-color: var(--ash-deep);
}
.chip.is-active {
  color: var(--ink);
  background: var(--amber);
  border-color: var(--amber);
}
.search {
  width: 200px;
}

.session-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.empty {
  color: var(--ash-deep);
  font-size: var(--fs-sm);
  padding: 48px 0;
  text-align: center;
}
.row {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 11px 12px;
  border: 1px solid transparent;
  border-radius: var(--radius);
  transition: background 0.12s, border-color 0.12s;
}
.row:hover {
  background: var(--slate);
  border-color: var(--hair-soft);
}
.row-dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: var(--c, var(--ash));
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--c, var(--ash)) 20%, transparent);
  flex-shrink: 0;
}
.row-src {
  font-size: var(--fs-xs);
  color: var(--bone-dim);
  letter-spacing: 0.06em;
  width: 44px;
}
.row-time {
  font-size: var(--fs-sm);
  color: var(--bone-dim);
}
.row-dur {
  font-size: var(--fs-sm);
  color: var(--ash);
  width: 56px;
}
.row-err {
  font-size: var(--fs-xs);
  color: var(--oxblood-soft);
}
.row-id {
  font-size: var(--fs-xs);
  color: var(--ash-deep);
}
.row-spacer {
  flex: 1;
}
.more {
  font-family: var(--font-mono);
}
</style>
