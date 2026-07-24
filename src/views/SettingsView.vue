<script setup lang="ts">
import { ref, computed } from "vue";
import AppShell from "../components/shell/AppShell.vue";
import ConnectionTab from "../components/settings/ConnectionTab.vue";
import CaptureTab from "../components/settings/CaptureTab.vue";
import RetentionTab from "../components/settings/RetentionTab.vue";
import AboutTab from "../components/settings/AboutTab.vue";
import { isTauri } from "../composables/tauri";

/** 设置页：分 tab。Capture/Retention 仅 Tauri（浏览器无本地 server）。 */
const tauri = isTauri();
type SettingsTab = "connection" | "capture" | "retention" | "about";
const activeTab = ref<SettingsTab>("connection");

const tabs = computed<{ key: SettingsTab; label: string }[]>(() => {
  const list: { key: SettingsTab; label: string }[] = [
    { key: "connection", label: "连接" },
    { key: "about", label: "关于" },
  ];
  if (tauri) {
    list.splice(1, 0, { key: "capture", label: "采集" }, { key: "retention", label: "保留" });
  }
  return list;
});
</script>

<template>
  <AppShell>
    <section class="settings-view">
      <header class="sv-head">
        <div class="eyebrow">偏好</div>
        <h1 class="sv-title">设置</h1>
      </header>

      <el-tabs v-model="activeTab" class="settings-tabs">
        <el-tab-pane
          v-for="t in tabs"
          :key="t.key"
          :label="t.label"
          :name="t.key"
        >
          <ConnectionTab v-if="t.key === 'connection'" />
          <CaptureTab v-else-if="t.key === 'capture'" />
          <RetentionTab v-else-if="t.key === 'retention'" />
          <AboutTab v-else />
        </el-tab-pane>
      </el-tabs>
    </section>
  </AppShell>
</template>

<style scoped>
.settings-view {
  display: flex;
  flex-direction: column;
  padding: 22px 24px;
  height: 100%;
}
.sv-head { margin-bottom: 16px; }
.sv-title {
  margin: 4px 0 0;
  font-size: var(--fs-xl);
  font-weight: 600;
  color: var(--bone);
  letter-spacing: -0.01em;
}
.settings-tabs {
  flex: 1;
  min-height: 0;
}
.settings-tabs :deep(.el-tabs__content) {
  overflow-y: auto;
  padding-top: 8px;
}
</style>
