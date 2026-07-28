<script setup lang="ts">
// 本地优先宣言 · 数据主权（方案 §6.8 第 3 节）
// 签名元素：棱镜分光 beam 分隔线（白光 -> 棱镜 -> 琥珀/冷青双光谱）+ 编号信条网格
import { computed } from "vue";
import Reveal from "./Reveal.vue";
import { useLang } from "../composables/useLang";

const { t } = useLang();

const beliefs = computed(() => [
  { k: "01", t: t("manifesto.b1.t"), d: t("manifesto.b1.d") },
  { k: "02", t: t("manifesto.b2.t"), d: t("manifesto.b2.d") },
  { k: "03", t: t("manifesto.b3.t"), d: t("manifesto.b3.d") },
  { k: "04", t: t("manifesto.b4.t"), d: t("manifesto.b4.d") },
]);
</script>

<template>
  <section id="manifesto" class="section manifesto">
    <div class="section-inner">
      <Reveal>
        <p class="eyebrow mono">{{ t("manifesto.eyebrow") }}</p>
        <h2 class="mani-h2">
          {{ t("manifesto.h2_pre") }}<br />
          {{ t("manifesto.h2_mid") }}<span class="accent-amber">{{ t("manifesto.h2_accent1") }}</span>{{ t("manifesto.h2_between") }}<span class="strike">{{ t("manifesto.h2_accent2") }}</span>{{ t("manifesto.h2_suf") }}
        </h2>
        <!-- 棱镜分光 beam：白光 -> 棱镜 -> 琥珀/冷青双光谱 -->
        <div class="prism-beam" aria-hidden="true">
          <span class="beam beam-white" />
          <span class="prism" />
          <span class="beam beam-amber" />
          <span class="beam beam-teal" />
        </div>
      </Reveal>

      <Reveal>
        <div class="belief-grid">
          <article
            v-for="(b, i) in beliefs"
            :key="b.k"
            class="belief"
            :class="{ teal: i % 2 === 1 }"
          >
            <span class="belief-k mono">{{ b.k }}</span>
            <h3 class="belief-t">{{ b.t }}</h3>
            <p class="belief-d">{{ b.d }}</p>
          </article>
        </div>
      </Reveal>
    </div>
  </section>
</template>

<style scoped>
.manifesto {
  background:
    radial-gradient(
      ellipse 50% 35% at 30% 20%,
      rgba(240, 168, 61, 0.03),
      transparent
    ),
    var(--color-ink);
}
.mani-h2 {
  font-size: clamp(2rem, 5vw, 3.25rem);
  line-height: 1.1;
  letter-spacing: -0.025em;
  font-weight: 700;
  color: var(--color-bone);
  margin: 0 0 2.5rem;
  max-width: 40rem;
}
.strike {
  color: var(--color-ash-deep);
  text-decoration: line-through;
  text-decoration-color: var(--color-oxblood);
  text-decoration-thickness: 2px;
}

/* prism beam 分隔线 */
.prism-beam {
  display: flex;
  align-items: center;
  gap: 0;
  margin: 0 0 4rem;
  max-width: 40rem;
}
.beam {
  display: block;
  height: 3px;
  border-radius: 2px;
}
.beam-white {
  flex: 1;
  background: var(--color-bone);
  opacity: 0.55;
}
.prism {
  width: 0;
  height: 0;
  border-top: 10px solid transparent;
  border-bottom: 10px solid transparent;
  border-left: 14px solid rgba(230, 234, 240, 0.4);
  flex-shrink: 0;
  filter: drop-shadow(0 0 3px rgba(230, 234, 240, 0.3));
}
.beam-amber {
  flex: 1.2;
  background: var(--color-amber);
  box-shadow: 0 0 8px rgba(240, 168, 61, 0.5);
}
.beam-teal {
  flex: 1.2;
  background: var(--color-teal);
  box-shadow: 0 0 8px rgba(77, 208, 200, 0.5);
}

.belief-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 1.5rem;
}
@media (min-width: 768px) {
  .belief-grid {
    grid-template-columns: 1fr 1fr;
    gap: 2.5rem 3rem;
  }
}
.belief {
  border-left: 1px solid var(--color-hair);
  padding-left: 1.5rem;
  position: relative;
}
.belief::before {
  content: "";
  position: absolute;
  left: -1px;
  top: 0;
  width: 1px;
  height: 24px;
  background: var(--color-amber);
}
.belief.teal::before {
  background: var(--color-teal);
}
.belief-k {
  font-size: 0.75rem;
  color: var(--color-ash-deep);
  letter-spacing: 0.1em;
}
.belief-t {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--color-bone);
  margin: 0.5rem 0 0.625rem;
}
.belief-d {
  font-size: 0.9375rem;
  line-height: 1.65;
  color: var(--color-ash);
  margin: 0;
}
</style>
