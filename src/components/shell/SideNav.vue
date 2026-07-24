<script setup lang="ts">
import { useRoute } from "vue-router";
import { isTauri } from "../../composables/tauri";

/** 持久侧栏导航：Sessions / Live / Tenants / Settings。
 *  Live 的本机通道在浏览器隐藏（D6：浏览器无法自录）。 */
const route = useRoute();
const tauri = isTauri();

const nav = [
  { route: "/", label: "会话", icon: "sessions", match: (p: string) => p === "/" || p.startsWith("/s/") },
  { route: "/live", label: "实时", icon: "live", match: (p: string) => p.startsWith("/live") },
  { route: "/tenants", label: "租户", icon: "tenants", match: (p: string) => p.startsWith("/tenants") },
  { route: "/settings", label: "设置", icon: "settings", match: (p: string) => p.startsWith("/settings") },
];

// 浏览器隐藏 Live（无法自录）；保留 web/tauri 通道观测入口在 Live 内部
const visibleNav = computed(() =>
  nav.filter((n) => n.route !== "/live" || tauri),
);
</script>

<template>
  <nav class="side-nav">
    <RouterLink to="/" class="brand" aria-label="replay observer">
      <span class="brand-mark" aria-hidden="true">
        <svg viewBox="0 0 24 24" width="20" height="20" fill="none">
          <rect x="2" y="4" width="20" height="16" rx="2.5" stroke="currentColor" stroke-width="1.5" />
          <circle cx="6" cy="8" r="1" fill="currentColor" />
          <circle cx="6" cy="12" r="1" fill="currentColor" />
          <circle cx="6" cy="16" r="1" fill="currentColor" />
          <path d="M11 12l4 2-4 2z" fill="currentColor" />
        </svg>
      </span>
      <span class="brand-name">replay</span>
    </RouterLink>

    <div class="nav-list">
      <RouterLink
        v-for="item in visibleNav"
        :key="item.route"
        :to="item.route"
        class="nav-item"
        :class="{ 'is-active': item.match(route.path) }"
      >
        <span class="nav-icon" aria-hidden="true">
          <svg v-if="item.icon === 'sessions'" viewBox="0 0 24 24" width="18" height="18" fill="none">
            <rect x="3" y="5" width="18" height="14" rx="2" stroke="currentColor" stroke-width="1.5" />
            <path d="M3 9h18" stroke="currentColor" stroke-width="1.5" />
            <path d="M7 13h7M7 16h5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
          </svg>
          <svg v-else-if="item.icon === 'live'" viewBox="0 0 24 24" width="18" height="18" fill="none">
            <circle cx="12" cy="12" r="3" fill="currentColor" />
            <circle cx="12" cy="12" r="7" stroke="currentColor" stroke-width="1.5" opacity="0.5" />
            <circle cx="12" cy="12" r="10.5" stroke="currentColor" stroke-width="1.5" opacity="0.25" />
          </svg>
          <svg v-else-if="item.icon === 'tenants'" viewBox="0 0 24 24" width="18" height="18" fill="none">
            <path d="M4 7l8-4 8 4-8 4z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" />
            <path d="M4 12l8 4 8-4M4 17l8 4 8-4" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" opacity="0.5" />
          </svg>
          <svg v-else viewBox="0 0 24 24" width="18" height="18" fill="none">
            <circle cx="12" cy="12" r="3" stroke="currentColor" stroke-width="1.5" />
            <path d="M12 2v3M12 19v3M2 12h3M19 12h3M5 5l2 2M17 17l2 2M5 19l2-2M17 7l2-2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
          </svg>
        </span>
        <span class="nav-label">{{ item.label }}</span>
      </RouterLink>
    </div>

    <div class="nav-foot">
      <span class="nav-version mono">observer · console 2.0</span>
    </div>
  </nav>
</template>

<style scoped>
.side-nav {
  display: flex;
  flex-direction: column;
  width: 200px;
  flex-shrink: 0;
  background: var(--ink-2);
  border-right: 1px solid var(--hair);
  padding: 16px 10px;
}
.brand {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px 16px;
  color: var(--bone);
  text-decoration: none;
}
.brand-mark {
  color: var(--amber);
  display: flex;
  filter: drop-shadow(0 0 6px var(--amber-glow));
}
.brand-name {
  font-size: var(--fs-md);
  font-weight: 600;
  letter-spacing: 0.02em;
}
.nav-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  color: var(--ash);
  font-size: var(--fs-sm);
  text-decoration: none;
  transition: color 0.12s, background 0.12s;
}
.nav-item:hover {
  color: var(--bone-dim);
  background: var(--slate);
}
.nav-item.is-active {
  color: var(--bone);
  background: var(--slate-2);
  box-shadow: inset 2px 0 0 var(--amber);
}
.nav-item.is-active .nav-icon {
  color: var(--amber);
}
.nav-icon {
  display: flex;
  color: var(--ash-deep);
  transition: color 0.12s;
}
.nav-foot {
  margin-top: auto;
  padding: 12px 10px 4px;
}
.nav-version {
  font-size: 10px;
  color: var(--ash-deep);
  letter-spacing: 0.08em;
}
</style>
