<script setup lang="ts">
/** 设计过的空状态：图标 + 标题 + 副文案 + 可选 CTA。取代「一行字」凑合。 */
defineProps<{
  icon?: string;
  title: string;
  hint?: string;
  ctaLabel?: string;
}>();

const emit = defineEmits<{ (e: "cta"): void }>();
</script>

<template>
  <div class="empty-state">
    <div v-if="icon" class="es-icon mono" aria-hidden="true">{{ icon }}</div>
    <div v-else class="es-glyph" aria-hidden="true">
      <svg viewBox="0 0 48 48" width="40" height="40" fill="none">
        <rect x="6" y="10" width="36" height="28" rx="3" stroke="currentColor" stroke-width="1.5" opacity="0.5" />
        <path d="M6 18h36" stroke="currentColor" stroke-width="1.5" opacity="0.5" />
        <circle cx="10" cy="14" r="1" fill="currentColor" opacity="0.5" />
      </svg>
    </div>
    <div class="es-title">{{ title }}</div>
    <div v-if="hint" class="es-hint">{{ hint }}</div>
    <el-button v-if="ctaLabel" size="small" type="primary" @click="emit('cta')">
      {{ ctaLabel }}
    </el-button>
  </div>
</template>

<style scoped>
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 72px 24px;
  text-align: center;
}
.es-icon {
  font-size: 36px;
  color: var(--ash-deep);
  line-height: 1;
}
.es-glyph {
  color: var(--ash-deep);
  opacity: 0.7;
}
.es-title {
  font-size: var(--fs-md);
  color: var(--bone-dim);
  font-weight: 500;
}
.es-hint {
  font-size: var(--fs-sm);
  color: var(--ash-deep);
  max-width: 360px;
  line-height: 1.6;
}
</style>
