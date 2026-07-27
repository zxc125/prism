import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

// 官网（营销站）独立 Vite 配置 - 与 console（src/）分开构建
// 技术栈见 docs/品牌/官网（方案）.md §6.9
// base：GH Pages 项目页需 /<repo>/ 前缀（CI 注入 BASE_URL），自部署到根域名用 /（默认）
// 不引 @types/node：globalThis 取 process，仅在 Node 构建期生效
const baseUrl =
  (
    globalThis as unknown as {
      process?: { env?: Record<string, string | undefined> };
    }
  ).process?.env?.BASE_URL || "/";

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  base: baseUrl,
  server: {
    // console 占用 1420；官网用 4321，避免并行 dev 冲突
    port: 4321,
    strictPort: true,
  },
  build: {
    outDir: "dist",
  },
});
