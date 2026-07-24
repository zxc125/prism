<script setup lang="ts">
import { reactive, ref, onMounted, computed } from "vue";
import {
  loadBackendConfig,
  saveBackendConfig,
  resetBackend,
  getBackend,
  type BackendConfig,
  type Whoami,
} from "../../composables/backend";
import { isTauri } from "../../composables/tauri";
import { fmtBytes } from "../common/format";
import StatusDot from "../common/StatusDot.vue";

/** 连接 tab：云端 endpoint + key + 当前租户信息块。
 *  P10：浏览器登录后这里显示已连接租户；Tauri 桌面可切本地/云端。 */
const tauri = isTauri();
const backend = reactive<BackendConfig>(loadBackendConfig());
const whoami = ref<Whoami | null>(null);

async function loadWhoami() {
  try {
    whoami.value = await getBackend().whoami();
  } catch (e) {
    console.error("[settings] whoami failed", e);
  }
}

function saveBackend() {
  if (backend.mode === "http" && !backend.endpoint.trim()) {
    ElMessage.warning("云端模式需要填写 endpoint，已退回本地");
    backend.mode = "tauri";
  }
  saveBackendConfig({ ...backend });
  resetBackend();
  ElMessage.success(backend.mode === "http" ? "已切换到云端连接" : "已切换到本地模式");
  void loadWhoami();
}

function logout() {
  localStorage.removeItem("observer-backend");
  window.location.reload();
}

onMounted(loadWhoami);

const quotaPct = computed(() => {
  const w = whoami.value;
  if (!w?.multiTenant || !w.quotaBytes) return null;
  return Math.min(100, ((w.usageBytes ?? 0) / w.quotaBytes) * 100);
});
</script>

<template>
  <div class="tab-pane">
    <!-- 当前连接状态 -->
    <div class="field-group">
      <div class="group-label eyebrow">当前连接</div>
      <div class="conn-block">
        <StatusDot
          :color="backend.mode === 'http' ? 'var(--amber)' : 'var(--teal)'"
          :glow="backend.mode === 'http'"
          :size="8"
        />
        <div class="conn-info">
          <div class="conn-mode mono">
            {{ tauri ? (backend.mode === "http" ? "云端" : "本地") : "浏览器 → 云端" }}
          </div>
          <div v-if="backend.endpoint" class="conn-ep mono">{{ backend.endpoint }}</div>
        </div>
        <div v-if="whoami?.multiTenant" class="conn-tenant">
          <span class="ct-label eyebrow">tenant</span>
          <span class="ct-id mono">{{ whoami.tenantId }}</span>
          <div v-if="quotaPct !== null" class="ct-quota">
            <div class="cq-bar"><div class="cq-fill" :style="{ width: quotaPct + '%' }" /></div>
            <span class="cq-label mono">
              {{ fmtBytes(whoami.usageBytes ?? 0) }} / {{ fmtBytes(whoami.quotaBytes ?? 0) }}
            </span>
          </div>
        </div>
      </div>
    </div>

    <!-- 数据来源切换（仅 Tauri；浏览器恒为云端） -->
    <div v-if="tauri" class="field-group">
      <div class="group-label eyebrow">数据来源</div>
      <el-form label-width="96px">
        <el-form-item label="模式">
          <el-radio-group v-model="backend.mode">
            <el-radio value="tauri">本地</el-radio>
            <el-radio value="http">云端</el-radio>
          </el-radio-group>
          <span class="field-hint">本地 = invoke；云端 = HTTP 调自托管 server</span>
        </el-form-item>
        <el-form-item v-if="backend.mode === 'http'" label="Endpoint">
          <el-input v-model="backend.endpoint" placeholder="https://obs.example.com" style="width: 280px" />
        </el-form-item>
        <el-form-item v-if="backend.mode === 'http'" label="API Key">
          <el-input v-model="backend.apiKey" placeholder="Bearer token" style="width: 280px" show-password />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="saveBackend">应用</el-button>
        </el-form-item>
      </el-form>
    </div>

    <!-- 浏览器：退出登录 -->
    <div v-else class="field-group">
      <div class="group-label eyebrow">会话</div>
      <el-button @click="logout">退出登录</el-button>
      <span class="field-hint">清除本地 endpoint + key，返回登录页</span>
    </div>
  </div>
</template>

<style scoped>
.tab-pane { display: flex; flex-direction: column; gap: 20px; }
.field-group {
  background: var(--slate);
  border: 1px solid var(--hair);
  border-radius: var(--radius);
  padding: 16px 18px;
}
.group-label {
  padding-bottom: 10px;
  margin-bottom: 12px;
  border-bottom: 1px solid var(--hair-soft);
}
.conn-block {
  display: flex;
  align-items: center;
  gap: 12px;
}
.conn-info { display: flex; flex-direction: column; gap: 2px; }
.conn-mode { font-size: var(--fs-sm); color: var(--bone); }
.conn-ep { font-size: var(--fs-xs); color: var(--ash); }
.conn-tenant {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 10px;
}
.ct-label { font-size: 10px; color: var(--ash-deep); }
.ct-id { font-size: var(--fs-sm); color: var(--amber); }
.ct-quota { display: flex; align-items: center; gap: 8px; }
.cq-bar { width: 80px; height: 4px; background: var(--hair-soft); border-radius: 2px; overflow: hidden; }
.cq-fill { height: 100%; background: linear-gradient(90deg, var(--teal), var(--amber)); }
.cq-label { font-size: 10px; color: var(--ash); }
.field-hint { color: var(--ash-deep); font-size: var(--fs-xs); margin-left: 12px; }
</style>
