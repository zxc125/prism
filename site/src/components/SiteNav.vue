<script setup lang="ts">
// 粘性站点导航 - 滚出 hero 后浮现（方案 §6.6 滚动揭示配套）
// slim + backdrop-blur，wordmark + section 锚点 + GitHub + CTA
import { ref, onMounted, onUnmounted } from "vue";
import PrismLogo from "./PrismLogo.vue";
import GithubIcon from "./GithubIcon.vue";
import { useLang } from "../composables/useLang";
import { withBase } from "vitepress";

const { t, currentLang, toggle } = useLang();

const visible = ref(false);
const onScroll = () => {
  visible.value = window.scrollY > window.innerHeight * 0.55;
};
onMounted(() =>
  window.addEventListener("scroll", onScroll, { passive: true }),
);
onUnmounted(() => window.removeEventListener("scroll", onScroll));

const links = [
  { label: "nav.diagnosis", href: "#diagnosis" },
  { label: "nav.multiwindow", href: "#multiwindow" },
  { label: "nav.deploy", href: "#deploy" },
  { label: "nav.compare", href: "#compare" },
  { label: "nav.quickstart", href: "#quickstart" },
  { label: "nav.docs", href: withBase("/docs/quickstart") },
];
const langLabel = () => (currentLang.value === "zh-CN" ? "EN" : "中");
</script>

<template>
  <header class="sitenav" :class="{ shown: visible }">
    <a class="nav-wordmark" href="#top">
      <PrismLogo :height="24" />
      <span class="nav-name">{{ t("nav.wordmark") }}</span>
    </a>
    <nav class="nav-links">
      <a
        v-for="l in links"
        :key="l.href"
        class="nav-link"
        :href="l.href"
        :target="l.href.startsWith('/') ? '_blank' : undefined"
        rel="noopener"
      >
        {{ t(l.label) }}
      </a>
    </nav>
    <div class="nav-actions">
      <button
        class="lang-toggle"
        type="button"
        :aria-label="currentLang === 'zh-CN' ? 'English' : '中文'"
        @click="toggle"
      >
        {{ langLabel() }}
      </button>
      <a
        class="nav-gh"
        href="https://github.com/zxc125/prism"
        rel="noreferrer"
        target="_blank"
        aria-label="GitHub"
      >
        <GithubIcon :size="17" />
      </a>
      <a class="btn btn-primary nav-cta" href="#quickstart">{{ t("nav.cta") }}</a>
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
.lang-toggle {
  font-family: var(--font-mono);
  font-size: 0.6875rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  color: var(--color-ash);
  background: transparent;
  border: 1px solid var(--color-hair);
  border-radius: 4px;
  padding: 0.25rem 0.5rem;
  cursor: pointer;
  line-height: 1;
  transition:
    color 0.15s,
    border-color 0.15s,
    background 0.15s;
}
.lang-toggle:hover {
  color: var(--color-bone);
  border-color: var(--color-ash);
  background: var(--color-slate);
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
