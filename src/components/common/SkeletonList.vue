<script setup lang="ts">
/** 骨架屏：列表/卡片加载态，取代 spinner-only。 */
defineProps<{ rows?: number }>();
</script>

<template>
  <div class="skeleton-list">
    <div v-for="i in rows ?? 6" :key="i" class="sk-row">
      <div class="sk-dot" />
      <div class="sk-line w-sm" />
      <div class="sk-line w-md" />
      <div class="sk-line w-lg" />
      <div class="sk-spacer" />
      <div class="sk-line w-btn" />
    </div>
  </div>
</template>

<style scoped>
.skeleton-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 4px 0;
}
.sk-row {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 12px 12px;
}
.sk-dot,
.sk-line {
  background: linear-gradient(
    90deg,
    var(--slate-2) 0%,
    var(--slate-3) 50%,
    var(--slate-2) 100%
  );
  background-size: 200% 100%;
  animation: sk-shimmer 1.6s ease-in-out infinite;
  border-radius: var(--radius-sm);
}
.sk-dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  flex-shrink: 0;
}
.sk-line {
  height: 12px;
}
.w-sm { width: 44px; }
.w-md { width: 80px; }
.w-lg { width: 120px; }
.w-btn { width: 60px; }
.sk-spacer { flex: 1; }
@keyframes sk-shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
@media (prefers-reduced-motion: reduce) {
  .sk-dot, .sk-line { animation: none; }
}
</style>
