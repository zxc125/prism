import { createRouter, createWebHashHistory } from "vue-router";
import SessionsView from "../views/SessionsView.vue";
import LiveView from "../views/LiveView.vue";
import SettingsView from "../views/SettingsView.vue";
import PlayerView from "../views/PlayerView.vue";
import TenantsView from "../views/TenantsView.vue";
import TenantDetailView from "../views/TenantDetailView.vue";

// 使用 hash 模式：Tauri 打包后走自定义协议/本地文件，
// history 模式深链或刷新会 404，hash 模式最稳。
// P10：浏览器化同样依赖 hash 模式（observer-server 静态托管 SPA fallback）。
const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "sessions", component: SessionsView },
    { path: "/live", name: "live", component: LiveView },
    { path: "/s/:id", name: "player", component: PlayerView },
    { path: "/tenants", name: "tenants", component: TenantsView },
    { path: "/tenants/:id", name: "tenant-detail", component: TenantDetailView },
    { path: "/settings", name: "settings", component: SettingsView },
    // 兼容旧 /player/:id -> 重定向到 /s/:id（已开过的旧窗口 hash 仍可用）
    { path: "/player/:id", redirect: (to) => `/s/${to.params.id}` },
  ],
});

export default router;
