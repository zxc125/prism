<script setup lang="ts">
import type { SessionMeta } from "../../composables/backend";
import { sourceOf, SRC_COLOR, SRC_LABEL, fmtClock, sessionDur } from "../common/format";
import StatusDot from "../common/StatusDot.vue";

/** 单条会话卡：来源色点 + 时间 + 时长 + 名称/ID + 操作。 */
const props = defineProps<{ session: SessionMeta }>();
const emit = defineEmits<{
  (e: "open", id: string): void;
  (e: "edit", s: SessionMeta): void;
  (e: "export", s: SessionMeta): void;
  (e: "delete", id: string): void;
}>();

function onCommand(cmd: string, s: SessionMeta) {
  if (cmd === "delete") emit("delete", s.id);
  else if (cmd === "edit") emit("edit", s);
  else if (cmd === "export") emit("export", s);
}
</script>

<template>
  <div class="sess-card">
    <StatusDot :color="SRC_COLOR[sourceOf(props.session)]" :size="9" />
    <span class="sc-src mono">{{ SRC_LABEL[sourceOf(props.session)] }}</span>
    <span class="sc-time mono">{{ fmtClock(props.session.startedAt) }}</span>
    <span class="sc-dur mono">{{ sessionDur(props.session) }}</span>
    <span
      class="sc-id mono"
      :class="{ 'is-named': props.session.name }"
      :title="props.session.name ? `${props.session.name} · ${props.session.id}` : props.session.id"
    >{{ props.session.name || props.session.id }}</span>
    <span v-if="props.session.importedAt" class="sc-tag mono">导入</span>
    <span class="sc-spacer" />
    <el-button size="small" type="primary" @click="emit('open', props.session.id)">回放</el-button>
    <el-dropdown trigger="click" @command="(cmd: string) => onCommand(cmd, props.session)">
      <el-button size="small" class="more" @click.stop>⋯</el-button>
      <template #dropdown>
        <el-dropdown-menu>
          <el-dropdown-item command="edit">编辑信息</el-dropdown-item>
          <el-dropdown-item command="export">导出</el-dropdown-item>
          <el-dropdown-item command="delete" divided>删除</el-dropdown-item>
        </el-dropdown-menu>
      </template>
    </el-dropdown>
  </div>
</template>

<style scoped>
.sess-card {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 11px 12px;
  border: 1px solid transparent;
  border-radius: var(--radius);
  transition: background 0.12s, border-color 0.12s;
}
.sess-card:hover {
  background: var(--slate);
  border-color: var(--hair-soft);
}
.sc-src {
  font-size: var(--fs-xs);
  color: var(--bone-dim);
  letter-spacing: 0.06em;
  width: 44px;
}
.sc-time {
  font-size: var(--fs-sm);
  color: var(--bone-dim);
}
.sc-dur {
  font-size: var(--fs-sm);
  color: var(--ash);
  width: 56px;
}
.sc-id {
  font-size: var(--fs-xs);
  color: var(--ash-deep);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 280px;
}
.sc-id.is-named { color: var(--bone); }
.sc-tag {
  font-size: 10px;
  color: var(--src-tauri);
  border: 1px solid color-mix(in srgb, var(--src-tauri) 40%, transparent);
  border-radius: var(--radius-sm);
  padding: 0 5px;
  letter-spacing: 0.06em;
  flex-shrink: 0;
}
.sc-spacer { flex: 1; }
.more { font-family: var(--font-mono); }
</style>
