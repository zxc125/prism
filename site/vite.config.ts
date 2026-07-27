import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

// 官网（营销站）独立 Vite 配置 - 与 console（src/）分开构建
// 技术栈见 docs/品牌/官网（方案）.md §6.9
export default defineConfig({
  plugins: [vue(), tailwindcss()],
  server: {
    // console 占用 1420；官网用 4321，避免并行 dev 冲突
    port: 4321,
    strictPort: true,
  },
  build: {
    outDir: "dist",
  },
});
