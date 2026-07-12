<script setup lang="ts">
import { useRoute } from "vue-router";

const route = useRoute();
const id = route.params.id as string;
</script>

<template>
  <main class="page">
    <el-card class="box" shadow="hover">
      <template #header>
        <h2>播放器 - {{ id }}</h2>
      </template>

      <el-empty description="在此接入 rrweb 播放逻辑" />
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
  max-width: 720px;
}

h2 {
  margin: 0;
}
</style>
