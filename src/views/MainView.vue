<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";

const name = ref("");
const greetMsg = ref("");

const recording = ref(false);
const sessions = ref<
  Array<{ id: string; startedAt: number; endedAt?: number }>
>([]);

async function greet() {
  if (!name.value) {
    ElMessage.warning("请输入名字");
    return;
  }
  try {
    greetMsg.value = await invoke<string>("greet", { name: name.value });
    ElMessage.success(greetMsg.value);
  } catch (e) {
    ElMessage.error(`调用失败: ${e}`);
  }
}

async function refreshSessions() {
  try {
    sessions.value = await invoke("list_sessions");
  } catch (e) {
    ElMessage.error(`读取列表失败: ${e}`);
  }
}

async function startSession() {
  try {
    await invoke("start_session");
    recording.value = true;
    ElMessage.success("已开始录制");
  } catch (e) {
    ElMessage.error(`开始失败: ${e}`);
  }
}

async function stopSession() {
  try {
    await invoke("stop_session");
    recording.value = false;
    ElMessage.success("已停止录制");
    await refreshSessions();
  } catch (e) {
    ElMessage.error(`停止失败: ${e}`);
  }
}

async function openSettings() {
  try {
    await invoke("open_window", { route: "/settings" });
  } catch (e) {
    ElMessage.error(`打开失败: ${e}`);
  }
}

async function openPlayer(id: string) {
  try {
    await invoke("open_window", { route: `/player/${id}` });
  } catch (e) {
    ElMessage.error(`打开失败: ${e}`);
  }
}

async function deleteSession(id: string) {
  try {
    await ElMessageBox.confirm("确定删除该录制？", "提示", {
      type: "warning",
    });
  } catch {
    return;
  }
  try {
    await invoke("delete_session", { id });
    await refreshSessions();
    ElMessage.success("已删除");
  } catch (e) {
    ElMessage.error(`删除失败: ${e}`);
  }
}

function fmt(ts?: number) {
  if (!ts) return "-";
  return new Date(ts).toLocaleString("zh-CN");
}

function duration(s: { startedAt: number; endedAt?: number } | Record<string, unknown>) {
  const end = (s as { endedAt?: number }).endedAt ?? Date.now();
  const sec = Math.round((end - (s as { startedAt: number }).startedAt) / 1000);
  return `${sec}s`;
}

onMounted(refreshSessions);
</script>

<template>
  <main class="page">
    <el-card class="box" shadow="hover">
      <template #header>
        <h2>Tauri + Vue + Element Plus</h2>
      </template>

      <p class="desc">输入名字调用 Rust <code>greet</code>，或打开新窗口。</p>

      <el-space>
        <el-input
          v-model="name"
          placeholder="请输入名字..."
          style="width: 240px"
          clearable
          @keyup.enter="greet"
        />
        <el-button type="primary" @click="greet">Greet</el-button>
      </el-space>

      <p v-if="greetMsg" class="result">{{ greetMsg }}</p>

      <el-divider />

      <el-space>
        <el-button @click="openSettings">打开设置窗口</el-button>
      </el-space>

      <el-divider />

      <h3>录制</h3>
      <el-space>
        <el-button
          :type="recording ? 'danger' : 'success'"
          @click="recording ? stopSession() : startSession()"
        >
          {{ recording ? "停止录制" : "开始录制" }}
        </el-button>
        <el-button @click="refreshSessions">刷新列表</el-button>
      </el-space>

      <el-table
        :data="sessions"
        size="small"
        style="margin-top: 12px"
        empty-text="暂无录制"
      >
        <el-table-column label="开始时间" width="200">
          <template #default="{ row }">{{ fmt(row.startedAt) }}</template>
        </el-table-column>
        <el-table-column label="时长" width="80">
          <template #default="{ row }">{{ duration(row) }}</template>
        </el-table-column>
        <el-table-column label="操作">
          <template #default="{ row }">
            <el-button size="small" type="primary" @click="openPlayer(row.id)">
              回放
            </el-button>
            <el-button size="small" type="danger" @click="deleteSession(row.id)">
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </main>
</template>

<style scoped>
.page {
  padding: 40px 16px;
}

.box {
  width: 100%;
}

h2 {
  margin: 0;
}

h3 {
  margin: 0 0 8px;
}

.desc {
  color: var(--el-text-color-secondary);
  margin: 0 0 16px;
}

.result {
  margin-top: 16px;
  color: var(--el-color-primary);
  font-weight: 600;
}
</style>
