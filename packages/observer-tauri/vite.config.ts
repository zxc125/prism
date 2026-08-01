import { defineConfig } from "vite";
import { resolve } from "node:path";
import dts from "vite-plugin-dts";

// 库模式构建：产物 ESM + d.ts + sourcemap。
// @tauri-apps/api 与 rrweb 为 peer 依赖（消费方 Tauri 运行时自带），external 化；
// @prism-obs/observer-sdk 为 dependency 但同样 external（避免重复打包，由消费方 npm install 拉取）。
export default defineConfig({
  plugins: [
    dts({ rollupTypes: true, include: ["src/**/*.ts"] }),
  ],
  build: {
    lib: {
      entry: resolve(__dirname, "src/index.ts"),
      formats: ["es"],
      fileName: "index",
    },
    sourcemap: true,
    rollupOptions: {
      // 用正则匹配子路径（@tauri-apps/api/core 等），否则整个 api 会被打进 bundle
      external: [
        "rrweb",
        /^@tauri-apps\/api/,
        /^@prism-obs\/observer-sdk/,
      ],
    },
  },
});
