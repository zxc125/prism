<script setup lang="ts">
// 核心能力 · 六个记忆点（方案 §6.8 第 4 节）
// 签名元素：6 卡网格，琥珀/冷青交替色编码图标 + hover 上浮，hairline 边
import {
  HardDrive,
  AppWindow,
  Waves,
  Stethoscope,
  Package,
  FileJson,
} from "lucide-vue-next";
import Reveal from "./Reveal.vue";

const caps = [
  {
    icon: HardDrive,
    accent: "amber",
    title: "本地优先，数据不出本地",
    desc: "默认零云依赖。用户数据不经任何第三方服务器，隐私天然合规，无厂商锁定。",
  },
  {
    icon: AppWindow,
    accent: "teal",
    title: "多窗口对齐录制，独家",
    desc: "Tauri 多窗口共享墙上时钟，回放多轨同步。桌面应用调试的核心论点，别家做不到。",
  },
  {
    icon: Waves,
    accent: "amber",
    title: "交错事件模型",
    desc: "error / console / network 作为 type:6 交错进 DOM 事件流，共享同一条时间轴，不跨流对齐。",
  },
  {
    icon: Stethoscope,
    accent: "teal",
    title: "诊断导向，不是监控",
    desc: "不做告警、不做 RUM 指标。专注复现 -> 看懂 -> 标注 -> 分享的诊断闭环。",
  },
  {
    icon: Package,
    accent: "amber",
    title: "单二进制自托管",
    desc: "observer-server 一个文件，API + 前端一起托管。内网、离线、合规审计都能用。",
  },
  {
    icon: FileJson,
    accent: "teal",
    title: "开放 bundle 契约",
    desc: "prism-session 明文 JSON 规范，本地 / server / 云端三路共用。随时带走。",
  },
];
</script>

<template>
  <section id="capabilities" class="section">
    <div class="section-inner">
      <Reveal>
        <header class="section-head">
          <p class="eyebrow mono">核心能力 · 六个记忆点</p>
          <h2 class="section-h2">
            每一条都<span class="accent-amber">站得住</span>，<br />
            不是 side feature 凑数。
          </h2>
          <p class="section-sub">
            从采集到部署，六个可一句话讲清的能力，每个都对应产品里一个真实机制，而非营销话术。
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
