<script setup lang="ts">
// 快速开始 · CLI 命令 + 开源链接（方案 §6.8 第 11 节）
// 签名元素：双终端块（SDK 嵌入 / 自托管），冷青 $ 提示符 + 琥珀输出 + ash 注释
import { computed } from "vue";
import { ArrowUpRight, Terminal, Package } from "lucide-vue-next";
import GithubIcon from "./GithubIcon.vue";
import Reveal from "./Reveal.vue";
import { useLang } from "../composables/useLang";

const { t } = useLang();

const sdkSteps = computed(() => [
  { prompt: "$", cmd: "pnpm add @prism-obs/observer-sdk", note: t("quickstart.sdk.s1") },
  { prompt: ">", cmd: 'import { recordOffline } from "@prism-obs/observer-sdk"', note: t("quickstart.sdk.s2") },
  { prompt: ">", cmd: 'recordOffline({ endpoint: "http://localhost:8080/ingest" })', note: t("quickstart.sdk.s3") },
]);

const serverSteps = computed(() => [
  { prompt: "$", cmd: "observer-server \\", note: t("quickstart.server.s1") },
  { prompt: " ", cmd: "  --bind 0.0.0.0:8080 \\", note: t("quickstart.server.s2") },
  { prompt: " ", cmd: "  --web-dir ./console \\", note: t("quickstart.server.s3") },
  { prompt: " ", cmd: "  --tenants tenants.json", note: t("quickstart.server.s4") },
]);
</script>

<template>
  <section id="quickstart" class="section qs">
    <div class="section-inner">
      <Reveal>
        <header class="section-head center">
          <p class="eyebrow mono">{{ t("quickstart.eyebrow") }}</p>
          <h2 class="section-h2">
            <span class="accent-amber">{{ t("quickstart.h2_accent1") }}</span>{{ t("quickstart.h2_mid") }}<br />
            {{ t("quickstart.h2_pre") }}<span class="accent-teal">{{ t("quickstart.h2_accent2") }}</span>{{ t("quickstart.h2_suf") }}
          </h2>
          <p class="section-sub">
            {{ t("quickstart.sub") }}
          </p>
        </header>
      </Reveal>

      <Reveal>
        <div class="qs-grid">
          <!-- SDK 嵌入 -->
          <div class="term-card">
            <div class="term-head">
              <span class="term-label mono">
                <Package :size="14" />
                {{ t("quickstart.sdk.label") }}
              </span>
              <span class="chip">SDK</span>
            </div>
            <div class="term-body mono">
              <div
                v-for="(s, i) in sdkSteps"
                :key="i"
                class="term-line"
              >
                <span class="term-prompt" :class="s.prompt === '$' ? 'dollar' : 'arrow'">{{ s.prompt }}</span>
                <span class="term-cmd">{{ s.cmd }}</span>
                <span v-if="s.note" class="term-note"># {{ s.note }}</span>
              </div>
            </div>
          </div>

          <!-- 自托管 -->
          <div class="term-card">
            <div class="term-head">
              <span class="term-label mono">
                <Terminal :size="14" />
                {{ t("quickstart.server.label") }}
              </span>
              <span class="chip">SELF-HOST</span>
            </div>
            <div class="term-body mono">
              <div
                v-for="(s, i) in serverSteps"
                :key="i"
                class="term-line"
              >
                <span class="term-prompt" :class="s.prompt === '$' ? 'dollar' : 'arrow'">{{ s.prompt }}</span>
                <span class="term-cmd">{{ s.cmd }}</span>
                <span v-if="s.note" class="term-note"># {{ s.note }}</span>
              </div>
            </div>
          </div>
        </div>
      </Reveal>

      <Reveal>
        <div class="qs-cta">
          <a
            class="btn btn-primary lg"
            href="https://github.com/zxc125/prism"
            rel="noreferrer"
            target="_blank"
          >
            <GithubIcon :size="18" />
            {{ t("quickstart.cta") }}
            <ArrowUpRight :size="16" />
          </a>
          <p class="qs-foot mono">
            {{ t("quickstart.foot") }}
          </p>
        </div>
      </Reveal>
    </div>
  </section>
</template>

<style scoped>
.qs {
  background:
    radial-gradient(
      ellipse 50% 35% at 50% 30%,
      rgba(240, 168, 61, 0.025),
      transparent
    ),
    var(--color-ink);
}
.qs-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 1.25rem;
}
@media (min-width: 900px) {
  .qs-grid {
    grid-template-columns: 1fr 1fr;
  }
}
.term-card {
  background: var(--color-ink-2);
  border: 1px solid var(--color-hair);
  border-radius: 10px;
  overflow: hidden;
}
.term-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid var(--color-hair);
  background: var(--color-slate);
}
.term-label {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.8125rem;
  color: var(--color-bone);
}
.term-body {
  padding: 1.125rem 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  overflow-x: auto;
}
.term-line {
  display: flex;
  align-items: baseline;
  gap: 0.6rem;
  white-space: nowrap;
  font-size: 0.8125rem;
  line-height: 1.5;
}
.term-prompt {
  flex-shrink: 0;
  width: 12px;
}
.term-prompt.dollar {
  color: var(--color-teal);
}
.term-prompt.arrow {
  color: var(--color-amber);
}
.term-cmd {
  color: var(--color-bone);
}
.term-note {
  color: var(--color-ash-deep);
  font-size: 0.75rem;
  margin-left: 0.5rem;
}

.qs-cta {
  margin-top: 3rem;
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1rem;
}
.qs-foot {
  font-size: 0.75rem;
  color: var(--color-ash-deep);
  letter-spacing: 0.04em;
}
</style>
