<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from "vue";
import { useRouter } from "vue-router";
import {
  getBackend,
  resetBackend,
  type SessionMeta,
  type Source,
} from "../../composables/backend";
import { isTauri, pickBundleFile, onWindowFocus } from "../../composables/tauri";
import { sourceOf } from "../common/format";
import SessionCard from "./SessionCard.vue";
import SessionFilters from "./SessionFilters.vue";
import EmptyState from "../common/EmptyState.vue";
import SkeletonList from "../common/SkeletonList.vue";

/** 会话列表视图：浏览 + 筛选 + 搜索 + 导入 + 编辑/导出/删除。 */
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
        <SessionFilters v-model:src-filter="srcFilter" v-model:search="search" />
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
        :hint="sessions.length
          ? '调整筛选或搜索词试试'
          : '在「实时」页本机通道开始录制，或用 web SDK 上报，也可导入已有 bundle'"
        :cta-label="tauri ? '去录制' : undefined"
        @cta="router.push('/live')"
      />
      <SessionCard
        v-for="s in filtered"
        :key="s.id"
        :session="s"
        @open="openPlayer"
        @edit="openEdit"
        @export="exportSession"
        @delete="deleteSession"
      />
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
          <el-input v-model="editForm.name" placeholder="给这个会话起个名字" maxlength="60" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="editForm.note" type="textarea" :rows="3" placeholder="发生了什么、如何复现…" />
        </el-form-item>
        <el-form-item label="标签">
          <el-input v-model="editForm.tagsStr" placeholder="逗号分隔，如 login, bug" />
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
  overflow-y: auto;
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
  transition: color 0.12s, border-color 0.12s, background 0.12s;
}
.import-btn:hover {
  color: var(--bone-dim);
  border-color: var(--ash-deep);
}
.import-btn .mono { color: var(--amber); }
.edit-form :deep(.el-form-item__label) {
  font-size: var(--fs-xs);
  padding-bottom: 4px;
}
</style>
