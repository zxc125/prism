<script setup lang="ts">
import AppShell from "../components/shell/AppShell.vue";
import SourceRack from "../components/live/SourceRack.vue";
import EmptyState from "../components/common/EmptyState.vue";
import { isTauri } from "../composables/tauri";

/** Live 视图：源机架 + 录制控制。
 *  浏览器无法自录（D6）：显示提示，引导用 web/tauri SDK 上报。 */
const tauri = isTauri();
</script>

<template>
  <AppShell>
    <section class="live-view">
      <header class="lv-head">
        <div class="eyebrow">实时观测</div>
        <h1 class="lv-title">源机架</h1>
        <p class="lv-sub">本机自录 + 外部 web/tauri 通道上报，统一落 console 本地 server。</p>
      </header>

      <SourceRack v-if="tauri" />

      <EmptyState
        v-else
        icon="◌"
        title="浏览器无法自录"
        hint="浏览器环境没有 Tauri 录制能力。请用 web SDK（@prism-obs/observer-sdk）上报到自托管 server，或在 Tauri 桌面端使用本机通道。"
      />
    </section>
  </AppShell>
</template>

<style scoped>
.live-view {
  display: flex;
  flex-direction: column;
  padding: 22px 24px;
  gap: 18px;
  max-width: 720px;
}
.lv-head { margin-bottom: 4px; }
.lv-title {
  margin: 4px 0 0;
  font-size: var(--fs-xl);
  font-weight: 600;
  color: var(--bone);
  letter-spacing: -0.01em;
}
.lv-sub {
  margin: 6px 0 0;
  font-size: var(--fs-xs);
  color: var(--ash-deep);
}
</style>
