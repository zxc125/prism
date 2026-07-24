<script setup lang="ts">
/** 状态点：来源色 / 状态色，含可选 glow（实时态）。 */
defineProps<{
  color?: string;
  glow?: boolean;
  pulse?: boolean;
  size?: number;
}>();
</script>

<template>
  <span
    class="status-dot"
    :class="{ glow, pulse }"
    :style="{
      '--c': color ?? 'var(--ash-deep)',
      width: (size ?? 8) + 'px',
      height: (size ?? 8) + 'px',
    }"
    aria-hidden="true"
  />
</template>

<style scoped>
.status-dot {
  display: inline-block;
  border-radius: 50%;
  background: var(--c);
  flex-shrink: 0;
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--c) 18%, transparent);
}
.status-dot.glow {
  box-shadow: 0 0 8px color-mix(in srgb, var(--c) 60%, transparent);
}
.status-dot.pulse {
  animation: dot-pulse 1.4s ease-in-out infinite;
}
@keyframes dot-pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.4; transform: scale(0.7); }
}
</style>
