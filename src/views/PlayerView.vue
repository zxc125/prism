<script setup lang="ts">
import { useRoute } from "vue-router";
import PlayerShell from "../components/player/PlayerShell.vue";

/** /s/:id 路由入口：薄包装，逻辑全部在 PlayerShell + 子组件。
 *  保留独立文件以维持现有 import 兼容（capabilities 等）。 */
const route = useRoute();
const id = route.params.id as string;
</script>

<template>
  <PlayerShell :id="id" />
</template>
