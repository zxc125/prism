// 鉴 / Prism 官网 VitePress 配置（P13）
// 营销页 `/`（index.md, layout: marketing）+ 文档 `/docs/*`；en locale 在 `/en/`
// 技术栈见 docs/架构/官网文档站（方案）.md §4
import { defineConfig } from "vitepress";
import tailwindcss from "@tailwindcss/vite";

// GH Pages 项目页需 /<repo>/ 前缀（CI 注入 BASE_URL）；自部署到根域名用 /
const baseUrlRaw =
  (
    globalThis as unknown as {
      process?: { env?: Record<string, string | undefined> };
    }
  ).process?.env?.BASE_URL || "/";
const baseUrl = baseUrlRaw.endsWith("/") ? baseUrlRaw : baseUrlRaw + "/";
// head 里的静态资源引用要带 base 前缀（VitePress 不自动 patch head href）
const u = (p: string) => baseUrl + p.replace(/^\//, "");

const zhSidebar = [
  { text: "快速开始", link: "/docs/quickstart" },
  { text: "核心概念", link: "/docs/concepts" },
  { text: "Web SDK", link: "/docs/web" },
  { text: "Tauri Plugin", link: "/docs/tauri" },
  { text: "私有化部署", link: "/docs/deploy" },
];

const enSidebar = [
  { text: "Quick Start", link: "/en/docs/quickstart" },
  { text: "Core Concepts", link: "/en/docs/concepts" },
  { text: "Web SDK", link: "/en/docs/web" },
  { text: "Tauri Plugin", link: "/en/docs/tauri" },
  { text: "Self-Hosting", link: "/en/docs/deploy" },
];

export default defineConfig({
  title: "鉴 / Prism",
  description:
    "本地优先的前端观测平台。会话回放、诊断信号、多窗口对齐——数据留在你手里，不上云，不锁仓。",
  base: baseUrl,
  appearance: "force-dark",
  cleanUrls: true,
  lastUpdated: true,
  head: [
    ["link", { rel: "icon", type: "image/svg+xml", href: u("/logo.svg") }],
    ["link", { rel: "apple-touch-icon", href: u("/logo.svg") }],
    ["link", { rel: "manifest", href: u("/manifest.webmanifest") }],
    ["meta", { name: "theme-color", content: "#0A0C10" }],
    ["meta", { name: "apple-mobile-web-app-capable", content: "yes" }],
    [
      "meta",
      {
        name: "apple-mobile-web-app-status-bar-style",
        content: "black-translucent",
      },
    ],
    ["meta", { name: "apple-mobile-web-app-title", content: "鉴/Prism" }],
    ["meta", { property: "og:type", content: "website" }],
    [
      "meta",
      {
        property: "og:title",
        content: "鉴 / Prism — 本地优先的前端观测平台",
      },
    ],
    ["meta", { property: "og:image", content: u("/og-image.svg") }],
    ["meta", { name: "twitter:card", content: "summary_large_image" }],
    ["meta", { name: "twitter:image", content: u("/og-image.svg") }],
  ],
  vite: {
    plugins: [tailwindcss()],
  },
  locales: {
    // root = 简体中文（默认语种，内容在站点根）
    root: {
      label: "简体中文",
      lang: "zh-CN",
      themeConfig: {
        nav: [
          { text: "文档", link: "/docs/quickstart" },
          { text: "GitHub", link: "https://github.com/zxc125/prism" },
        ],
        sidebar: { "/docs/": zhSidebar },
        search: { provider: "local" },
        socialLinks: [
          { icon: "github", link: "https://github.com/zxc125/prism" },
        ],
        outline: { label: "本页目录" },
        docFooter: { prev: "上一篇", next: "下一篇" },
        sidebarMenuLabel: "菜单",
        returnToTopLabel: "回到顶部",
        darkModeSwitchLabel: "主题",
        siteTitle: "鉴 / Prism",
      },
    },
    en: {
      label: "English",
      lang: "en",
      link: "/en/docs/quickstart",
      themeConfig: {
        nav: [
          { text: "Docs", link: "/en/docs/quickstart" },
          { text: "GitHub", link: "https://github.com/zxc125/prism" },
        ],
        sidebar: { "/en/docs/": enSidebar },
        search: { provider: "local" },
        socialLinks: [
          { icon: "github", link: "https://github.com/zxc125/prism" },
        ],
        siteTitle: "Prism",
      },
    },
  },
});
