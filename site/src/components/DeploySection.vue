<script setup lang="ts">
// 部署形态 · 桌面 App / 单二进制 / 浏览器（方案 §6.8 第 8 节）
// 签名元素：三形态并置卡片，各带迷你拓扑 + 来源色点 + mono meta
import { Monitor, Terminal, Globe } from "lucide-vue-next";
import Reveal from "./Reveal.vue";

const forms = [
  {
    icon: Monitor,
    accent: "amber",
    dot: "amber",
    tag: "桌面 App",
    title: "Tauri 2 桌面应用",
    desc: "macOS / Windows / Linux，零云依赖。本地分析台，开箱即用，数据落 appDataDir。",
    meta: ["Tauri 2", "跨平台", "零云依赖"],
  },
  {
    icon: Terminal,
    accent: "teal",
    dot: "teal",
    tag: "单二进制",
    title: "observer-server",
    desc: "一个二进制同时托管 API + console 前端。--web-dir 启用 SPA，绑 0.0.0.0 即公网，多租户配额。",
    meta: ["单文件", "SPA 托管", "多租户"],
  },
  {
    icon: Globe,
    accent: "amber",
    dot: "amber",
    tag: "浏览器",
    title: "纯浏览器访问",
    desc: "console 可纯浏览器跑，LoginGate 输 endpoint + key 即用。无需安装，HttpBackend 直连自托管 server。",
    meta: ["零安装", "HttpBackend", "LoginGate"],
  },
];
</script>

<template>
  <section id="deploy" class="section deploy">
    <div class="section-inner">
      <Reveal>
        <header class="section-head">
          <p class="eyebrow mono">部署形态 · 三选一</p>
          <h2 class="section-h2">
            内网、离线、合规审计--<br />
            <span class="accent-teal">总有一款拓扑</span>接得住。
          </h2>
          <p class="section-sub">
            同一份核心代码，三种部署形态：桌面 App 零云依赖，单二进制自托管，纯浏览器零安装。从个人开发者到政企内网，按需选拓扑，数据始终在你手里。
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
