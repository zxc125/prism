<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, h } from "vue";
import { useRouter } from "vue-router";
// P21：cellRenderer（render 函数）内的 EP 组件不经 unplugin resolver 注入样式
// （main.ts 无全局 EP css），手动 import 须显式携带 style；table-v2/auto-resizer 同理（O1 去险）。
import {
  ElButton,
  ElCheckbox,
  ElDropdown,
  ElDropdownMenu,
  ElDropdownItem,
} from "element-plus";
import type { Column } from "element-plus";
// FixedDir 是 enum（非字符串联合），主入口只导出类型不导出运行时值——从 table-v2 constants 取值
import { FixedDir } from "element-plus/es/components/table-v2/src/constants";
import "element-plus/es/components/button/style/css";
import "element-plus/es/components/checkbox/style/css";
import "element-plus/es/components/dropdown/style/css";
import "element-plus/es/components/table-v2/style/css";
import {
  getBackend,
  resetBackend,
  type SessionMeta,
  type Source,
} from "../../composables/backend";
import {
  isTauri,
  pickBundleFile,
  onWindowFocus,
} from "../../composables/tauri";
import {
  sourceOf,
  SRC_COLOR,
  SRC_LABEL,
  fmtClock,
  sessionDur,
  isSessionOpen,
} from "../common/format";
import StatusDot from "../common/StatusDot.vue";
import SessionFilters from "./SessionFilters.vue";
import EmptyState from "../common/EmptyState.vue";
import SkeletonList from "../common/SkeletonList.vue";

/** 会话列表视图：el-table-v2 通道台账（P21）+ 筛选 + 搜索 + 导入 + 编辑/导出/删除。 */
const router = useRouter();
const tauri = isTauri();

const sessions = ref<SessionMeta[]>([]);
const loading = ref(true);
const srcFilter = ref<"all" | Source>("all");
const search = ref("");

// 元信息编辑弹窗
const editOpen = ref(false);
const editForm = ref({ id: "", name: "", note: "", tagsStr: "" });

const filtered = computed(() => {
  let list = sessions.value;
  if (srcFilter.value !== "all") {
    list = list.filter((s) => sourceOf(s) === srcFilter.value);
  }
  const q = search.value.trim().toLowerCase();
  if (q)
    list = list.filter(
      (s) =>
        s.id.toLowerCase().includes(q) ||
        (s.name?.toLowerCase().includes(q) ?? false),
    );
  return list;
});

const listTitle = computed(() =>
  sessions.value.length ? `${sessions.value.length} 个会话` : "会话观测台",
);

// 批量删除（P21 增量）：选择只经 checkbox 触发；全选语义 = 当前筛选结果全集
// （虚拟表格只渲染可视区，全选须作用于 filtered 全集才有用）
const selected = ref<Set<string>>(new Set());
const selectedCount = computed(() => selected.value.size);
const allSelected = computed(
  () =>
    filtered.value.length > 0 &&
    filtered.value.every((s) => selected.value.has(s.id)),
);
const someSelected = computed(() =>
  filtered.value.some((s) => selected.value.has(s.id)),
);
function toggleAll(v: boolean) {
  const next = new Set(selected.value);
  for (const s of filtered.value) {
    if (v) next.add(s.id);
    else next.delete(s.id);
  }
  selected.value = next;
}
function toggleOne(id: string, v: boolean) {
  const next = new Set(selected.value);
  if (v) next.add(id);
  else next.delete(id);
  selected.value = next;
}
async function batchDelete() {
  const ids = [...selected.value];
  if (!ids.length) return;
  try {
    await ElMessageBox.confirm(
      `删除 ${ids.length} 个录制后无法恢复。`,
      "批量删除录制",
      {
        type: "warning",
        confirmButtonText: "删除",
        cancelButtonText: "取消",
      },
    );
  } catch {
    return;
  }
  try {
    for (const id of ids) await getBackend().deleteSession(id);
    await refresh();
    selected.value = new Set();
    ElMessage.success(`已删除 ${ids.length} 个会话`);
  } catch (e) {
    ElMessage.error(`删除失败: ${e}`);
    await refresh();
  }
}

async function refresh() {
  loading.value = true;
  try {
    sessions.value = await getBackend().listSessions();
  } catch (e) {
    ElMessage.error(`读取列表失败: ${e}`);
  } finally {
    loading.value = false;
  }
}

function openPlayer(id: string) {
  // D3：player 走 in-app 路由 /s/:id（两端统一），不再开独立窗口
  router.push(`/s/${id}`);
}

async function deleteSession(id: string) {
  try {
    await ElMessageBox.confirm("删除该录制后无法恢复。", "删除录制", {
      type: "warning",
      confirmButtonText: "删除",
      cancelButtonText: "取消",
    });
  } catch {
    return;
  }
  try {
    await getBackend().deleteSession(id);
    await refresh();
    ElMessage.success("已删除");
  } catch (e) {
    ElMessage.error(`删除失败: ${e}`);
  }
}

function openEdit(s: SessionMeta) {
  editForm.value = {
    id: s.id,
    name: s.name ?? "",
    note: s.note ?? "",
    tagsStr: (s.tags ?? []).join(", "),
  };
  editOpen.value = true;
}

async function saveEdit() {
  const { id, name, note, tagsStr } = editForm.value;
  const tags = tagsStr
    .split(",")
    .map((t) => t.trim())
    .filter(Boolean);
  try {
    await getBackend().updateSessionMeta(id, {
      name: name.trim(),
      note: note.trim(),
      tags,
    });
    editOpen.value = false;
    await refresh();
    ElMessage.success("已保存");
  } catch (e) {
    ElMessage.error(`保存失败: ${e}`);
  }
}

async function exportSession(s: SessionMeta) {
  try {
    const bundle = await getBackend().exportSession(s.id);
    const base = (s.name || s.id).replace(/[^\w一-龥-]/g, "_");
    const blob = new Blob([JSON.stringify(bundle, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${base}.rrweb-session.json`;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
    ElMessage.success("已导出");
  } catch (e) {
    ElMessage.error(`导出失败: ${e}`);
  }
}

async function triggerImport() {
  const picked = await pickBundleFile();
  if (!picked) return;
  try {
    if ("path" in picked) {
      await getBackend().importBundlePath(picked.path);
    } else {
      await getBackend().importBundleContent(picked.content);
    }
    await refresh();
    ElMessage.success("已导入会话");
  } catch (err) {
    ElMessage.error(`导入失败: ${err}`);
  }
}

// ---- el-table-v2 列渲染（P21）----
// cellRenderer 输出不带 scoped data-v，行样式驻 theme.css 的 .sess-table/.st-*（非 scoped）。

function onCommand(cmd: string, s: SessionMeta) {
  if (cmd === "delete") deleteSession(s.id);
  else if (cmd === "edit") openEdit(s);
  else if (cmd === "export") exportSession(s);
}

const columns: Column[] = [
  {
    key: "sel",
    dataKey: "sel",
    title: "",
    width: 48,
    headerCellRenderer: () =>
      h(ElCheckbox, {
        modelValue: allSelected.value,
        indeterminate: someSelected.value && !allSelected.value,
        "onUpdate:modelValue": (v: unknown) => toggleAll(Boolean(v)),
        ariaLabel: "全选",
      }),
    cellRenderer: ({ rowData }) => {
      const s = rowData as SessionMeta;
      return h(ElCheckbox, {
        modelValue: selected.value.has(s.id),
        "onUpdate:modelValue": (v: unknown) => toggleOne(s.id, Boolean(v)),
        ariaLabel: `选择会话 ${s.id}`,
      });
    },
  },
  {
    key: "src",
    dataKey: "source",
    title: "来源",
    width: 76,
    cellRenderer: ({ rowData }) => {
      const s = rowData as SessionMeta;
      return h("span", { class: "st-src mono" }, [
        h(StatusDot, { color: SRC_COLOR[sourceOf(s)], size: 9 }),
        SRC_LABEL[sourceOf(s)],
      ]);
    },
  },
  {
    key: "time",
    dataKey: "startedAt",
    title: "时间",
    width: 120,
    cellRenderer: ({ rowData }) =>
      h(
        "span",
        { class: "mono st-time" },
        fmtClock((rowData as SessionMeta).startedAt),
      ),
  },
  {
    key: "dur",
    dataKey: "endedAt",
    title: "时长",
    width: 92,
    cellRenderer: ({ rowData }) => {
      const s = rowData as SessionMeta;
      // P21 D4：未结束显式徽标，不再渲染 now 兜底的增长数值
      return isSessionOpen(s)
        ? h("span", { class: "st-open mono" }, "未结束")
        : h("span", { class: "mono st-dur" }, sessionDur(s) ?? "—");
    },
  },
  {
    key: "name",
    dataKey: "name",
    title: "会话",
    width: 200,
    minWidth: 200,
    flexGrow: 1,
    cellRenderer: ({ rowData }) => {
      const s = rowData as SessionMeta;
      const label = s.name || s.id;
      return h(
        "span",
        { class: "st-name", title: s.name ? `${s.name} · ${s.id}` : s.id },
        [
          h("span", { class: ["mono", s.name ? "is-named" : "st-id"] }, label),
          s.importedAt ? h("span", { class: "st-tag mono" }, "导入") : null,
        ],
      );
    },
  },
  {
    key: "user",
    dataKey: "user",
    title: "用户",
    width: 110,
    cellRenderer: ({ rowData }) => {
      const u = (rowData as SessionMeta).user;
      // EP CellRenderer 不收 null：无 user 渲染空 span（留空，与 App 列空值一致）
      if (!u) return h("span", { class: "st-user" });
      // 人名走 sans（等宽仅限时间码/ID/技术值）；无 name 退化 id 归技术值走 mono
      return h(
        "span",
        {
          class: u.name ? "st-user" : "mono st-user is-id",
          title: u.name ? `${u.name} · ${u.id}` : u.id,
        },
        u.name || u.id,
      );
    },
  },
  {
    key: "app",
    dataKey: "appId",
    title: "App",
    width: 140,
    cellRenderer: ({ rowData }) => {
      const app = (rowData as SessionMeta).appId;
      return h("span", { class: "mono st-app" }, app ?? "");
    },
  },
  {
    key: "ops",
    dataKey: "ops",
    title: "操作",
    width: 124,
    fixed: FixedDir.RIGHT,
    align: "right",
    cellRenderer: ({ rowData }) => {
      const s = rowData as SessionMeta;
      return h("span", { class: "st-ops" }, [
        h(
          ElButton,
          { size: "small", type: "primary", onClick: () => openPlayer(s.id) },
          () => "回放",
        ),
        h(
          ElDropdown,
          { trigger: "click", onCommand: (cmd: string) => onCommand(cmd, s) },
          {
            default: () =>
              h(ElButton, { size: "small", class: "more" }, () => "⋯"),
            dropdown: () =>
              h(ElDropdownMenu, null, () => [
                h(ElDropdownItem, { command: "edit" }, () => "编辑信息"),
                h(ElDropdownItem, { command: "export" }, () => "导出"),
                h(
                  ElDropdownItem,
                  { command: "delete", divided: true },
                  () => "删除",
                ),
              ]),
          },
        ),
      ]);
    },
  },
];

// 切回窗口刷新（Tauri）；浏览器无此事件，路由切回时 onMounted 自动刷
let unlistenFocus: (() => void) | null = null;
onMounted(async () => {
  await refresh();
  unlistenFocus = await onWindowFocus((focused) => {
    if (focused) {
      // 设置页改 Backend 后同步
      resetBackend();
      void refresh();
    }
  });
});
onBeforeUnmount(() => unlistenFocus?.());

defineExpose({ refresh });
</script>

<template>
  <section class="sessions-view">
    <header class="sv-head">
      <div>
        <div class="eyebrow">会话观测</div>
        <h1 class="sv-title">{{ listTitle }}</h1>
      </div>
      <div class="sv-actions">
        <SessionFilters
          v-model:src-filter="srcFilter"
          v-model:search="search"
        />
        <button
          class="import-btn danger-btn"
          :disabled="!selectedCount"
          @click="batchDelete"
        >
          <span class="mono">✕</span>
          删除所选{{ selectedCount ? `(${selectedCount})` : "" }}
        </button>
        <button class="import-btn" @click="triggerImport">
          <span class="mono">↧</span> 导入
        </button>
      </div>
    </header>

    <div class="sv-list">
      <SkeletonList v-if="loading" :rows="6" />
      <EmptyState
        v-else-if="!filtered.length"
        :icon="sessions.length ? '⊘' : '◌'"
        :title="sessions.length ? '无匹配会话' : '暂无会话'"
        :hint="
          sessions.length
            ? '调整筛选或搜索词试试'
            : '在「实时」页本机通道开始录制，或用 web SDK 上报，也可导入已有 bundle'
        "
        :cta-label="tauri ? '去录制' : undefined"
        @cta="router.push('/live')"
      />
      <div v-else class="sess-table-wrap">
        <el-auto-resizer>
          <template #default="{ height, width }">
            <el-table-v2
              class="sess-table"
              :columns="columns"
              :data="filtered"
              :width="width"
              :height="height"
              :row-height="40"
              :header-height="36"
              row-key="id"
              fixed
            />
          </template>
        </el-auto-resizer>
      </div>
    </div>

    <!-- 元信息编辑弹窗 -->
    <el-dialog
      v-model="editOpen"
      title="会话信息"
      width="420px"
      :close-on-click-modal="false"
    >
      <el-form label-position="top" class="edit-form">
        <el-form-item label="名称">
          <el-input
            v-model="editForm.name"
            placeholder="给这个会话起个名字"
            maxlength="60"
          />
        </el-form-item>
        <el-form-item label="备注">
          <el-input
            v-model="editForm.note"
            type="textarea"
            :rows="3"
            placeholder="发生了什么、如何复现…"
          />
        </el-form-item>
        <el-form-item label="标签">
          <el-input
            v-model="editForm.tagsStr"
            placeholder="逗号分隔，如 login, bug"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="editOpen = false">取消</el-button>
        <el-button type="primary" @click="saveEdit">保存</el-button>
      </template>
    </el-dialog>
  </section>
</template>

<style scoped>
.sessions-view {
  display: flex;
  flex-direction: column;
  padding: 22px 24px;
  min-width: 0;
  height: 100%;
}
.sv-head {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 18px;
}
.sv-title {
  margin: 4px 0 0;
  font-size: var(--fs-xl);
  font-weight: 600;
  color: var(--bone);
  letter-spacing: -0.01em;
}
.sv-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.sv-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
  min-height: 0;
}
.sess-table-wrap {
  flex: 1;
  min-height: 0;
}
.import-btn {
  appearance: none;
  border: 1px solid var(--hair);
  background: transparent;
  color: var(--ash);
  font-family: var(--font-sans);
  font-size: var(--fs-xs);
  padding: 5px 11px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  letter-spacing: 0.04em;
  display: flex;
  align-items: center;
  gap: 5px;
  transition:
    color 0.12s,
    border-color 0.12s,
    background 0.12s;
}
.import-btn:hover {
  color: var(--bone-dim);
  border-color: var(--ash-deep);
}
.import-btn .mono {
  color: var(--amber);
}
.danger-btn {
  color: var(--oxblood-soft);
  border-color: color-mix(in srgb, var(--oxblood) 45%, transparent);
}
.danger-btn .mono {
  color: var(--oxblood-soft);
}
.danger-btn:hover:not(:disabled) {
  color: var(--oxblood-soft);
  border-color: var(--oxblood);
  background: var(--oxblood-tint);
}
.danger-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.edit-form :deep(.el-form-item__label) {
  font-size: var(--fs-xs);
  padding-bottom: 4px;
}
</style>
