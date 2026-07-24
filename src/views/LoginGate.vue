<script setup lang="ts">
import { ref } from "vue";
import {
  saveBackendConfig,
  resetBackend,
  HttpBackend,
  type Whoami,
} from "../composables/backend";

/** 浏览器登录墙：无有效 backend 配置（endpoint+key）且非 Tauri 时显示。
 *  提交 -> 调 whoami() 验证 -> 成功存 localStorage 进主应用；失败提示。
 *  Tauri 桌面跳过（默认本地模式）。
 *
 *  UX：浏览器模式下 console 是从 observer-server 托管的，endpoint 几乎总是
 *  = 当前 origin，所以预填 window.location.origin，避免手填易错（漏 http://、
 *  多 /、端口打错）。用户仍可改填别的 endpoint（如反代域名）。 */
const emit = defineEmits<{ (e: "success"): void }>();

const endpoint = ref(
  typeof window !== "undefined" ? window.location.origin : "",
);
const apiKey = ref("");
const checking = ref(false);
const error = ref("");

async function submit() {
  if (!endpoint.value.trim()) {
    error.value = "请填写 endpoint";
    return;
  }
  checking.value = true;
  error.value = "";
  try {
    const ep = endpoint.value.trim().replace(/\/$/, "");
    const backend = new HttpBackend({ endpoint: ep, apiKey: apiKey.value.trim() });
    const w: Whoami = await backend.whoami();
    // 验证通过：保存配置
    saveBackendConfig({
      mode: "http",
      endpoint: ep,
      apiKey: apiKey.value.trim(),
    });
    resetBackend();
    emit("success");
    ElMessage.success(w.multiTenant ? `已连接 · 租户 ${w.tenantId}` : "已连接");
  } catch (e) {
    error.value = `连接失败: ${e}`;
  } finally {
    checking.value = false;
  }
}
</script>

<template>
  <div class="login-gate">
    <div class="lg-card">
      <div class="lg-brand">
        <span class="lg-mark" aria-hidden="true">
          <svg viewBox="0 0 24 24" width="32" height="32" fill="none">
            <rect x="2" y="4" width="20" height="16" rx="2.5" stroke="currentColor" stroke-width="1.5" />
            <circle cx="6" cy="8" r="1" fill="currentColor" />
            <circle cx="6" cy="12" r="1" fill="currentColor" />
            <circle cx="6" cy="16" r="1" fill="currentColor" />
            <path d="M11 12l4 2-4 2z" fill="currentColor" />
          </svg>
        </span>
        <div>
          <div class="lg-title">replay · observer</div>
          <div class="lg-sub eyebrow">连接到自托管 server</div>
        </div>
      </div>

      <form class="lg-form" @submit.prevent="submit">
        <label class="lg-field">
          <span class="lg-label eyebrow">Endpoint</span>
          <el-input
            v-model="endpoint"
            placeholder="https://obs.example.com"
            size="large"
            autofocus
          />
        </label>
        <label class="lg-field">
          <span class="lg-label eyebrow">API Key</span>
          <el-input
            v-model="apiKey"
            placeholder="Bearer token（server --token / tenants.json key）"
            size="large"
            show-password
          />
        </label>
        <div v-if="error" class="lg-error">{{ error }}</div>
        <el-button
          type="primary"
          size="large"
          :loading="checking"
          class="lg-submit"
          @click="submit"
        >
          {{ checking ? "验证中…" : "连接" }}
        </el-button>
      </form>

      <p class="lg-hint">
        key 存 localStorage，跨标签共享。自托管私有云威胁模型可控；
        未来可换 httpOnly cookie + session。
      </p>
    </div>
  </div>
</template>

<style scoped>
.login-gate {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100vh;
  background: var(--ink);
  padding: 24px;
}
.lg-card {
  width: 400px;
  max-width: 92vw;
  background: var(--slate);
  border: 1px solid var(--hair);
  border-radius: var(--radius-lg);
  padding: 32px;
  box-shadow: var(--shadow-3);
}
.lg-brand {
  display: flex;
  align-items: center;
  gap: 14px;
  padding-bottom: 24px;
  margin-bottom: 24px;
  border-bottom: 1px solid var(--hair-soft);
}
.lg-mark {
  color: var(--amber);
  display: flex;
  filter: drop-shadow(0 0 10px var(--amber-glow));
}
.lg-title {
  font-size: var(--fs-lg);
  font-weight: 600;
  color: var(--bone);
}
.lg-sub {
  font-size: 10px;
  color: var(--ash-deep);
  margin-top: 2px;
}
.lg-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.lg-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.lg-label { font-size: 10px; color: var(--ash-deep); }
.lg-error {
  color: var(--oxblood-soft);
  font-size: var(--fs-xs);
  padding: 8px 12px;
  background: var(--oxblood-tint);
  border-radius: var(--radius-sm);
}
.lg-submit { width: 100%; margin-top: 4px; }
.lg-hint {
  margin: 20px 0 0;
  font-size: var(--fs-xs);
  color: var(--ash-deep);
  line-height: 1.6;
}
</style>
