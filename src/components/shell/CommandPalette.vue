<script setup lang="ts">
/**
 * 命令面板（cmd+k）：快速跳转 + 会话搜索。
 *  P10 阶段 7 打磨：最小可用版本，覆盖导航 + 会话直达。
 */
import { ref, computed, watch, nextTick } from "vue";
import { useRouter } from "vue-router";
import { getBackend, type SessionMeta } from "../../composables/backend";
import { sourceOf, SRC_LABEL, fmtClock } from "../common/format";

const open = defineModel<boolean>("open", { default: false });

const router = useRouter();
const query = ref("");
const inputRef = ref<HTMLInputElement>();
const sessions = ref<SessionMeta[]>([]);
const loading = ref(false);
const selIdx = ref(0);

// 打开时拉会话列表 + 聚焦输入
watch(open, async (v) => {
  if (v) {
    selIdx.value = 0;
    query.value = "";
    await nextTick();
    inputRef.value?.focus();
    if (!sessions.value.length) {
      loading.value = true;
      try {
        sessions.value = await getBackend().listSessions();
      } catch {
        sessions.value = [];
      } finally {
        loading.value = false;
      }
    }
  }
});

const navCommands = computed(() => {
  const base = [
    { kind: "nav" as const, label: "会话", hint: "Sessions", route: "/" },
    { kind: "nav" as const, label: "实时", hint: "Live", route: "/live" },
    { kind: "nav" as const, label: "租户", hint: "Tenants", route: "/tenants" },
    { kind: "nav" as const, label: "设置", hint: "Settings", route: "/settings" },
  ];
  if (!query.value) return base;
  const q = query.value.toLowerCase();
  return base.filter((c) => c.label.toLowerCase().includes(q) || c.hint.toLowerCase().includes(q));
});

const matchedSessions = computed(() => {
  if (!query.value) return [];
  const q = query.value.toLowerCase();
  return sessions.value
    .filter((s) => s.id.toLowerCase().includes(q) || (s.name?.toLowerCase().includes(q) ?? false))
    .slice(0, 6)
    .map((s) => ({ kind: "session" as const, label: s.name || s.id, hint: `${SRC_LABEL[sourceOf(s)]} · ${fmtClock(s.startedAt)}`, id: s.id }));
});

const items = computed(() => [...navCommands.value, ...matchedSessions.value]);

function run(item: (typeof items.value)[number]) {
  if (item.kind === "nav") {
    router.push(item.route);
  } else {
    router.push(`/s/${item.id}`);
  }
  open.value = false;
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "ArrowDown") {
    e.preventDefault();
    selIdx.value = Math.min(items.value.length - 1, selIdx.value + 1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    selIdx.value = Math.max(0, selIdx.value - 1);
  } else if (e.key === "Enter") {
    e.preventDefault();
    const it = items.value[selIdx.value];
    if (it) run(it);
  } else if (e.key === "Escape") {
    open.value = false;
  }
}

// 全局 cmd+k / ctrl+k 监听
if (typeof window !== "undefined") {
  window.addEventListener("keydown", (e) => {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      open.value = !open.value;
    }
  });
}
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="cmdk-overlay" @click.self="open = false">
      <div class="cmdk-panel">
        <div class="cmdk-input-row">
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" class="cmdk-search-icon">
            <circle cx="11" cy="11" r="7" stroke="currentColor" stroke-width="1.5" />
            <path d="M21 21l-4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
          </svg>
          <input
            ref="inputRef"
            v-model="query"
            class="cmdk-input"
            placeholder="跳转或搜索会话…"
            @keydown="onKeydown"
          />
        </div>
        <div class="cmdk-list">
          <div v-if="!items.length && !loading" class="cmdk-empty">
            无匹配
          </div>
          <button
            v-for="(it, i) in items"
            :key="it.kind + it.label"
            class="cmdk-item"
            :class="{ 'is-sel': i === selIdx }"
            @click="run(it)"
            @mousemove="selIdx = i"
          >
            <span class="ci-label">{{ it.label }}</span>
            <span class="ci-hint mono">{{ it.hint }}</span>
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.cmdk-overlay {
  position: fixed;
  inset: 0;
  background: var(--el-mask-color);
  display: flex;
  justify-content: center;
  align-items: flex-start;
  padding-top: 12vh;
  z-index: 2000;
}
.cmdk-panel {
  width: 560px;
  max-width: 92vw;
  background: var(--slate);
  border: 1px solid var(--hair);
  border-radius: var(--radius);
  box-shadow: var(--shadow-3);
  overflow: hidden;
}
.cmdk-input-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  border-bottom: 1px solid var(--hair-soft);
}
.cmdk-search-icon { color: var(--ash); flex-shrink: 0; }
.cmdk-input {
  flex: 1;
  background: transparent;
  border: 0;
  outline: 0;
  color: var(--bone);
  font-size: var(--fs-md);
  font-family: var(--font-sans);
}
.cmdk-input::placeholder { color: var(--ash-deep); }
.cmdk-list {
  max-height: 360px;
  overflow-y: auto;
  padding: 4px;
}
.cmdk-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 9px 12px;
  background: transparent;
  border: 0;
  cursor: pointer;
  border-radius: var(--radius-sm);
  text-align: left;
  transition: background 0.1s;
}
.cmdk-item.is-sel {
  background: var(--slate-2);
}
.ci-label {
  color: var(--bone-dim);
  font-size: var(--fs-sm);
}
.cmdk-item.is-sel .ci-label { color: var(--bone); }
.ci-hint {
  font-size: var(--fs-xs);
  color: var(--ash-deep);
}
.cmdk-empty {
  padding: 24px;
  text-align: center;
  color: var(--ash-deep);
  font-size: var(--fs-sm);
}
</style>
