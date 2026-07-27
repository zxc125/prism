<script setup lang="ts">
// 粘性站点导航 - 滚出 hero 后浮现（方案 §6.6 滚动揭示配套）
// slim + backdrop-blur，wordmark + section 锚点 + GitHub + CTA
import { ref, onMounted, onUnmounted } from "vue";
import PrismLogo from "./PrismLogo.vue";
import GithubIcon from "./GithubIcon.vue";

const visible = ref(false);
const onScroll = () => {
  visible.value = window.scrollY > window.innerHeight * 0.55;
};
onMounted(() =>
  window.addEventListener("scroll", onScroll, { passive: true }),
);
onUnmounted(() => window.removeEventListener("scroll", onScroll));

const links = [
  { label: "诊断", href: "#diagnosis" },
  { label: "多窗口", href: "#multiwindow" },
  { label: "部署", href: "#deploy" },
  { label: "对比", href: "#compare" },
  { label: "开始", href: "#quickstart" },
];
</script>

<template>
  <header class="sitenav" :class="{ shown: visible }">
    <a class="nav-wordmark" href="#top">
      <PrismLogo :height="24" />
      <span class="nav-name">鉴 / Prism</span>
    </a>
    <nav class="nav-links">
      <a
        v-for="l in links"
        :key="l.href"
        class="nav-link"
        :href="l.href"
      >
        {{ l.label }}
      </a>
    </nav>
    <div class="nav-actions">
      <a
        class="nav-gh"
        href="https://github.com"
        rel="noreferrer"
        target="_blank"
        aria-label="GitHub"
      >
        <GithubIcon :size="17" />
      </a>
      <a class="btn btn-primary nav-cta" href="#quickstart">自托管 -></a>
    </div>
  </header>
</template>

<style scoped>
.sitenav {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 2rem;
  background: rgba(10, 12, 16, 0.72);
  backdrop-filter: saturate(160%) blur(12px);
  -webkit-backdrop-filter: saturate(160%) blur(12px);
  border-bottom: 1px solid var(--color-hair);
  opacity: 0;
  transform: translateY(-100%);
  transition:
    opacity 0.3s ease,
    transform 0.3s ease;
  pointer-events: none;
}
.sitenav.shown {
  opacity: 1;
  transform: none;
  pointer-events: auto;
}
.nav-wordmark {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  text-decoration: none;
}
.nav-name {
  font-size: 0.9375rem;
  font-weight: 600;
  color: var(--color-bone);
}
.nav-links {
  display: flex;
  align-items: center;
  gap: 1.5rem;
}
.nav-link {
  font-size: 0.8125rem;
  color: var(--color-ash);
  text-decoration: none;
  transition: color 0.15s;
}
.nav-link:hover {
  color: var(--color-bone);
}
.nav-actions {
  display: flex;
  align-items: center;
  gap: 0.875rem;
}
.nav-gh {
  display: inline-flex;
  align-items: center;
  color: var(--color-ash);
  transition: color 0.15s;
}
.nav-gh:hover {
  color: var(--color-bone);
}
.nav-cta {
  font-size: 0.8125rem;
  padding: 0.5rem 0.9rem;
}

@media (max-width: 720px) {
  .nav-links {
    display: none;
  }
  .sitenav {
    padding: 0.625rem 1.25rem;
  }
}
</style>
