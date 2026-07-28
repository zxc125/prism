<script setup lang="ts">
// 架构透明 · rrweb-based / Rust 核心 / 开放 bundle（方案 §6.8 第 10 节）
// 签名元素：分层架构栈（bottom-up）+ bundle JSON 样例（mono，色编码键）
import { Boxes, Cpu, FileJson, MonitorPlay } from "lucide-vue-next";
import Reveal from "./Reveal.vue";

const layers = [
  {
    icon: MonitorPlay,
    name: "Console / Player UI",
    sub: "Vue 3 · 多轨时间轴 · 诊断信号流 · Backend 抽象",
    color: "amber",
  },
  {
    icon: FileJson,
    name: "bundle 契约 · prism-session",
    sub: "明文 JSON · 三路共用 · 跨进程/跨机迁移唯一契约",
    color: "teal",
  },
  {
    icon: Cpu,
    name: "Rust 核心 · observer-storage / observer-server",
    sub: "落盘 · HTTP API · 多租户 · gzip · 服务端 redact · 限流",
    color: "amber",
  },
  {
    icon: Boxes,
    name: "rrweb 2 · 录制基座",
    sub: "DOM 快照 + 增量 · type:6 plugin 交错诊断信号",
    color: "teal",
  },
];

const bundleLines = [
  { tok: "key", v: '"format"', s: '"prism-session"' },
  { tok: "key", v: '"version"', s: "1" },
  { tok: "key", v: '"session"', s: '{ "id": "1721908482", "startedAt": "…" }' },
  { tok: "key", v: '"windows"', s: '[ /* shown/hidden/focus 生命周期 */ ]' },
  { tok: "key", v: '"segments"', s: '[ /* rrweb 事件流（含 type:6 交错） */ ]' },
  { tok: "key", v: '"annotations"', s: '[ /* 用户标注，session 级 */ ]' },
];
</script>

<template>
  <section id="architecture" class="section arch">
    <div class="section-inner">
      <Reveal>
        <header class="section-head">
          <p class="eyebrow mono">架构透明 · 没有黑盒</p>
          <h2 class="section-h2">
            建在<span class="accent-amber">rrweb</span>之上，<br />
            核心是<span class="accent-teal">Rust</span>，契约是<span class="accent-amber">明文 JSON</span>。
          </h2>
          <p class="section-sub">
            不靠魔法。录制基座是开源 rrweb 2，存储与 server 是独立 Rust crate，跨进程迁移走明文 JSON 契约。每一层都可读、可审计、可替换。
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
