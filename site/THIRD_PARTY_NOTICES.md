# Third-Party Notices

本文件汇总 `site/` 引用的第三方组件来源与许可。

## vue-bits 组件

`site/src/components/` 下通过 [jsrepo](https://jsrepo.dev/) 从 [`DavidHDev/vue-bits`](https://github.com/DavidHDev/vue-bits) 拉取的组件源码，遵循其原始许可：

- **来源**：https://github.com/DavidHDev/vue-bits
- **许可**：MIT + Commons Clause License Condition v1.0
- **版权**：Copyright (c) 2025 David Haz

许可要点（非法律摘要，详见 [LICENSE.md](https://github.com/DavidHDev/vue-bits/blob/main/LICENSE.md)）：

- ✅ 允许作为 application / website / product 的一部分使用、复制、修改、分发，含商业目的
- ❌ 禁止出售、sublicense 或再分发组件本身--无论单独、打包、模板还是 ported 版本

**约束**：拉取的组件源码内须保留原始版权声明与许可条款，不得移除。鉴 / Prism 开源 `site/` 仓库时，此文件作为第三方来源披露。

---

## 拉取记录

| 组件 | 来源路径 | 本地路径 | 拉取日期 | 依赖 |
| --- | --- | --- | --- | --- |
| Aurora | `Backgrounds/Aurora` (@vue-bits registry) | `site/src/components/Aurora.vue` | 2026-07-27 | `ogl@^1.0.11` |

> 注：因 `vue-bits.dev` 服务端对 jsrepo CLI 的 fetch 返回 SPA HTML（UA/内容协商问题），`jsrepo add` 不可用。组件源码改为从 GitHub 仓库 `DavidHDev/vue-bits` 直接拉取（`src/content/Backgrounds/Aurora/Aurora.vue`），源码与 registry 托管版本一致，版权头已保留。
