<script setup lang="ts">
// 快速开始 · CLI 命令 + 开源链接（方案 §6.8 第 11 节）
// 签名元素：双终端块（SDK 嵌入 / 自托管），冷青 $ 提示符 + 琥珀输出 + ash 注释
import { ArrowUpRight, Terminal, Package } from "lucide-vue-next";
import GithubIcon from "./GithubIcon.vue";
import Reveal from "./Reveal.vue";

const sdkSteps = [
  { prompt: "$", cmd: "pnpm add @rrweb-demo/observer-sdk", note: "或 npm / yarn" },
  { prompt: ">", cmd: 'import { recordOffline } from "@rrweb-demo/observer-sdk"', note: "一行接入" },
  { prompt: ">", cmd: 'recordOffline({ endpoint: "http://localhost:8080/ingest" })', note: "断网照录，恢复回传" },
];

const serverSteps = [
  { prompt: "$", cmd: "observer-server \\", note: "单二进制" },
  { prompt: " ", cmd: "  --bind 0.0.0.0:8080 \\", note: "绑公网或内网" },
  { prompt: " ", cmd: "  --web-dir ./console \\", note: "SPA 静态托管" },
  { prompt: " ", cmd: "  --tenants tenants.json", note: "多租户配额" },
];
</script>

<template>
  <section id="quickstart" class="section qs">
    <div class="section-inner">
      <Reveal>
        <header class="section-head center">
          <p class="eyebrow mono">快速开始 · 两条路</p>
          <h2 class="section-h2">
            <span class="accent-amber">一行接入</span>，<br />
            或<span class="accent-teal">一个二进制</span>自托管。
          </h2>
          <p class="section-sub">
            嵌入任意 web 应用，或起一个自托管 server。全开源，数据在你手里。
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
                嵌入你的应用
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
                自托管 server
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
            href="https://github.com"
            rel="noreferrer"
            target="_blank"
          >
            <GithubIcon :size="18" />
            在 GitHub 上阅读源码
            <ArrowUpRight :size="16" />
          </a>
          <p class="qs-foot mono">
            MIT · Rust 核心 + Vue console + Web SDK · 全开源
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
