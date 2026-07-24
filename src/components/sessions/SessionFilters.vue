<script setup lang="ts">
import { SRC_FILTERS } from "../common/format";
import type { Source } from "../../composables/backend";

/** 筛选 + 搜索条。v-model 双向绑定 srcFilter / search。 */
const srcFilter = defineModel<"all" | Source>("srcFilter", { default: "all" });
const search = defineModel<string>("search", { default: "" });
</script>

<template>
  <div class="filters">
    <button
      v-for="c in SRC_FILTERS"
      :key="c.key"
      class="chip mono"
      :class="{ 'is-active': srcFilter === c.key }"
      @click="srcFilter = c.key"
    >
      {{ c.label }}
    </button>
    <el-input
      v-model="search"
      class="search"
      placeholder="搜索 ID / 名称"
      size="small"
      clearable
    />
  </div>
</template>

<style scoped>
.filters {
  display: flex;
  align-items: center;
  gap: 6px;
}
.chip {
  appearance: none;
  border: 1px solid var(--hair);
  background: transparent;
  color: var(--ash);
  font-size: var(--fs-xs);
  padding: 5px 11px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  letter-spacing: 0.04em;
  transition: color 0.12s, border-color 0.12s, background 0.12s;
}
.chip:hover {
  color: var(--bone-dim);
  border-color: var(--ash-deep);
}
.chip.is-active {
  color: var(--ink);
  background: var(--amber);
  border-color: var(--amber);
}
.search { width: 220px; }
</style>
