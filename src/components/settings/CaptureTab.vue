<script setup lang="ts">
import { reactive, ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

/** 采集 tab：HTTP 接收 server 配置 + 采集信号开关。
 *  仅 Tauri（浏览器无法自录，无本地 server）。 */
type IngestStatus = {
  enabled: boolean;
  port: number;
  token: string;
  listening: boolean;
  addr: string | null;
};

const form = reactive({
  captureErrors: true,
  captureConsole: true,
  captureNetwork: true,
  captureNetBody: false,
  serverEnabled: true,
  serverPort: 1421,
  serverToken: "",
});

const status = ref<IngestStatus | null>(null);

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
        // retainMax 在 RetentionTab 维护，这里读回保持不变
        retainMax: status.value ? (status.value as any).retainMax ?? 50 : 50,
      },
    });
    ElMessage.success("接收设置已保存（端口修改重启生效）");
  } catch (e) {
    ElMessage.error(`保存失败: ${e}`);
  }
}

const endpoint = computed(() =>
  status.value?.addr ? `http://${status.value.addr}` : `http://127.0.0.1:${form.serverPort}`,
);

onMounted(load);
</script>

<template>
  <div class="tab-pane">
    <div class="field-group">
      <div class="group-label eyebrow">采集信号</div>
      <el-form label-width="96px">
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
      </el-form>
    </div>

    <div class="field-group">
      <div class="group-label eyebrow">HTTP 接收</div>
      <el-form label-width="96px">
        <el-form-item label="HTTP 服务">
          <el-switch v-model="form.serverEnabled" />
          <span class="field-hint">监听 127.0.0.1 · 接收外部上报</span>
        </el-form-item>
        <el-form-item label="端口">
          <el-input-number v-model="form.serverPort" :min="1024" :max="65535" controls-position="right" style="width: 140px" />
          <span class="field-hint">修改后重启生效</span>
        </el-form-item>
        <el-form-item label="鉴权 token">
          <el-input v-model="form.serverToken" placeholder="留空 = 不鉴权（本机回环）" style="width: 260px" />
          <span class="field-hint">SDK init 传入同一 token</span>
        </el-form-item>
        <el-form-item label="接入点">
          <code class="endpoint mono">{{ endpoint }}</code>
          <span class="field-hint">
            {{ status?.listening ? (status.enabled ? "监听中" : "已停用") : "未监听 · 端口占用？" }}
          </span>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="save">保存</el-button>
        </el-form-item>
      </el-form>
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
.field-hint { color: var(--ash-deep); font-size: var(--fs-xs); margin-left: 12px; }
.endpoint {
  padding: 3px 8px;
  background: var(--slate-2);
  border: 1px solid var(--hair);
  border-radius: var(--radius-sm);
  color: var(--teal);
}
</style>
