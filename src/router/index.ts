import { createRouter, createWebHashHistory } from "vue-router";
import MainView from "../views/MainView.vue";
import SettingsView from "../views/SettingsView.vue";
import PlayerView from "../views/PlayerView.vue";

// 使用 hash 模式：Tauri 打包后走自定义协议/本地文件，
// history 模式深链或刷新会 404，hash 模式最稳。
const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "main", component: MainView },
    { path: "/settings", name: "settings", component: SettingsView },
    { path: "/player/:id", name: "player", component: PlayerView },
  ],
});

export default router;
