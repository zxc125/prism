<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { getBackend, type Whoami } from "../../composables/backend";
import { fmtBytes } from "../common/format";
import StatusDot from "../common/StatusDot.vue";
import EmptyState from "../common/EmptyState.vue";

/** Tenant 列表：当前 key 可见的 tenant（起步只 1 个，只读）。
 *  P9 服务端多租户在 P10 首次有 console 出口。 */
const router = useRouter();
const whoami = ref<Whoami | null>(null);
const loading = ref(true);

async function refresh() {
  loading.value = true;
  try {
    whoami.value = await getBackend().whoami();
  } catch (e) {
    console.error("[tenants] whoami failed", e);
  } finally {
    loading.value = false;
  }
}

onMounted(refresh);

const quotaPct = computed(() => {
  const w = whoami.value;
  if (!w?.multiTenant || !w.quotaBytes) return null;
  return Math.min(100, ((w.usageBytes ?? 0) / w.quotaBytes) * 100);
});
</script>

<template>
  <section class="tenant-list">
    <header class="tl-head">
      <div>
        <div class="eyebrow">多租户</div>
        <h1 class="tl-title">租户</h1>
        <p class="tl-sub">当前 API key 可见的租户。增删改 key/tenant 留 P10.5/P11。</p>
      </div>
    </header>

    <EmptyState
      v-if="!loading && !whoami?.multiTenant"
      icon="◌"
      title="单租户模式"
      hint="当前 server 未启用多租户（无 tenants.json）。配置多租户后，这里会展示租户列表与配额。"
    />

    <div v-else-if="whoami?.multiTenant" class="tenant-cards">
      <div class="tenant-card" @click="router.push(`/tenants/${whoami.tenantId}`)">
        <div class="tc-head">
          <StatusDot color="var(--amber)" :glow="true" :size="9" />
          <span class="tc-id mono">{{ whoami.tenantId }}</span>
          <span class="tc-arrow mono">→</span>
        </div>
        <div class="tc-body">
          <div v-if="whoami.appIds?.length" class="tc-row">
            <span class="tc-key eyebrow">appIds</span>
            <span class="tc-val mono">{{ whoami.appIds.join(", ") }}</span>
          </div>
          <div v-if="whoami.quotaBytes" class="tc-row">
            <span class="tc-key eyebrow">配额</span>
            <div class="tc-quota">
              <div class="tq-bar">
                <div class="tq-fill" :style="{ width: (quotaPct ?? 0) + '%' }" />
              </div>
              <span class="tq-label mono">
                {{ fmtBytes(whoami.usageBytes ?? 0) }} / {{ fmtBytes(whoami.quotaBytes) }}
              </span>
            </div>
          </div>
          <div v-if="whoami.rateLimit?.maxRpm" class="tc-row">
            <span class="tc-key eyebrow">限流</span>
            <span class="tc-val mono">{{ whoami.rateLimit.maxRpm }} rpm</span>
          </div>
          <div v-if="whoami.retention?.maxSessions" class="tc-row">
            <span class="tc-key eyebrow">保留</span>
            <span class="tc-val mono">≤ {{ whoami.retention.maxSessions }} 会话</span>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.tenant-list {
  display: flex;
  flex-direction: column;
  padding: 22px 24px;
  gap: 18px;
  max-width: 720px;
}
.tl-head { margin-bottom: 4px; }
.tl-title {
  margin: 4px 0 0;
  font-size: var(--fs-xl);
  font-weight: 600;
  color: var(--bone);
  letter-spacing: -0.01em;
}
.tl-sub {
  margin: 6px 0 0;
  font-size: var(--fs-xs);
  color: var(--ash-deep);
}
.tenant-cards {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.tenant-card {
  background: var(--slate);
  border: 1px solid var(--hair);
  border-radius: var(--radius);
  padding: 16px 18px;
  cursor: pointer;
  transition: border-color 0.12s, box-shadow 0.12s;
}
.tenant-card:hover {
  border-color: var(--ash-deep);
  box-shadow: var(--shadow-1);
}
.tc-head {
  display: flex;
  align-items: center;
  gap: 10px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--hair-soft);
}
.tc-id {
  font-size: var(--fs-md);
  color: var(--bone);
  letter-spacing: 0.04em;
}
.tc-arrow {
  margin-left: auto;
  color: var(--ash-deep);
}
.tc-body {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding-top: 12px;
}
.tc-row {
  display: flex;
  align-items: center;
  gap: 12px;
}
.tc-key {
  width: 64px;
  flex-shrink: 0;
  font-size: 10px;
  color: var(--ash-deep);
}
.tc-val {
  font-size: var(--fs-sm);
  color: var(--bone-dim);
}
.tc-quota {
  display: flex;
  align-items: center;
  gap: 10px;
  flex: 1;
}
.tq-bar {
  flex: 1;
  height: 6px;
  background: var(--hair-soft);
  border-radius: 3px;
  overflow: hidden;
  max-width: 200px;
}
.tq-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--teal), var(--amber));
  transition: width 0.3s;
}
.tq-label {
  font-size: var(--fs-xs);
  color: var(--ash);
}
</style>
