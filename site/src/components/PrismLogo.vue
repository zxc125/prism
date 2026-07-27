<script setup lang="ts">
// 棱镜 logo - 白光入棱镜分出琥珀(DOM)/冷青(信号)双光谱
// 静态可移植（favicon/og），hero 内通过 glow 启用发光
withDefaults(
  defineProps<{
    height?: number
    glow?: boolean
  }>(),
  { height: 40, glow: false },
);
</script>

<template>
  <svg
    viewBox="0 0 200 64"
    :height="height"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    class="prism-logo"
    :class="{ glow }"
  >
    <line x1="2" y1="32" x2="78" y2="32" stroke="var(--color-bone)" stroke-width="3" stroke-linecap="round" class="beam beam-white" />
    <polygon
      points="78,12 78,52 118,32"
      fill="rgba(230,234,240,0.05)"
      stroke="rgba(230,234,240,0.35)"
      stroke-width="1.5"
      stroke-linejoin="round"
    />
    <line x1="118" y1="32" x2="198" y2="14" stroke="var(--color-amber)" stroke-width="3" stroke-linecap="round" class="beam beam-amber" />
    <line x1="118" y1="32" x2="198" y2="50" stroke="var(--color-teal)" stroke-width="3" stroke-linecap="round" class="beam beam-teal" />
  </svg>
</template>

<style scoped>
.beam-white { filter: drop-shadow(0 0 2px rgba(230, 234, 240, 0.3)); }
.glow .beam-white { filter: drop-shadow(0 0 4px rgba(230, 234, 240, 0.5)); }
.glow .beam-amber { filter: drop-shadow(0 0 5px rgba(240, 168, 61, 0.55)); }
.glow .beam-teal { filter: drop-shadow(0 0 5px rgba(77, 208, 200, 0.55)); }
</style>
