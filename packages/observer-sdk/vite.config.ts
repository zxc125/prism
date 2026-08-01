import { defineConfig } from "vite";
import { resolve } from "node:path";
import dts from "vite-plugin-dts";

// 库模式构建：产物 ESM + d.ts + sourcemap，rrweb 作为 peer 依赖外部化（由消费方提供）。
// 发布到 npm 的包入口指向 dist（见 package.json exports）；monorepo 内部 dev 走根
// vite.config.ts 的 alias 直连 src 源码（热更新），两条路径互不干扰。
export default defineConfig({
  plugins: [
    // rollupTypes 把散落的 .d.ts 合并成单个 index.d.ts，消费方无需 src 即可获得完整类型
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
      external: ["rrweb"],
    },
  },
});
