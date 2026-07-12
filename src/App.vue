<script setup lang="ts">
import { RouterView } from "vue-router";
import { useRecorder } from "./composables/useRecorder";

// 每个窗口挂载即启用录制器：监听会话广播与段控制
const recorder = useRecorder();
onBeforeUnmount(() => recorder.destroy());
</script>

<template>
  <RouterView />
</template>

<style>
html,
body,
#app {
  margin: 0;
  min-height: 100vh;
}
</style>
