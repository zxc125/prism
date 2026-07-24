<script setup lang="ts">
import { ref, onMounted, watchEffect } from "vue";
import { getBackend, loadBackendConfig, type Whoami } from "../../composables/backend";
import { isTauri } from "../../composables/tauri";
import { fmtBytes } from "../common/format";
import StatusDot from "../common/StatusDot.vue";

/** 顶栏 tenant 上下文 + 连接指示器 + 搜索 + 命令面板入口。
 *  D5：tenant 上下文放在顶栏左（最显眼），多租户是一等公民。 */
const emit = defineEmits<{ (e: "cmdk"): void }>();

const whoami = ref<Whoami | null>(null);
const loading = ref(false);
const tauri = isTauri();

const cfg = ref(loadBackendConfig());
const endpointLabel = computed(() => {
  if (tauri) return cfg.value.mode === "http" && cfg.value.endpoint ? cfg.value.endpoint : "本地";
  return cfg.value.endpoint || "未连接";
});
const isCloud = computed(() => !tauri || cfg.value.mode === "http");

async function refresh() {
  loading.value = true;
  try {
    whoami.value = await getBackend().whoami();
  } catch (e) {
    console.error("[topbar] whoami failed", e);
    whoami.value = null;
  } finally {
    loading.value = false;
  }
}

// 配置可能在外部（设置页）变更，重新读
watchEffect(() => {
  cfg.value = loadBackendConfig();
});

defineExpose({ refresh });
onMounted(refresh);

// 配额余量百分比
const quotaPct = computed(() => {
  const w = whoami.value;
  if (!w?.multiTenant || !w.quotaBytes) return null;
  const used = w.usageBytes ?? 0;
  return Math.min(100, (used / w.quotaBytes) * 100);
});
const quotaLabel = computed(() => {
  const w = whoami.value;
  if (!w?.multiTenant || !w.quotaBytes) return null;
  return `${fmtBytes(w.usageBytes ?? 0)} / ${fmtBytes(w.quotaBytes)}`;
});
</script>

<template>
  <header class="top-bar">
    <!-- tenant 上下文（左·一等公民） -->
    <div class="tb-left">
      <div v-if="whoami?.multiTenant" class="tenant-ctx">
        <StatusDot color="var(--amber)" :glow="true" :size="7" />
        <span class="tenant-id mono">{{ whoami.tenantId }}</span>
        <div v-if="quotaPct !== null" class="quota-bar" :title="quotaLabel ?? ''">
          <div class="qb-fill" :style="{ width: quotaPct + '%' }" />
        </div>
        <span v-if="quotaLabel" class="quota-label mono">{{ quotaLabel }}</span>
      </div>
      <div v-else class="tenant-ctx is-single">
        <StatusDot color="var(--ash-deep)" :size="7" />
        <span class="tenant-id mono">{{ isCloud ? "云端" : "本地" }}</span>
      </div>
    </div>

    <!-- 连接指示器 -->
    <div class="tb-center">
      <div class="conn-indicator">
        <StatusDot
          :color="isCloud ? 'var(--amber)' : 'var(--teal)'"
          :glow="isCloud"
          :size="6"
        />
        <span class="conn-label mono">{{ endpointLabel }}</span>
      </div>
    </div>

    <!-- 右：搜索 + cmd+k -->
    <div class="tb-right">
      <button class="cmdk-btn" @click="emit('cmdk')">
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none">
          <circle cx="11" cy="11" r="7" stroke="currentColor" stroke-width="1.5" />
          <path d="M21 21l-4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
        </svg>
        <span class="cmdk-text">搜索</span>
        <kbd class="cmdk-kbd mono">⌘K</kbd>
      </button>
    </div>
  </header>
</template>

<style scoped>
.top-bar {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  height: 44px;
  padding: 0 16px;
  background: var(--ink-2);
  border-bottom: 1px solid var(--hair);
}
.tb-left { display: flex; align-items: center; }
.tb-center { display: flex; justify-content: center; }
.tb-right { display: flex; justify-content: flex-end; }

.tenant-ctx {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 10px;
  background: var(--slate);
  border: 1px solid var(--hair);
  border-radius: var(--radius);
}
.tenant-ctx.is-single {
  background: transparent;
  border-color: var(--hair-soft);
}
.tenant-id {
  font-size: var(--fs-sm);
  color: var(--bone);
  letter-spacing: 0.04em;
}
.quota-bar {
  width: 64px;
  height: 4px;
  background: var(--hair-soft);
  border-radius: 2px;
  overflow: hidden;
}
.qb-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--teal), var(--amber));
  transition: width 0.3s;
}
.quota-label {
  font-size: 10px;
  color: var(--ash);
}

.conn-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
}
.conn-label {
  font-size: var(--fs-xs);
  color: var(--ash);
  letter-spacing: 0.04em;
}

.cmdk-btn {
  appearance: none;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 10px;
  background: var(--slate);
  border: 1px solid var(--hair);
  border-radius: var(--radius-sm);
  color: var(--ash);
  font-size: var(--fs-xs);
  cursor: pointer;
  transition: color 0.12s, border-color 0.12s;
}
.cmdk-btn:hover {
  color: var(--bone-dim);
  border-color: var(--ash-deep);
}
.cmdk-kbd {
  font-size: 10px;
  padding: 1px 5px;
  background: var(--ink-2);
  border: 1px solid var(--hair);
  border-radius: 3px;
  color: var(--ash-deep);
}
</style>
