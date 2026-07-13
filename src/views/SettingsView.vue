<script setup lang="ts">
const form = reactive({
  theme: "dark",
  autoStart: false,
  // 采集（P2 落地生效）
  captureErrors: true,
  captureConsole: true,
  captureNetwork: true,
  captureNetBody: false,
  // 接收（P4 落地生效）
  serverEnabled: false,
  serverPort: 1421,
  serverToken: "",
  // 保留
  retainMax: 50,
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
      <p class="settings-sub">采集与接收项在对应阶段（P2 / P4）落地后生效。</p>
    </header>

    <el-form label-width="96px" class="form">
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
        <div class="group-label eyebrow">采集</div>
        <el-form-item label="错误">
          <el-switch v-model="form.captureErrors" />
          <span class="field-hint">window.onerror / unhandledrejection</span>
        </el-form-item>
        <el-form-item label="Console">
          <el-switch v-model="form.captureConsole" />
          <span class="field-hint">log / warn / error / info / debug</span>
        </el-form-item>
        <el-form-item label="网络">
          <el-switch v-model="form.captureNetwork" />
          <span class="field-hint">fetch / XMLHttpRequest</span>
        </el-form-item>
        <el-form-item label="请求体">
          <el-switch v-model="form.captureNetBody" />
          <span class="field-hint">默认关 · 含 PII 风险</span>
        </el-form-item>
      </div>

      <div class="field-group">
        <div class="group-label eyebrow">接收</div>
        <el-form-item label="HTTP 服务">
          <el-switch v-model="form.serverEnabled" />
          <span class="field-hint">监听 127.0.0.1 · 接收外部上报</span>
        </el-form-item>
        <el-form-item label="端口">
          <el-input-number
            v-model="form.serverPort"
            :min="1024"
            :max="65535"
            controls-position="right"
            style="width: 140px"
          />
        </el-form-item>
        <el-form-item label="鉴权 token">
          <el-input
            v-model="form.serverToken"
            placeholder="本地 token · 避免同机误投"
            style="width: 260px"
          />
        </el-form-item>
      </div>

      <div class="field-group">
        <div class="group-label eyebrow">保留</div>
        <el-form-item label="最大会话数">
          <el-input-number
            v-model="form.retainMax"
            :min="1"
            :max="9999"
            controls-position="right"
            style="width: 140px"
          />
          <span class="field-hint">超出按时间倒序淘汰</span>
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
.settings-sub {
  margin: 6px 0 0;
  font-size: var(--fs-xs);
  color: var(--ash-deep);
}
.form {
  flex: 1;
  overflow: auto;
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
