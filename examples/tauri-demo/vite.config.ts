import { defineConfig } from "vite";

// Tauri 期望固定端口；clearScreen false 避免吞掉 Rust 日志
export default defineConfig({
  clearScreen: false,
  server: {
    port: 1422,
    strictPort: true,
  },
  build: {
    target: "es2021",
  },
});
