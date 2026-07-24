<script setup lang="ts">
import { isTauri } from "../../composables/tauri";

/** 关于 tab：版本 + 模式 + 链接。 */
const tauri = isTauri();
const version = "2.0.0"; // P10 console 2.0
</script>

<template>
  <div class="tab-pane">
    <div class="about-block">
      <div class="ab-brand">
        <span class="ab-mark" aria-hidden="true">
          <svg viewBox="0 0 24 24" width="28" height="28" fill="none">
            <rect x="2" y="4" width="20" height="16" rx="2.5" stroke="currentColor" stroke-width="1.5" />
            <circle cx="6" cy="8" r="1" fill="currentColor" />
            <circle cx="6" cy="12" r="1" fill="currentColor" />
            <circle cx="6" cy="16" r="1" fill="currentColor" />
            <path d="M11 12l4 2-4 2z" fill="currentColor" />
          </svg>
        </span>
        <div>
          <div class="ab-name">replay · observer console</div>
          <div class="ab-version mono">v{{ version }} · {{ tauri ? "Tauri 桌面" : "浏览器" }}</div>
        </div>
      </div>
      <p class="ab-desc">
        本地优先的前端观测平台：录制 / 回放 / 诊断 / 导出 / 标注 / 多租户。
        P10 起支持浏览器零安装访问（observer-server 托管）。
      </p>
      <div class="ab-links">
        <a class="ab-link" href="#/settings">文档</a>
        <a class="ab-link" href="#/settings">bundle 规范</a>
        <a class="ab-link" href="#/settings">阶段路径</a>
      </div>
    </div>
  </div>
</template>

<style scoped>
.tab-pane { display: flex; flex-direction: column; gap: 20px; max-width: 640px; }
.about-block {
  background: var(--slate);
  border: 1px solid var(--hair);
  border-radius: var(--radius);
  padding: 24px;
}
.ab-brand {
  display: flex;
  align-items: center;
  gap: 14px;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--hair-soft);
}
.ab-mark {
  color: var(--amber);
  display: flex;
  filter: drop-shadow(0 0 8px var(--amber-glow));
}
.ab-name {
  font-size: var(--fs-lg);
  font-weight: 600;
  color: var(--bone);
}
.ab-version {
  font-size: var(--fs-xs);
  color: var(--ash);
  margin-top: 2px;
}
.ab-desc {
  font-size: var(--fs-sm);
  color: var(--bone-dim);
  line-height: 1.6;
  margin: 16px 0;
}
.ab-links {
  display: flex;
  gap: 14px;
}
.ab-link {
  color: var(--amber);
  font-size: var(--fs-sm);
  text-decoration: none;
  transition: color 0.12s;
}
.ab-link:hover { color: var(--amber-soft); }
</style>
