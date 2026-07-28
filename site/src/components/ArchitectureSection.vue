<script setup lang="ts">
// 架构透明 · rrweb-based / Rust 核心 / 开放 bundle（方案 §6.8 第 10 节）
// 签名元素：分层架构栈（bottom-up）+ bundle JSON 样例（mono，色编码键）
import { computed } from "vue";
import { Boxes, Cpu, FileJson, MonitorPlay } from "lucide-vue-next";
import Reveal from "./Reveal.vue";
import { useLang } from "../composables/useLang";

const { t } = useLang();

const layers = computed(() => [
  {
    icon: MonitorPlay,
    name: t("architecture.l1.name"),
    sub: t("architecture.l1.sub"),
    color: "amber",
  },
  {
    icon: FileJson,
    name: t("architecture.l2.name"),
    sub: t("architecture.l2.sub"),
    color: "teal",
  },
  {
    icon: Cpu,
    name: t("architecture.l3.name"),
    sub: t("architecture.l3.sub"),
    color: "amber",
  },
  {
    icon: Boxes,
    name: t("architecture.l4.name"),
    sub: t("architecture.l4.sub"),
    color: "teal",
  },
]);

const bundleLines = computed(() => [
  { tok: "key", v: '"format"', s: '"prism-session"' },
  { tok: "key", v: '"version"', s: "1" },
  { tok: "key", v: '"session"', s: t("architecture.bundle.session") },
  { tok: "key", v: '"windows"', s: t("architecture.bundle.windows") },
  { tok: "key", v: '"segments"', s: t("architecture.bundle.segments") },
  { tok: "key", v: '"annotations"', s: t("architecture.bundle.annotations") },
]);
</script>

<template>
  <section id="architecture" class="section arch">
    <div class="section-inner">
      <Reveal>
        <header class="section-head">
          <p class="eyebrow mono">{{ t("architecture.eyebrow") }}</p>
          <h2 class="section-h2">
            {{ t("architecture.h2_pre") }}<span class="accent-amber">{{ t("architecture.h2_accent1") }}</span>{{ t("architecture.h2_mid") }}<br />
            {{ t("architecture.h2_suf1") }}<span class="accent-teal">{{ t("architecture.h2_accent2") }}</span>{{ t("architecture.h2_suf2") }}<span class="accent-amber">{{ t("architecture.h2_accent3") }}</span>{{ t("architecture.h2_suf3") }}
          </h2>
          <p class="section-sub">
            {{ t("architecture.sub") }}
          </p>
        </header>
      </Reveal>

      <Reveal>
        <div class="arch-grid">
          <!-- 分层栈 -->
          <div class="arch-stack">
            <div
              v-for="(l, i) in layers"
              :key="i"
              class="arch-layer"
              :class="`accent-${l.color}`"
            >
              <div class="layer-icon">
                <component :is="l.icon" :size="18" />
              </div>
              <div class="layer-body">
                <span class="layer-name mono">{{ l.name }}</span>
                <span class="layer-sub">{{ l.sub }}</span>
              </div>
            </div>
          </div>

          <!-- bundle JSON 样例 -->
          <div class="arch-code">
            <div class="code-head">
              <span class="code-dot dot-r" />
              <span class="code-dot dot-y" />
              <span class="code-dot dot-g" />
              <span class="code-title mono">prism-session.bundle.json</span>
            </div>
            <pre class="code-body mono"><code><span class="code-line"><span class="p">{</span></span><span
              v-for="(l, i) in bundleLines"
              :key="i"
              class="code-line indented"
            ><span class="k">{{ l.v }}</span><span class="p">:</span> <span class="s">{{ l.s }}</span><span
                v-if="i < bundleLines.length - 1"
                class="p"
              >,</span></span><span class="code-line"><span class="p">}</span></span></code></pre>
          </div>
        </div>
      </Reveal>
    </div>
  </section>
</template>

<style scoped>
.arch {
  background: var(--color-ink);
}
.arch-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 1.5rem;
}
@media (min-width: 900px) {
  .arch-grid {
    grid-template-columns: 1fr 1fr;
    gap: 2rem;
    align-items: start;
  }
}
.arch-stack {
  display: flex;
  flex-direction: column-reverse;
  gap: 0.75rem;
}
.arch-layer {
  display: flex;
  align-items: center;
  gap: 1rem;
  background: var(--color-slate);
  border: 1px solid var(--color-hair);
  border-radius: 10px;
  padding: 1.125rem 1.25rem;
  position: relative;
}
.arch-layer.accent-amber {
  border-left: 2px solid var(--color-amber);
}
.arch-layer.accent-teal {
  border-left: 2px solid var(--color-teal);
}
.layer-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 6px;
  background: var(--color-slate-2);
  flex-shrink: 0;
}
.accent-amber .layer-icon {
  color: var(--color-amber);
}
.accent-teal .layer-icon {
  color: var(--color-teal);
}
.layer-body {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  min-width: 0;
}
.layer-name {
  font-size: 0.8125rem;
  color: var(--color-bone);
  font-weight: 600;
}
.layer-sub {
  font-size: 0.75rem;
  color: var(--color-ash);
  line-height: 1.4;
}

/* code block */
.arch-code {
  background: var(--color-ink-2);
  border: 1px solid var(--color-hair);
  border-radius: 10px;
  overflow: hidden;
}
.code-head {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid var(--color-hair);
  background: var(--color-slate);
}
.code-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
}
.dot-r {
  background: var(--color-oxblood);
  opacity: 0.7;
}
.dot-y {
  background: var(--color-amber);
  opacity: 0.7;
}
.dot-g {
  background: var(--color-teal);
  opacity: 0.7;
}
.code-title {
  margin-left: 0.5rem;
  font-size: 0.6875rem;
  color: var(--color-ash-deep);
}
.code-body {
  margin: 0;
  padding: 1.25rem;
  font-size: 0.8125rem;
  line-height: 1.7;
  color: var(--color-bone-dim);
  overflow-x: auto;
}
.code-body code {
  font-family: inherit;
}
.code-line {
  display: block;
  white-space: pre;
}
.code-line.indented {
  padding-left: 1.2em;
}
.k {
  color: var(--color-teal);
}
.s {
  color: var(--color-amber);
}
.p {
  color: var(--color-ash);
}
</style>
