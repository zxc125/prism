<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { getBackend, type Whoami, type SessionMeta } from "../../composables/backend";
import { sourceOf, SRC_LABEL, fmtClock, sessionDur, fmtBytes } from "../common/format";
import StatusDot from "../common/StatusDot.vue";
import EmptyState from "../common/EmptyState.vue";

/** Tenant 详情：配额用量 / 会话数 / 保留 / redact（只读）。
 *  数据源：/whoami + /sessions 列表。 */
const props = defineProps<{ id: string }>();
const router = useRouter();

const whoami = ref<Whoami | null>(null);
const sessions = ref<SessionMeta[]>([]);
const loading = ref(true);

async function refresh() {
  loading.value = true;
  try {
    const [w, ss] = await Promise.all([
      getBackend().whoami(),
      getBackend().listSessions(),
    ]);
    whoami.value = w;
    sessions.value = ss;
  } catch (e) {
    console.error("[tenant-detail] load failed", e);
  } finally {
    loading.value = false;
  }
}

onMounted(refresh);

const isCurrent = computed(() => whoami.value?.tenantId === props.id);
const quotaPct = computed(() => {
  const w = whoami.value;
  if (!w?.quotaBytes) return null;
  return Math.min(100, ((w.usageBytes ?? 0) / w.quotaBytes) * 100);
});
</script>

<template>
  <section class="tenant-detail">
    <header class="td-head">
      <div class="td-crumb">
        <RouterLink to="/tenants" class="crumb-link">租户</RouterLink>
        <span class="crumb-sep">/</span>
        <span class="crumb-current mono">{{ id }}</span>
      </div>
    </header>

    <EmptyState
      v-if="!loading && !isCurrent"
      icon="⊘"
      title="租户不可见"
      hint="当前 API key 无权访问此租户，或 server 未启用多租户。"
    />

    <template v-else-if="whoami?.multiTenant">
      <div class="td-card">
        <div class="td-card-head">
          <StatusDot color="var(--amber)" :glow="true" :size="10" />
          <span class="td-id mono">{{ whoami.tenantId }}</span>
        </div>

        <div class="td-rows">
          <div class="td-row">
            <span class="td-key eyebrow">appIds</span>
            <span class="td-val mono">{{ whoami.appIds?.length ? whoami.appIds.join(", ") : "（不校验）" }}</span>
          </div>

          <div v-if="whoami.quotaBytes" class="td-row">
            <span class="td-key eyebrow">配额</span>
            <div class="td-quota">
              <div class="tq-bar">
                <div class="tq-fill" :style="{ width: (quotaPct ?? 0) + '%' }" />
              </div>
              <span class="tq-label mono">
                {{ fmtBytes(whoami.usageBytes ?? 0) }} / {{ fmtBytes(whoami.quotaBytes) }}
                ({{ (quotaPct ?? 0).toFixed(1) }}%)
              </span>
            </div>
          </div>

          <div v-if="whoami.rateLimit?.maxRpm" class="td-row">
            <span class="td-key eyebrow">限流</span>
            <span class="td-val mono">{{ whoami.rateLimit.maxRpm }} 请求/分钟</span>
          </div>

          <div class="td-row">
            <span class="td-key eyebrow">保留</span>
            <span class="td-val mono">
              {{ whoami.retention?.maxSessions ? `≤ ${whoami.retention.maxSessions} 会话` : "不限" }}
              {{ whoami.retention?.maxAgeDays ? `· ${whoami.retention.maxAgeDays} 天` : "" }}
            </span>
          </div>

          <div class="td-row">
            <span class="td-key eyebrow">会话数</span>
            <span class="td-val mono">{{ sessions.length }}</span>
          </div>
        </div>
      </div>

      <!-- 此租户的会话快览 -->
      <div class="td-sessions">
        <div class="eyebrow td-sess-title">会话快览 · 前 10 条</div>
        <div v-if="!sessions.length" class="td-sess-empty">暂无会话</div>
        <div v-for="s in sessions.slice(0, 10)" :key="s.id" class="td-sess-row" @click="router.push(`/s/${s.id}`)">
          <StatusDot :color="`var(--src-${sourceOf(s)})`" :size="7" />
          <span class="ts-src mono">{{ SRC_LABEL[sourceOf(s)] }}</span>
          <span class="ts-time mono">{{ fmtClock(s.startedAt) }}</span>
          <span class="ts-dur mono">{{ sessionDur(s) }}</span>
          <span class="ts-id mono">{{ s.name || s.id }}</span>
        </div>
      </div>
    </template>
  </section>
</template>

<style scoped>
.tenant-detail {
  display: flex;
  flex-direction: column;
  padding: 22px 24px;
  gap: 18px;
  max-width: 720px;
}
.td-head { margin-bottom: 4px; }
.td-crumb {
  display: flex;
  align-items: center;
  gap: 8px;
}
.crumb-link {
  color: var(--ash);
  text-decoration: none;
  font-size: var(--fs-sm);
  transition: color 0.12s;
}
.crumb-link:hover { color: var(--bone-dim); }
.crumb-sep { color: var(--ash-deep); font-size: var(--fs-sm); }
.crumb-current {
  color: var(--bone);
  font-size: var(--fs-sm);
  letter-spacing: 0.04em;
}
.td-card {
  background: var(--slate);
  border: 1px solid var(--hair);
  border-radius: var(--radius);
  padding: 18px;
}
.td-card-head {
  display: flex;
  align-items: center;
  gap: 10px;
  padding-bottom: 14px;
  border-bottom: 1px solid var(--hair-soft);
}
.td-id {
  font-size: var(--fs-lg);
  color: var(--bone);
  letter-spacing: 0.04em;
}
.td-rows {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding-top: 14px;
}
.td-row {
  display: flex;
  align-items: center;
  gap: 16px;
}
.td-key {
  width: 72px;
  flex-shrink: 0;
  font-size: 10px;
  color: var(--ash-deep);
}
.td-val {
  font-size: var(--fs-sm);
  color: var(--bone-dim);
}
.td-quota {
  display: flex;
  align-items: center;
  gap: 12px;
  flex: 1;
}
.tq-bar {
  flex: 1;
  height: 6px;
  background: var(--hair-soft);
  border-radius: 3px;
  overflow: hidden;
  max-width: 240px;
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
.td-sessions {
  background: var(--slate);
  border: 1px solid var(--hair);
  border-radius: var(--radius);
  padding: 14px 18px;
}
.td-sess-title {
  padding-bottom: 10px;
  border-bottom: 1px solid var(--hair-soft);
  margin-bottom: 6px;
}
.td-sess-empty {
  padding: 20px;
  text-align: center;
  color: var(--ash-deep);
  font-size: var(--fs-sm);
}
.td-sess-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 4px;
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: background 0.12s;
}
.td-sess-row:hover { background: var(--slate-2); }
.ts-src {
  font-size: var(--fs-xs);
  color: var(--bone-dim);
  width: 40px;
}
.ts-time { font-size: var(--fs-xs); color: var(--bone-dim); }
.ts-dur { font-size: var(--fs-xs); color: var(--ash); width: 50px; }
.ts-id {
  font-size: var(--fs-xs);
  color: var(--ash-deep);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
