<script setup lang="ts">
import { reactive, ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

/** 保留 tab：单租户保留策略（多租户 per-tenant 在 tenants.json 配置）。 */
type IngestStatus = {
  enabled: boolean;
  port: number;
  token: string;
  retainMax: number;
  listening: boolean;
  addr: string | null;
};

const form = reactive({ retainMax: 50 });
const status = ref<IngestStatus | null>(null);

async function load() {
  try {
    const s = await invoke<IngestStatus>("get_ingest_config");
    status.value = s;
    form.retainMax = s.retainMax;
  } catch (e) {
    ElMessage.error(`读取保留配置失败: ${e}`);
  }
}

async function save() {
  try {
    status.value = await invoke<IngestStatus>("set_ingest_config", {
      config: {
        enabled: status.value?.enabled ?? true,
        port: status.value?.port ?? 1421,
        token: status.value?.token ?? "",
        retainMax: form.retainMax,
      },
    });
    ElMessage.success("保留策略已保存");
  } catch (e) {
    ElMessage.error(`保存失败: ${e}`);
  }
}

onMounted(load);
</script>

<template>
  <div class="tab-pane">
    <div class="field-group">
      <div class="group-label eyebrow">会话保留</div>
      <el-form label-width="96px">
        <el-form-item label="最大会话数">
          <el-input-number v-model="form.retainMax" :min="0" :max="9999" controls-position="right" style="width: 140px" />
          <span class="field-hint">超出按 startedAt 倒序淘汰；0 = 不限</span>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="save">保存</el-button>
        </el-form-item>
      </el-form>
    </div>
    <p class="retention-hint">
      多租户模式下，保留策略在 <code class="mono">tenants.json</code> 的 per-tenant
      <code class="mono">retention</code> 字段配置，此处仅控制单租户模式。
    </p>
  </div>
</template>

<style scoped>
.tab-pane { display: flex; flex-direction: column; gap: 20px; max-width: 640px; }
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
.retention-hint {
  font-size: var(--fs-xs);
  color: var(--ash-deep);
  line-height: 1.6;
  margin: 0;
}
.retention-hint code {
  padding: 1px 5px;
  background: var(--slate-2);
  border: 1px solid var(--hair);
  border-radius: 3px;
  color: var(--ash);
}
</style>
