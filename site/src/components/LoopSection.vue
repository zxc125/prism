<script setup lang="ts">
// 闭环 · 录 -> 回放 -> 诊断 -> 标注 -> 导出/分享（方案 §6.8 第 7 节）
// 签名元素：5 段横向流，琥珀 -> 冷青 渐进色编码 + 连接线 + 编号节点
import { Disc, Play, Stethoscope, MessageSquarePlus, Share2 } from "lucide-vue-next";
import Reveal from "./Reveal.vue";

const stages = [
  { n: "01", icon: Disc, label: "录制", desc: "DOM 级 + 诊断信号，多窗口对齐", accent: "amber" },
  { n: "02", icon: Play, label: "回放", desc: "多轨时间轴 + 真实播放头", accent: "amber" },
  { n: "03", icon: Stethoscope, label: "诊断", desc: "交错信号，同一条时间轴", accent: "teal" },
  { n: "04", icon: MessageSquarePlus, label: "标注", desc: "session 级标注，与事件流分离", accent: "teal" },
  { n: "05", icon: Share2, label: "导出 / 分享", desc: "单文件 bundle，零依赖 JSON", accent: "amber" },
];
</script>

<template>
  <section id="loop" class="section">
    <div class="section-inner">
      <Reveal>
        <header class="section-head center">
          <p class="eyebrow mono">闭环 · 一条诊断链路</p>
          <h2 class="section-h2">
            从<span class="accent-amber">录到分享</span>，<br />
            不在五个工具间来回横跳。
          </h2>
          <p class="section-sub">
            一条链路走完诊断全流程：录制采集 DOM + 诊断信号，回放多轨对齐，诊断看懂，标注留痕，导出单文件 bundle 分享。每一步都在同一个工具里。
          </p>
        </header>
      </Reveal>

      <Reveal>
        <div class="loop-flow">
          <template v-for="(s, i) in stages" :key="s.n">
            <article class="loop-node" :class="`accent-${s.accent}`">
              <div class="node-icon">
                <component :is="s.icon" :size="22" />
              </div>
              <span class="node-n mono">{{ s.n }}</span>
              <h3 class="node-label">{{ s.label }}</h3>
              <p class="node-desc">{{ s.desc }}</p>
            </article>
            <div
              v-if="i < stages.length - 1"
              class="loop-arrow"
              aria-hidden="true"
            >
              <span class="arrow-line" />
              <span class="arrow-head" />
            </div>
          </template>
        </div>
      </Reveal>
    </div>
  </section>
</template>

<style scoped>
.loop-flow {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: 0;
}
@media (min-width: 900px) {
  .loop-flow {
    flex-direction: row;
    align-items: stretch;
    gap: 0;
  }
}
.loop-node {
  flex: 1;
  background: var(--color-slate);
  border: 1px solid var(--color-hair);
  border-radius: 10px;
  padding: 1.75rem 1.5rem;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.5rem;
  position: relative;
  margin-bottom: 1rem;
}
@media (min-width: 900px) {
  .loop-node {
    margin-bottom: 0;
  }
}
.loop-node.accent-amber {
  border-top: 2px solid var(--color-amber);
}
.loop-node.accent-teal {
  border-top: 2px solid var(--color-teal);
}
.node-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border-radius: 8px;
  background: var(--color-slate-2);
  border: 1px solid var(--color-hair);
}
.accent-amber .node-icon {
  color: var(--color-amber);
}
.accent-teal .node-icon {
  color: var(--color-teal);
}
.node-n {
  font-size: 0.6875rem;
  color: var(--color-ash-deep);
  letter-spacing: 0.12em;
  margin-top: 0.5rem;
}
.node-label {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--color-bone);
  margin: 0;
}
.node-desc {
  font-size: 0.875rem;
  line-height: 1.55;
  color: var(--color-ash);
  margin: 0;
}

/* arrow: horizontal on desktop, rotated on mobile */
.loop-arrow {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 0.5rem;
  margin-bottom: 1rem;
}
@media (min-width: 900px) {
  .loop-arrow {
    margin-bottom: 0;
    padding: 0 0.25rem;
  }
}
.arrow-line {
  width: 20px;
  height: 1px;
  background: var(--color-hair);
}
@media (max-width: 899px) {
  .loop-arrow {
    transform: rotate(90deg);
    padding: 0.75rem 0;
  }
  .arrow-line {
    width: 24px;
  }
}
.arrow-head {
  width: 0;
  height: 0;
  border-top: 4px solid transparent;
  border-bottom: 4px solid transparent;
  border-left: 6px solid var(--color-ash-deep);
  margin-left: -1px;
}
</style>
