<script setup lang="ts">
// 滚动揭示：section 进入视口时淡入 + 微上移（方案 §6.6）
// 尊重 prefers-reduced-motion：直接显示
import { ref, onMounted, onUnmounted } from "vue";

const el = ref<HTMLElement | null>(null);
const shown = ref(false);
let io: IntersectionObserver | null = null;

onMounted(() => {
  if (!el.value) return;
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    shown.value = true;
    return;
  }
  io = new IntersectionObserver(
    (entries) => {
      entries.forEach((e) => {
        if (e.isIntersecting) {
          shown.value = true;
          io?.disconnect();
        }
      });
    },
    { threshold: 0.12, rootMargin: "0px 0px -8% 0px" },
  );
  io.observe(el.value);
});

onUnmounted(() => io?.disconnect());
</script>

<template>
  <div ref="el" class="reveal" :class="{ shown }">
    <slot />
  </div>
</template>

<style scoped>
.reveal {
  opacity: 0;
  transform: translateY(18px);
  transition:
    opacity 0.6s ease,
    transform 0.6s ease;
}
.reveal.shown {
  opacity: 1;
  transform: none;
}
@media (prefers-reduced-motion: reduce) {
  .reveal {
    opacity: 1;
    transform: none;
    transition: none;
  }
}
</style>
