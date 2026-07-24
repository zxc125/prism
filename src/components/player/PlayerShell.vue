<script setup lang="ts">
import "rrweb/dist/style.css";
import { ref, provide, onMounted } from "vue";
import { usePlayer } from "../../composables/usePlayer";
import { useAnnotations } from "../../composables/useAnnotations";
import { PLAYER_CTX, type PlayerCtx } from "./context";
import ReplayGrid from "./ReplayGrid.vue";
import Timeline from "./Timeline.vue";
import DiagnosisPanel from "./DiagnosisPanel.vue";

/** /s/:id 容器：面包屑 + 回放网格 + 诊断侧栏 + 传输控制条。
 *  创建 usePlayer / useAnnotations 并 provide 给子组件。 */

const props = defineProps<{ id: string }>();

const player = usePlayer(props.id);
const annos = useAnnotations(props.id);

const speeds = [0.5, 1, 2, 4];
const diagOpen = ref(true);

provide<PlayerCtx>(PLAYER_CTX, { sessionId: props.id, player, annos });

function fmt(ms: number) {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  const r = s % 60;
  const p = (n: number) => n.toString().padStart(2, "0");
  return `${p(m)}:${p(r)}`;
}

onMounted(async () => {
  try {
    await player.load();
  } catch (e) {
    ElMessage.error(`加载录制失败: ${e}`);
  }
  void annos.load();
});
</script>

<template>
  <main class="player" :class="{ 'diag-collapsed': !diagOpen }">
    <!-- 面包屑 + 会话元信息 -->
    <header class="bar">
      <div class="bar-crumb">
        <RouterLink to="/" class="crumb-link">会话</RouterLink>
        <span class="crumb-sep">/</span>
        <span class="crumb-current mono">{{ id }}</span>
      </div>
      <div class="bar-meta mono">
        <span>{{ player.timeline.value.labels.length }} 窗口</span>
        <span class="dot-sep">·</span>
        <span>{{ fmt(player.totalTime.value) }}</span>
        <template v-if="player.errorMarks.value.length">
          <span class="dot-sep">·</span>
          <span class="bar-err">⚠{{ player.errorMarks.value.length }}</span>
        </template>
      </div>
      <button
        class="bar-toggle mono"
        :class="{ 'is-on': diagOpen }"
        @click="diagOpen = !diagOpen"
      >
        诊断
      </button>
    </header>

    <div v-if="!player.ready.value" v-loading="true" class="loading" />

    <template v-else>
      <div class="body">
        <ReplayGrid />
        <DiagnosisPanel v-if="diagOpen" />
      </div>

      <footer class="transport">
        <button
          class="transport-play"
          :class="{ 'is-playing': player.playing.value }"
          :aria-label="player.playing.value ? '暂停' : '播放'"
          @click="player.playing.value ? player.pause() : player.play()"
        >
          <svg v-if="!player.playing.value" viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
            <path d="M8 5v14l11-7z" />
          </svg>
          <svg v-else viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
            <rect x="6" y="5" width="4" height="14" rx="1" />
            <rect x="14" y="5" width="4" height="14" rx="1" />
          </svg>
        </button>

        <Timeline />

        <span class="timecode mono">
          {{ fmt(player.currentTime.value) }}<span class="tc-sep">/</span>{{ fmt(player.totalTime.value) }}
        </span>

        <el-select
          :model-value="player.speed.value"
          class="speed"
          @update:model-value="(v) => player.setSpeed(v as number)"
        >
          <el-option v-for="s in speeds" :key="s" :label="s + 'x'" :value="s" />
        </el-select>

        <span class="sep" />

        <label class="follow">
          <el-switch
            :model-value="player.autoFollow.value"
            @update:model-value="(v) => player.setAutoFollow(v as boolean)"
          />
          <span>跟随焦点</span>
        </label>
      </footer>
    </template>
  </main>
</template>

<style scoped>
.player {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--ink);
}

/* ---- top bar ---- */
.bar {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 10px 16px;
  border-bottom: 1px solid var(--hair);
  background: var(--ink-2);
}
.bar-crumb {
  display: flex;
  align-items: center;
  gap: 8px;
}
.crumb-link {
  color: var(--ash);
  text-decoration: none;
  font-size: var(--fs-sm);
  transition: color 0.12s;
}
.crumb-link:hover { color: var(--bone-dim); }
.crumb-sep { color: var(--ash-deep); font-size: var(--fs-sm); }
.crumb-current {
  color: var(--bone);
  font-size: var(--fs-sm);
  letter-spacing: 0.04em;
}
.bar-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--ash);
  font-size: var(--fs-xs);
  margin-left: auto;
}
.bar-err { color: var(--oxblood-soft); }
.dot-sep { color: var(--ash-deep); }
.bar-toggle {
  appearance: none;
  border: 1px solid var(--hair);
  background: transparent;
  color: var(--ash);
  font-size: var(--fs-xs);
  padding: 4px 11px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  letter-spacing: 0.06em;
  transition: color 0.12s, border-color 0.12s, background 0.12s;
}
.bar-toggle:hover { color: var(--bone-dim); border-color: var(--ash-deep); }
.bar-toggle.is-on {
  color: var(--ink);
  background: var(--amber);
  border-color: var(--amber);
}

.loading { flex: 1; }

/* ---- body ---- */
.body {
  flex: 1;
  display: grid;
  grid-template-columns: minmax(0, 1fr) 340px;
  min-height: 0;
}
.player.diag-collapsed .body {
  grid-template-columns: minmax(0, 1fr);
}

/* ---- transport ---- */
.transport {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 12px 16px;
  border-top: 1px solid var(--hair);
  background: var(--slate);
}
.transport-play {
  width: 38px;
  height: 38px;
  border-radius: 50%;
  border: 1px solid var(--amber);
  background: var(--amber);
  color: var(--ink);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  flex-shrink: 0;
  transition: background 0.15s, transform 0.1s, box-shadow 0.15s;
  box-shadow: 0 0 12px var(--amber-glow);
}
.transport-play:hover {
  background: var(--amber-soft);
  box-shadow: 0 0 16px var(--amber-glow);
}
.transport-play:active { transform: scale(0.94); }
.transport-play.is-playing {
  background: transparent;
  color: var(--amber);
  box-shadow: 0 0 0 1px var(--amber) inset;
}
.timecode {
  font-size: var(--fs-sm);
  color: var(--bone);
  white-space: nowrap;
  letter-spacing: 0.06em;
}
.tc-sep { color: var(--ash-deep); margin: 0 4px; }
.speed { width: 84px; }
.sep { width: 1px; height: 22px; background: var(--hair); }
.follow {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: var(--fs-xs);
  color: var(--ash);
  cursor: pointer;
  white-space: nowrap;
}
.follow:hover { color: var(--bone-dim); }
</style>

<style>
/* tile 元素由 usePlayer 用 document.createElement 动态创建，不带 scoped 的
   data-v 属性，故这些规则放在非 scoped 块里；以 .grid 限定作用域避免泄漏。
   min-width:0 + minmax(0,...) 列模板防止 rrweb 原始宽度撑爆网格列。 */
.grid .tile-slot {
  display: flex;
  flex-direction: column;
  background: var(--slate);
  border: 1px solid var(--hair);
  border-radius: var(--radius);
  overflow: hidden;
  min-width: 0;
  min-height: 0;
}
.grid .tile-slot.is-main {
  grid-column: 1;
  grid-row: 1 / -1;
  border-color: var(--amber);
  box-shadow: 0 0 0 1px var(--amber-deep) inset, 0 0 28px var(--amber-glow);
}
.grid .tile-slot:not(.is-main) {
  grid-column: 2;
}
.grid .tile-slot.is-empty {
  background: repeating-linear-gradient(
    135deg,
    var(--slate) 0 10px,
    #11141A 10px 20px
  );
  border-style: dashed;
  border-color: var(--hair-soft);
}
.grid .tile-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 10px;
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
  letter-spacing: 0.06em;
  color: var(--bone-dim);
  background: var(--slate-2);
  border-bottom: 1px solid var(--hair);
  cursor: pointer;
  user-select: none;
}
.grid .tile-header::before {
  content: "";
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--lane-color, var(--ash));
  flex-shrink: 0;
}
.grid .tile-slot.is-main .tile-header {
  color: var(--bone);
  background: var(--amber-tint);
}
.grid .tile-slot.is-main .tile-header::after {
  content: "主";
  margin-left: auto;
  font-size: 10px;
  letter-spacing: 0.1em;
  color: var(--ink);
  background: var(--amber);
  padding: 1px 6px;
  border-radius: 2px;
  font-weight: 600;
}
.grid .tile-placeholder {
  display: none;
  flex: 1;
  align-items: center;
  justify-content: center;
  color: var(--ash-deep);
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
  letter-spacing: 0.1em;
}
.grid .tile-slot.is-empty .tile-placeholder {
  display: flex;
}
.grid .tile-root {
  flex: 1;
  overflow: hidden;
  background: #fff;
}
/* .replayer-wrapper 的 width/height/transform 由 usePlayer.fitSegment 按录制
   视口尺寸等比缩放设置，此处不覆写。 */
</style>
