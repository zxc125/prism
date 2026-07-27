<script setup lang="ts">
// 鉴 / Prism 官网 - 单页滚动 12 节（方案 §6.8）
import SiteNav from "./components/SiteNav.vue";
import HeroSection from "./components/HeroSection.vue";
import PainSection from "./components/PainSection.vue";
import ManifestoSection from "./components/ManifestoSection.vue";
import CapabilitiesSection from "./components/CapabilitiesSection.vue";
import MultiWindowSection from "./components/MultiWindowSection.vue";
import DiagnosisSection from "./components/DiagnosisSection.vue";
import LoopSection from "./components/LoopSection.vue";
import DeploySection from "./components/DeploySection.vue";
import CompareSection from "./components/CompareSection.vue";
import ArchitectureSection from "./components/ArchitectureSection.vue";
import QuickstartSection from "./components/QuickstartSection.vue";
import FooterSection from "./components/FooterSection.vue";
</script>

<template>
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
</template>
