<script setup lang="ts">
// 核心能力 · 六个记忆点（方案 §6.8 第 4 节）
// 签名元素：6 卡网格，琥珀/冷青交替色编码图标 + hover 上浮，hairline 边
import { computed } from "vue";
import {
  HardDrive,
  AppWindow,
  Waves,
  Stethoscope,
  Package,
  FileJson,
} from "lucide-vue-next";
import Reveal from "./Reveal.vue";
import { useLang } from "../composables/useLang";

const { t } = useLang();

const caps = computed(() => [
  {
    icon: HardDrive,
    accent: "amber",
    title: t("capabilities.c1.title"),
    desc: t("capabilities.c1.desc"),
  },
  {
    icon: AppWindow,
    accent: "teal",
    title: t("capabilities.c2.title"),
    desc: t("capabilities.c2.desc"),
  },
  {
    icon: Waves,
    accent: "amber",
    title: t("capabilities.c3.title"),
    desc: t("capabilities.c3.desc"),
  },
  {
    icon: Stethoscope,
    accent: "teal",
    title: t("capabilities.c4.title"),
    desc: t("capabilities.c4.desc"),
  },
  {
    icon: Package,
    accent: "amber",
    title: t("capabilities.c5.title"),
    desc: t("capabilities.c5.desc"),
  },
  {
    icon: FileJson,
    accent: "teal",
    title: t("capabilities.c6.title"),
    desc: t("capabilities.c6.desc"),
  },
]);
</script>

<template>
  <section id="capabilities" class="section">
    <div class="section-inner">
      <Reveal>
        <header class="section-head">
          <p class="eyebrow mono">{{ t("capabilities.eyebrow") }}</p>
          <h2 class="section-h2">
            {{ t("capabilities.h2_pre") }}<span class="accent-amber">{{ t("capabilities.h2_accent") }}</span>{{ t("capabilities.h2_mid") }}<br />
            {{ t("capabilities.h2_suf") }}
          </h2>
          <p class="section-sub">
            {{ t("capabilities.sub") }}
          </p>
        </header>
      </Reveal>

      <Reveal>
        <div class="cap-grid">
          <article
            v-for="(c, i) in caps"
            :key="i"
            class="cap-card"
            :class="`accent-${c.accent}`"
          >
            <div class="cap-icon">
              <component :is="c.icon" :size="20" />
            </div>
            <h3 class="cap-title">{{ c.title }}</h3>
            <p class="cap-desc">{{ c.desc }}</p>
          </article>
        </div>
      </Reveal>
    </div>
  </section>
</template>

<style scoped>
.cap-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 1.125rem;
}
@media (min-width: 640px) {
  .cap-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
@media (min-width: 1024px) {
  .cap-grid {
    grid-template-columns: repeat(3, 1fr);
  }
}
.cap-card {
  background: var(--color-slate);
  border: 1px solid var(--color-hair);
  border-radius: 10px;
  padding: 1.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  transition:
    border-color 0.15s,
    transform 0.15s;
}
.cap-card:hover {
  transform: translateY(-2px);
}
.cap-card.accent-amber:hover {
  border-color: rgba(240, 168, 61, 0.35);
}
.cap-card.accent-teal:hover {
  border-color: rgba(77, 208, 200, 0.35);
}
.cap-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 8px;
  border: 1px solid var(--color-hair);
  background: var(--color-slate-2);
}
.accent-amber .cap-icon {
  color: var(--color-amber);
  border-color: rgba(240, 168, 61, 0.25);
}
.accent-teal .cap-icon {
  color: var(--color-teal);
  border-color: rgba(77, 208, 200, 0.25);
}
.cap-title {
  font-size: 1.0625rem;
  font-weight: 600;
  color: var(--color-bone);
  margin: 0.25rem 0 0;
  line-height: 1.4;
}
.cap-desc {
  font-size: 0.9375rem;
  line-height: 1.6;
  color: var(--color-ash);
  margin: 0;
}
</style>
