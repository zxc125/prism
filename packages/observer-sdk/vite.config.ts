import { defineConfig } from "vite";
import { resolve } from "node:path";

// 库模式构建：产物 ESM + d.ts，rrweb 作为 peer 依赖外部化（由消费方提供）。
// 日常被本仓 Tauri 应用直接按 src 引用（见 package.json exports），无需预构建。
export default defineConfig({
  build: {
    lib: {
      entry: resolve(__dirname, "src/index.ts"),
      formats: ["es"],
      fileName: "index",
    },
    rollupOptions: {
      external: ["rrweb"],
    },
  },
});
