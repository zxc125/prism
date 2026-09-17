import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import AutoImport from "unplugin-auto-import/vite";
import Components from "unplugin-vue-components/vite";
import { ElementPlusResolver } from "unplugin-vue-components/resolvers";
import { fileURLToPath, URL } from "node:url";

// monorepo 内部 dev/build 直接走 SDK 源码（热更新）；npm 发布的消费方走 SDK package.json
// exports → dist。两端入口分离，避免 dev server 误用旧 dist 产物。
// P21 后补：observer-tauri 同样直连 src——此前走 node_modules → dist，CI 全新 checkout
// 无 dist 时 vite 构建必挂（与 tsconfig paths 配套，类型/运行时都脱离 dist）。
const sdkSrc = fileURLToPath(
  new URL("./packages/observer-sdk/src/index.ts", import.meta.url),
);
const tauriSrc = fileURLToPath(
  new URL("./packages/observer-tauri/src/index.ts", import.meta.url),
);

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [
    vue(),
    // Element Plus 按需自动导入：组件、ElMessage/ElMessageBox 等 API 及其样式
    // 生成类型声明到 src/ 下，由 tsconfig 的 include 自动收录
    AutoImport({
      imports: ["vue"],
      resolvers: [ElementPlusResolver()],
      dts: "src/auto-imports.d.ts",
    }),
    Components({
      resolvers: [ElementPlusResolver()],
      dts: "src/components.d.ts",
    }),
  ],

  resolve: {
    alias: {
      "@prism-obs/observer-sdk": sdkSrc,
      "@prism-obs/observer-tauri": tauriSrc,
    },
  },
  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  //    端口 1520：避让 Tauri 生态默认的 1420（多 Tauri 项目并行开发时冲突）
  server: {
    port: 1520,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1521,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
