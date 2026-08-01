<script setup lang="ts">
// 路由分发：营销页（frontmatter.layout === 'marketing'）渲染 MarketingPage；
// 其余（文档页）走 VitePress 默认主题 Layout（nav + sidebar + content）。
import DefaultTheme from "vitepress/theme";
import { useData } from "vitepress";
import MarketingPage from "./components/MarketingPage.vue";

const { Layout } = DefaultTheme;
const { frontmatter } = useData();
</script>

<template>
  <MarketingPage v-if="frontmatter.layout === 'marketing'" />
  <Layout v-else />
</template>
