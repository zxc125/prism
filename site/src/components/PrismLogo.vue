<script setup lang="ts">
// 棱镜 logo - 白光入菱形（◆），于中心折射分出琥珀(DOM)/冷青(信号)双光谱
// 定稿 R1 聚心：三束光于菱形中心交汇分光，节点标记折射点
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
    viewBox="0 0 80 52"
    :height="height"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    class="prism-logo"
    :class="{ glow }"
  >
    <!-- 菱镜玻璃面 -->
    <polygon
      points="20,26 40,8 60,26 40,44"
      fill="rgba(230,234,240,0.045)"
      stroke="rgba(230,234,240,0.34)"
      stroke-width="1.2"
      stroke-linejoin="round"
      class="prism-glass"
    />
    <!-- 白光入射 -> 中心 -->
    <line
      x1="2" y1="26" x2="40" y2="26"
      stroke="var(--color-bone)" stroke-width="2.5" stroke-linecap="round"
      class="beam beam-white"
    />
    <!-- 琥珀出射（中心 -> 右上，穿右上棱面） -->
    <line
      x1="40" y1="26" x2="78" y2="8"
      stroke="var(--color-amber)" stroke-width="2.5" stroke-linecap="round"
      class="beam beam-amber"
    />
    <!-- 冷青出射（中心 -> 右下，穿右下棱面） -->
    <line
      x1="40" y1="26" x2="78" y2="44"
      stroke="var(--color-teal)" stroke-width="2.5" stroke-linecap="round"
      class="beam beam-teal"
    />
    <!-- 折射节点 -->
    <circle cx="40" cy="26" r="2.6" fill="var(--color-bone)" class="node" />
  </svg>
</template>

<style scoped>
.beam-white { filter: drop-shadow(0 0 1.5px rgba(230, 234, 240, 0.35)); }
.node { filter: drop-shadow(0 0 2px rgba(230, 234, 240, 0.5)); }
.glow .prism-glass { stroke: rgba(230, 234, 240, 0.5); }
.glow .beam-white { filter: drop-shadow(0 0 4px rgba(230, 234, 240, 0.55)); }
.glow .beam-amber { filter: drop-shadow(0 0 5px rgba(240, 168, 61, 0.6)); }
.glow .beam-teal { filter: drop-shadow(0 0 5px rgba(77, 208, 200, 0.6)); }
.glow .node { filter: drop-shadow(0 0 6px rgba(230, 234, 240, 0.85)); }
</style>
