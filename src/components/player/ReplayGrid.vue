<script setup lang="ts">
import { ref, inject, onMounted, onBeforeUnmount } from "vue";
import { PLAYER_CTX, type PlayerCtx } from "./context";

/** 回放网格容器：usePlayer 用 document.createElement 动态创建 tile 元素挂到此 div。
 *  样式（tile-slot / tile-header / tile-root 等）放在 PlayerShell 的非 scoped 块里
 *  （因为动态元素不带 data-v 属性，scoped 选择器不命中）。 */
const gridRef = ref<HTMLElement>();
const ctx = inject<PlayerCtx>(PLAYER_CTX);
if (!ctx) throw new Error("ReplayGrid 必须在 PlayerShell 内使用");

onMounted(() => {
  if (gridRef.value) ctx.player.attachGrid(gridRef.value);
});
onBeforeUnmount(() => ctx.player.destroy());
</script>

<template>
  <div ref="gridRef" class="grid" />
</template>

<style scoped>
.grid {
  min-height: 0;
  display: grid;
  gap: 8px;
  padding: 12px;
  background: var(--ink);
}
</style>
