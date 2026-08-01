<script setup lang="ts">
// 营销页 —— 原 site/src/App.vue 的 12 节组合，整体包 <ClientOnly>：
// Phase 1 不做逐组件 SSG 改造，营销页保持 CSR 行为（与上线前一致），SSR 只出 fallback。
// 组件直接从 src/components/ 导入（全局注册仅作 md 复用兜底）。
import SiteNav from "../../../src/components/SiteNav.vue";
import HeroSection from "../../../src/components/HeroSection.vue";
import PainSection from "../../../src/components/PainSection.vue";
import ManifestoSection from "../../../src/components/ManifestoSection.vue";
import CapabilitiesSection from "../../../src/components/CapabilitiesSection.vue";
import MultiWindowSection from "../../../src/components/MultiWindowSection.vue";
import DiagnosisSection from "../../../src/components/DiagnosisSection.vue";
import LoopSection from "../../../src/components/LoopSection.vue";
import DeploySection from "../../../src/components/DeploySection.vue";
import CompareSection from "../../../src/components/CompareSection.vue";
import ArchitectureSection from "../../../src/components/ArchitectureSection.vue";
import QuickstartSection from "../../../src/components/QuickstartSection.vue";
import FooterSection from "../../../src/components/FooterSection.vue";
</script>

<template>
  <ClientOnly>
    <SiteNav />
    <main id="top">
      <HeroSection />
      <PainSection />
      <ManifestoSection />
      <CapabilitiesSection />
      <MultiWindowSection />
      <DiagnosisSection />
      <LoopSection />
      <DeploySection />
      <CompareSection />
      <ArchitectureSection />
      <QuickstartSection />
    </main>
    <FooterSection />

    <template #fallback>
      <div class="mk-boot">
        <div class="mk-boot-inner">
          <span class="mk-beam beam-amber" />
          <span class="mk-beam beam-teal" />
          <p class="mk-boot-name">鉴 / Prism</p>
          <p class="mk-boot-loading">本地优先的前端观测平台</p>
        </div>
      </div>
    </template>
  </ClientOnly>
</template>

<style scoped>
.mk-boot {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-ink);
}
.mk-boot-inner {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.75rem;
}
.mk-beam {
  width: 56px;
  height: 2px;
  border-radius: 1px;
  display: block;
}
.beam-amber {
  background: var(--color-amber);
  box-shadow: 0 0 6px rgba(240, 168, 61, 0.4);
}
.beam-teal {
  background: var(--color-teal);
  box-shadow: 0 0 6px rgba(77, 208, 200, 0.4);
}
.mk-boot-name {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--color-bone);
  margin: 0.5rem 0 0;
}
.mk-boot-loading {
  font-size: 0.8125rem;
  color: var(--color-ash);
  margin: 0;
}
</style>
