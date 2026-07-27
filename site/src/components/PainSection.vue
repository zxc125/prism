<script setup lang="ts">
// 痛点 · 云 RUM 的三重代价（方案 §6.8 第 2 节）
// 签名元素：oxblood 损伤色 + 顶部 hairline 红光，与全站琥珀/冷青暖冷光谱对照
import { ShieldOff, CircleDollarSign, LockKeyhole } from "lucide-vue-next";
import Reveal from "./Reveal.vue";

const costs = [
  {
    icon: ShieldOff,
    tag: "隐私代价",
    title: "用户会话上传到第三方云",
    desc: "每一次点击、输入、网络请求都被传到别人服务器。金融、医疗、政企场景天然不可接受。",
  },
  {
    icon: CircleDollarSign,
    tag: "成本代价",
    title: "按 session 收费，越用越贵",
    desc: "per-session / per-seat 计费。流量上去不是扩容，是账单爆炸。导出自己的数据还要找销售。",
  },
  {
    icon: LockKeyhole,
    tag: "锁仓代价",
    title: "数据不在你手里",
    desc: "会话存厂商托管，导出格式私有，迁移成本巨大。涨价、停服、改条款时，你没有退路。",
  },
];
</script>

<template>
  <section id="pain" class="section">
    <div class="section-inner">
      <Reveal>
        <header class="section-head">
          <p class="eyebrow mono">痛点 · 云 RUM 的三重代价</p>
          <h2 class="section-h2">
            把会话传到云端，<br />
            你付的不是钱，是<span class="accent-oxblood">控制权</span>。
          </h2>
          <p class="section-sub">
            主流会话回放工具默认把用户数据上传第三方云，按 session 收费，格式私有。隐私、成本、锁仓——三重代价，你全都要付。
          </p>
        </header>
      </Reveal>

      <Reveal>
        <div class="cost-grid">
          <article
            v-for="(c, i) in costs"
            :key="i"
            class="cost-card"
          >
            <div class="cost-icon">
              <component :is="c.icon" :size="22" />
            </div>
            <span class="chip cost-tag">{{ c.tag }}</span>
            <h3 class="cost-title">{{ c.title }}</h3>
            <p class="cost-desc">{{ c.desc }}</p>
          </article>
        </div>
      </Reveal>
    </div>
  </section>
</template>

<style scoped>
.cost-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 1.25rem;
}
@media (min-width: 768px) {
  .cost-grid {
    grid-template-columns: repeat(3, 1fr);
  }
}
.cost-card {
  background: var(--color-slate);
  border: 1px solid var(--color-hair);
  border-radius: 10px;
  padding: 2rem 1.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.875rem;
  position: relative;
  transition:
    border-color 0.15s,
    transform 0.15s;
}
.cost-card:hover {
  border-color: rgba(229, 72, 77, 0.35);
  transform: translateY(-2px);
}
.cost-card::before {
  content: "";
  position: absolute;
  top: 0;
  left: 1.75rem;
  right: 1.75rem;
  height: 1px;
  background: linear-gradient(
    90deg,
    transparent,
    var(--color-oxblood),
    transparent
  );
  opacity: 0.5;
}
.cost-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border-radius: 8px;
  background: rgba(229, 72, 77, 0.08);
  border: 1px solid rgba(229, 72, 77, 0.2);
  color: var(--color-oxblood);
}
.cost-tag {
  color: var(--color-oxblood);
  border-color: rgba(229, 72, 77, 0.25);
  background: rgba(229, 72, 77, 0.05);
  align-self: flex-start;
}
.cost-title {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--color-bone);
  line-height: 1.4;
  margin: 0;
}
.cost-desc {
  font-size: 0.9375rem;
  line-height: 1.6;
  color: var(--color-ash);
  margin: 0;
}
</style>
