import { defineConfig } from "vite";
import { fileURLToPath, URL } from "node:url";

// monorepo 内部 dev 直连 observer-tauri 源码（热更新）；npm 发布的消费方走
// package.json exports → dist。
const observerTauriSrc = fileURLToPath(
  new URL("../../packages/observer-tauri/src/index.ts", import.meta.url),
);

// Tauri 期望固定端口；clearScreen false 避免吞掉 Rust 日志
export default defineConfig({
  resolve: {
    alias: {
      "@prism-obs/observer-tauri": observerTauriSrc,
    },
  },
  clearScreen: false,
  server: {
    port: 1422,
    strictPort: true,
  },
  build: {
    target: "es2021",
  },
});
