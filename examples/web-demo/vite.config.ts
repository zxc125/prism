import { defineConfig } from "vite";

// 被观测样例应用：独立于 console 运行，嵌入 observer-sdk 上报到本地 server。
export default defineConfig({
  server: {
    port: 1422,
    strictPort: true,
  },
});
