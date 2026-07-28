<script setup lang="ts">
// 部署形态 · 桌面 App / 单二进制 / 浏览器（方案 §6.8 第 8 节）
// 签名元素：三形态并置卡片，各带迷你拓扑 + 来源色点 + mono meta
import { computed } from "vue";
import { Monitor, Terminal, Globe } from "lucide-vue-next";
import Reveal from "./Reveal.vue";
import { useLang } from "../composables/useLang";

const { t } = useLang();

const forms = computed(() => [
  {
    icon: Monitor,
    accent: "amber",
    dot: "amber",
    tag: t("deploy.f1.tag"),
    title: t("deploy.f1.title"),
    desc: t("deploy.f1.desc"),
    meta: ["Tauri 2", t("deploy.f1.meta.0"), t("deploy.f1.meta.1")],
  },
  {
    icon: Terminal,
    accent: "teal",
    dot: "teal",
    tag: t("deploy.f2.tag"),
    title: "observer-server",
    desc: t("deploy.f2.desc"),
    meta: [t("deploy.f2.meta.0"), t("deploy.f2.meta.1"), t("deploy.f2.meta.2")],
  },
  {
    icon: Globe,
    accent: "amber",
    dot: "amber",
    tag: t("deploy.f3.tag"),
    title: t("deploy.f3.title"),
    desc: t("deploy.f3.desc"),
    meta: [t("deploy.f3.meta.0"), "HttpBackend", "LoginGate"],
  },
]);
</script>

<template>
  <section id="deploy" class="section deploy">
    <div class="section-inner">
      <Reveal>
        <header class="section-head">
          <p class="eyebrow mono">{{ t("deploy.eyebrow") }}</p>
          <h2 class="section-h2">
            {{ t("deploy.h2_pre") }}<br />
            <span class="accent-teal">{{ t("deploy.h2_accent") }}</span>{{ t("deploy.h2_suf") }}
          </h2>
          <p class="section-sub">
            {{ t("deploy.sub") }}
          </p>
        </header>
      </Reveal>

      <Reveal>
        <div class="form-grid">
          <article
            v-for="(f, i) in forms"
            :key="i"
            class="form-card"
            :class="`accent-${f.accent}`"
          >
            <div class="form-top">
              <div class="form-icon">
                <component :is="f.icon" :size="22" />
              </div>
              <span class="chip form-tag">
                <span class="src-dot" :class="`dot-${f.dot}`" />
                {{ f.tag }}
              </span>
            </div>
            <h3 class="form-title">{{ f.title }}</h3>
            <p class="form-desc">{{ f.desc }}</p>
            <div class="form-meta">
              <span
                v-for="m in f.meta"
                :key="m"
                class="meta-item mono"
              >
                {{ m }}
              </span>
            </div>
          </article>
        </div>
      </Reveal>
    </div>
  </section>
</template>

<style scoped>
.deploy {
  background:
    radial-gradient(
      ellipse 50% 35% at 50% 20%,
      rgba(77, 208, 200, 0.025),
      transparent
    ),
    var(--color-ink);
}
.form-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 1.25rem;
}
@media (min-width: 768px) {
  .form-grid {
    grid-template-columns: repeat(3, 1fr);
  }
}
.form-card {
  background: var(--color-slate);
  border: 1px solid var(--color-hair);
  border-radius: 10px;
  padding: 1.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.875rem;
  transition:
    border-color 0.15s,
    transform 0.15s;
}
.form-card:hover {
  transform: translateY(-2px);
}
.form-card.accent-amber:hover {
  border-color: rgba(240, 168, 61, 0.35);
}
.form-card.accent-teal:hover {
  border-color: rgba(77, 208, 200, 0.35);
}
.form-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.form-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border-radius: 8px;
  background: var(--color-slate-2);
  border: 1px solid var(--color-hair);
}
.accent-amber .form-icon {
  color: var(--color-amber);
  border-color: rgba(240, 168, 61, 0.25);
}
.accent-teal .form-icon {
  color: var(--color-teal);
  border-color: rgba(77, 208, 200, 0.25);
}
.src-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
}
.dot-amber {
  background: var(--color-amber);
  box-shadow: 0 0 5px rgba(240, 168, 61, 0.5);
}
.dot-teal {
  background: var(--color-teal);
  box-shadow: 0 0 5px rgba(77, 208, 200, 0.5);
}
.form-title {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--color-bone);
  margin: 0.25rem 0 0;
  line-height: 1.4;
}
.form-desc {
  font-size: 0.9375rem;
  line-height: 1.6;
  color: var(--color-ash);
  margin: 0;
  flex: 1;
}
.form-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  padding-top: 0.875rem;
  border-top: 1px solid var(--color-hair);
}
.meta-item {
  font-size: 0.6875rem;
  color: var(--color-ash-deep);
  letter-spacing: 0.04em;
}
</style>
