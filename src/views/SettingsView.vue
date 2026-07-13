<script setup lang="ts">
const form = reactive({
  theme: "dark",
  autoStart: false,
});

function save() {
  ElMessage.success("设置已保存");
}
</script>

<template>
  <main class="settings">
    <header class="settings-head">
      <span class="eyebrow">偏好</span>
      <h1 class="settings-title">设置</h1>
    </header>

    <el-form label-width="88px" class="form">
      <div class="field-group">
        <div class="group-label eyebrow">外观</div>
        <el-form-item label="主题">
          <el-select v-model="form.theme" style="width: 200px">
            <el-option label="深色 · 控制台" value="dark" />
            <el-option label="浅色" value="light" />
          </el-select>
        </el-form-item>
      </div>

      <div class="field-group">
        <div class="group-label eyebrow">系统</div>
        <el-form-item label="开机自启">
          <el-switch v-model="form.autoStart" />
          <span class="field-hint">登录后自动启动并进入待机</span>
        </el-form-item>
      </div>
    </el-form>

    <footer class="settings-foot">
      <el-button type="primary" @click="save">保存设置</el-button>
    </footer>
  </main>
</template>

<style scoped>
.settings {
  display: flex;
  flex-direction: column;
  height: 100vh;
  padding: 22px 24px;
  background: var(--ink);
}
.settings-head {
  margin-bottom: 20px;
}
.settings-title {
  margin: 4px 0 0;
  font-size: var(--fs-xl);
  font-weight: 600;
  color: var(--bone);
  letter-spacing: -0.01em;
}
.form {
  flex: 1;
}
.field-group {
  margin-bottom: 24px;
}
.group-label {
  padding-bottom: 8px;
  margin-bottom: 4px;
  border-bottom: 1px solid var(--hair-soft);
}
.field-hint {
  color: var(--ash-deep);
  font-size: var(--fs-xs);
  margin-left: 12px;
}
.settings-foot {
  padding-top: 16px;
  border-top: 1px solid var(--hair-soft);
}
</style>
