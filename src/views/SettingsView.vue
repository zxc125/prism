<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import {
  loadBackendConfig,
  saveBackendConfig,
  resetBackend,
  type BackendConfig,
} from "../composables/backend";

type IngestStatus = {
  enabled: boolean;
  port: number;
  token: string;
  listening: boolean;
  addr: string | null;
};

const form = reactive({
  theme: "dark",
  autoStart: false,
  // 采集（采集器固定全开；按需开关待接线）
  captureErrors: true,
  captureConsole: true,
  captureNetwork: true,
  captureNetBody: false,
  // 接收（P4 落地生效）
  serverEnabled: true,
  serverPort: 1421,
  serverToken: "",
  // 保留
  retainMax: 50,
});

const status = ref<IngestStatus | null>(null);

// 云端连接（P8）：本地 invoke / 云端 HTTP。存 localStorage，切到云端后数据读/管理走 endpoint。
const backend = reactive<BackendConfig>(loadBackendConfig());

async function load() {
  try {
    const s = await invoke<IngestStatus>("get_ingest_config");
    status.value = s;
    form.serverEnabled = s.enabled;
    form.serverPort = s.port;
    form.serverToken = s.token;
  } catch (e) {
    ElMessage.error(`读取接收配置失败: ${e}`);
  }
}

async function save() {
  try {
    status.value = await invoke<IngestStatus>("set_ingest_config", {
      config: {
        enabled: form.serverEnabled,
        port: form.serverPort,
        token: form.serverToken,
      },
    });
    ElMessage.success("接收设置已保存（端口修改重启生效）");
  } catch (e) {
    ElMessage.error(`保存失败: ${e}`);
  }
}

function saveBackend() {
  // 切到 http 但未填 endpoint 时退回本地，避免空指针
  if (backend.mode === "http" && !backend.endpoint.trim()) {
    ElMessage.warning("云端模式需要填写 endpoint，已退回本地");
    backend.mode = "tauri";
  }
  saveBackendConfig({ ...backend });
  resetBackend();
  ElMessage.success(
    backend.mode === "http" ? "已切换到云端连接" : "已切换到本地模式",
  );
}

const endpoint = computed(() =>
  status.value?.addr ? `http://${status.value.addr}` : `http://127.0.0.1:${form.serverPort}`,
);

onMounted(load);
</script>

<template>
  <main class="settings">
    <header class="settings-head">
      <span class="eyebrow">偏好</span>
      <h1 class="settings-title">设置</h1>
      <p class="settings-sub">接收项（P4）已生效；采集开关固定全开，按需过滤待后续接线。</p>
    </header>

    <el-form label-width="96px" class="form">
      <div class="field-group">
        <div class="group-label eyebrow">外观</div>
        <el-form-item label="主题">
          <el-select v-model="form.theme" style="width: 200px">
            <el-option label="深色 · 控制台" value="dark" />
            <el-option label="浅色" value="light" />
          </el-select>
        </el-form-item>
      </div>

      <div class="field-group">
        <div class="group-label eyebrow">采集</div>
        <el-form-item label="错误">
          <el-switch v-model="form.captureErrors" />
          <span class="field-hint">window.onerror / unhandledrejection</span>
        </el-form-item>
        <el-form-item label="Console">
          <el-switch v-model="form.captureConsole" />
          <span class="field-hint">log / warn / error / info / debug</span>
        </el-form-item>
        <el-form-item label="网络">
          <el-switch v-model="form.captureNetwork" />
          <span class="field-hint">fetch / XMLHttpRequest</span>
        </el-form-item>
        <el-form-item label="请求体">
          <el-switch v-model="form.captureNetBody" />
          <span class="field-hint">默认关 · 含 PII 风险</span>
        </el-form-item>
      </div>

      <div class="field-group">
        <div class="group-label eyebrow">接收</div>
        <el-form-item label="HTTP 服务">
          <el-switch v-model="form.serverEnabled" />
          <span class="field-hint">监听 127.0.0.1 · 接收外部上报</span>
        </el-form-item>
        <el-form-item label="端口">
          <el-input-number
            v-model="form.serverPort"
            :min="1024"
            :max="65535"
            controls-position="right"
            style="width: 140px"
          />
          <span class="field-hint">修改后重启生效</span>
        </el-form-item>
        <el-form-item label="鉴权 token">
          <el-input
            v-model="form.serverToken"
            placeholder="留空 = 不鉴权（本机回环）"
            style="width: 260px"
          />
          <span class="field-hint">SDK init 传入同一 token</span>
        </el-form-item>
        <el-form-item label="接入点">
          <code class="endpoint mono">{{ endpoint }}</code>
          <span class="field-hint">
            {{ status?.listening ? (status.enabled ? "监听中" : "已停用") : "未监听 · 端口占用？" }}
          </span>
        </el-form-item>
      </div>

      <div class="field-group">
        <div class="group-label eyebrow">云端连接</div>
        <el-form-item label="数据来源">
          <el-radio-group v-model="backend.mode">
            <el-radio value="tauri">本地</el-radio>
            <el-radio value="http">云端</el-radio>
          </el-radio-group>
          <span class="field-hint">本地 = invoke；云端 = HTTP 调自托管 server</span>
        </el-form-item>
        <el-form-item v-if="backend.mode === 'http'" label="Endpoint">
          <el-input
            v-model="backend.endpoint"
            placeholder="https://obs.example.com"
            style="width: 280px"
          />
          <span class="field-hint">自托管 observer-server 地址</span>
        </el-form-item>
        <el-form-item v-if="backend.mode === 'http'" label="API Key">
          <el-input
            v-model="backend.apiKey"
            placeholder="Bearer token（server --token）"
            style="width: 280px"
            show-password
          />
          <span class="field-hint">Authorization: Bearer &lt;key&gt;</span>
        </el-form-item>
        <el-form-item v-if="backend.mode === 'http'">
          <el-button type="primary" @click="saveBackend">应用云端连接</el-button>
        </el-form-item>
        <el-form-item v-else>
          <el-button @click="saveBackend">应用本地模式</el-button>
        </el-form-item>
      </div>

      <div class="field-group">
        <div class="group-label eyebrow">保留</div>
        <el-form-item label="最大会话数">
          <el-input-number
            v-model="form.retainMax"
            :min="1"
            :max="9999"
            controls-position="right"
            style="width: 140px"
          />
          <span class="field-hint">超出按时间倒序淘汰</span>
        </el-form-item>
      </div>

      <div class="field-group">
        <div class="group-label eyebrow">系统</div>
        <el-form-item label="开机自启">
          <el-switch v-model="form.autoStart" />
          <span class="field-hint">登录后自动启动并进入待机</span>
        </el-form-item>
      </div>
    </el-form>

    <footer class="settings-foot">
      <el-button type="primary" @click="save">保存设置</el-button>
    </footer>
  </main>
</template>

<style scoped>
.settings {
  display: flex;
  flex-direction: column;
  height: 100vh;
  padding: 22px 24px;
  background: var(--ink);
}
.settings-head {
  margin-bottom: 20px;
}
.settings-title {
  margin: 4px 0 0;
  font-size: var(--fs-xl);
  font-weight: 600;
  color: var(--bone);
  letter-spacing: -0.01em;
}
.settings-sub {
  margin: 6px 0 0;
  font-size: var(--fs-xs);
  color: var(--ash-deep);
}
.form {
  flex: 1;
  overflow: auto;
}
.field-group {
  margin-bottom: 24px;
}
.group-label {
  padding-bottom: 8px;
  margin-bottom: 4px;
  border-bottom: 1px solid var(--hair-soft);
}
.field-hint {
  color: var(--ash-deep);
  font-size: var(--fs-xs);
  margin-left: 12px;
}
.endpoint {
  padding: 3px 8px;
  background: var(--slate-2);
  border: 1px solid var(--hair);
  border-radius: var(--radius-sm);
  color: var(--src-web);
}
.settings-foot {
  padding-top: 16px;
  border-top: 1px solid var(--hair-soft);
}
</style>
