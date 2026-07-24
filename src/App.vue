<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from "vue";
import { RouterView } from "vue-router";
import { useRecorder } from "./composables/useRecorder";
import { isTauri, currentWindowLabel } from "./composables/tauri";
import { loadBackendConfig } from "./composables/backend";
import LoginGate from "./views/LoginGate.vue";

/**
 * App 根组件：
 * - 浏览器非 Tauri：无 backend 配置（endpoint+key）时显示 LoginGate；验证后进主应用。
 * - Tauri 桌面：默认本地模式，无登录墙。
 * - useRecorder 仅在 Tauri + 非 player 窗口挂载（player 不录制自身）。
 */
const tauri = isTauri();
// 浏览器模式默认显示登录墙（避免主应用先闪一下再出登录）；init 后若已有配置则隐藏。
const showLogin = ref(!tauri);
const ready = ref(false);
const label = ref("main");

async function init() {
  label.value = await currentWindowLabel();
  if (tauri) {
    // Tauri 桌面：直接进主应用
    ready.value = true;
    return;
  }
  // 浏览器：检查 backend 配置，缺 endpoint 则保持登录墙
  const cfg = loadBackendConfig();
  showLogin.value = !cfg.endpoint;
  ready.value = true;
}

// 录制器：仅 Tauri + 非 player-* 窗口挂载（player 窗口不录制自身回放）
let recorder: ReturnType<typeof useRecorder> | null = null;
onMounted(async () => {
  await init();
  if (tauri && !label.value.startsWith("player-")) {
    recorder = useRecorder();
  }
});

function onLoginSuccess() {
  showLogin.value = false;
}

onBeforeUnmount(() => recorder?.destroy());
</script>

<template>
  <!-- 浏览器未就绪时白屏（避免闪主应用）；Tauri 就绪快 -->
  <template v-if="!ready">
    <div class="app-boot" />
  </template>
  <LoginGate v-else-if="!tauri && showLogin" @success="onLoginSuccess" />
  <RouterView v-else />
</template>

<style>
.app-boot {
  width: 100vw;
  height: 100vh;
  background: var(--ink);
}
</style>
