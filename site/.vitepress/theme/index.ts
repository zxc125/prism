// 鉴 / Prism 官网 VitePress 主题（P13）
// - 覆盖 Layout：`/`（layout: marketing）渲染营销页；其余走默认文档壳
// - 全量导入品牌 token（src/styles/theme.css，含 Tailwind v4 入口）+ VitePress 品牌映射
// - 自动注册 src/components/*.vue 为全局组件（供 MarketingPage 与 md 复用）
import type { Theme } from "vitepress";
import DefaultTheme from "vitepress/theme";
import Layout from "./Layout.vue";
import "./styles/vp-overrides.css";
import "../../src/styles/theme.css";

// 营销页组件原样复用（src/components/），glob 注册为全局组件
const modules = import.meta.glob("../../src/components/*.vue", {
  eager: true,
}) as Record<string, { default: unknown }>;

export default {
  extends: DefaultTheme,
  Layout,
  enhanceApp({ app }) {
    for (const path of Object.keys(modules)) {
      const name = path.split("/").pop()!.replace(/\.vue$/, "");
      app.component(name, modules[path].default);
    }
  },
} satisfies Theme;
