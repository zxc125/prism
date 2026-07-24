<script setup lang="ts">
import { ref, onMounted } from "vue";
import { resetBackend } from "../../composables/backend";
import SideNav from "./SideNav.vue";
import TopBar from "./TopBar.vue";
import CommandPalette from "./CommandPalette.vue";

/** app 骨架：侧栏 + 顶栏 + 主内容区。RouterView 由调用方注入默认 slot。 */
const topbar = ref<InstanceType<typeof TopBar>>();
const cmdkOpen = ref(false);

onMounted(() => {
  // cfg 变更后（设置页保存或其它标签改 localStorage）刷新顶栏：
  // 先 resetBackend 缓存让 getBackend 重建，再 whoami 重读
  window.addEventListener("storage", () => {
    resetBackend();
    topbar.value?.refresh();
  });
});

function onCmdk() {
  cmdkOpen.value = true;
}
</script>

<template>
  <div class="app-shell">
    <SideNav />
    <div class="shell-main">
      <TopBar ref="topbar" @cmdk="onCmdk" />
      <main class="shell-content">
        <slot />
      </main>
    </div>
    <CommandPalette v-model:open="cmdkOpen" />
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  height: 100vh;
  background: var(--ink);
}
.shell-main {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
}
.shell-content {
  flex: 1;
  min-height: 0;
  overflow: auto;
}
</style>
