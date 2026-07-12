<script setup lang="ts">
const form = reactive({
  theme: "light",
  autoStart: false,
});

function save() {
  ElMessage.success("设置已保存");
}
</script>

<template>
  <main class="page">
    <el-card class="box" shadow="hover">
      <template #header>
        <h2>设置</h2>
      </template>

      <el-form label-width="120px">
        <el-form-item label="主题">
          <el-select v-model="form.theme" style="width: 200px">
            <el-option label="浅色" value="light" />
            <el-option label="深色" value="dark" />
          </el-select>
        </el-form-item>
        <el-form-item label="开机自启">
          <el-switch v-model="form.autoStart" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="save">保存</el-button>
        </el-form-item>
      </el-form>
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
</style>
