<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";

const name = ref("");
const greetMsg = ref("");

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

async function openSettings() {
  // 单实例：label 固定为 settings，已存在则聚焦
  try {
    await invoke("open_window", { route: "/settings" });
  } catch (e) {
    ElMessage.error(`打开失败: ${e}`);
  }
}

async function openPlayer() {
  // 多实例：随机 id 生成不同 label，可同时开多个
  const id = Math.random().toString(36).slice(2, 8);
  try {
    await invoke("open_window", { route: `/player/${id}` });
  } catch (e) {
    ElMessage.error(`打开失败: ${e}`);
  }
}
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
        <el-button type="success" @click="openPlayer">打开播放器窗口</el-button>
      </el-space>
    </el-card>
  </main>
</template>

<style scoped>
.page {
  display: flex;
  justify-content: center;
  padding: 40px 16px;
}

.box {
  width: 100%;
  max-width: 560px;
}

h2 {
  margin: 0;
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
