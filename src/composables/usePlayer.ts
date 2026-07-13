import { Replayer, type eventWithTime } from "rrweb";
import { invoke } from "@tauri-apps/api/core";
import { ref } from "vue";

type RREvent = eventWithTime;
type WindowEvt = {
  type: "shown" | "hidden" | "focus";
  label: string;
  segmentId?: string;
  t: number;
};
type SessionData = {
  session: { id: string; startedAt: number; endedAt?: number };
  windows: WindowEvt[];
  segments: Record<string, RREvent[]>;
};

type SegmentInfo = {
  segmentId: string;
  label: string;
  events: RREvent[];
  firstTs: number;
  lastTs: number;
  shownAt: number;
  hiddenAt: number | null; // null = 持续到会话结束
  replayer: Replayer | null;
  container: HTMLDivElement | null;
};

type TimelineBand = { labelIdx: number; start: number; end: number };

// 各 Replayer 独立 RAF 与主时钟的允许漂移，超过则强制 re-seek 对齐，防止长录制漂移
const DRIFT_THRESHOLD = 120; // ms

// 窗口轨道色板：filmic、在深色面上可辨；供时间轴色带与回放磁贴头共用
export const LANE_COLORS = [
  "#E8A33D",
  "#C75B5B",
  "#5BC0BE",
  "#B084CC",
  "#8AB36A",
  "#D98E50",
  "#6FA8DC",
];

/**
 * 回放一个录制会话：按 windows.jsonl 的 shown/hidden 区间，
 * 在主时间轴上驱动各 segment 的 Replayer；每个 label 占一个稳定槽位，
 * 同 label 多段按时切换显隐，窗口 show/hide 不触发 reflow。
 */
export function usePlayer(sessionId: string) {
  const ready = ref(false);
  const playing = ref(false);
  const currentTime = ref(0); // 相对会话起点 ms
  const totalTime = ref(0);
  const speed = ref(1);

  let segs: SegmentInfo[] = [];
  // label -> 稳定槽位容器：每个窗口一个固定位置，show/hide 不 reflow
  let slots = new Map<string, HTMLDivElement>();
  let startTs = 0;
  let gridEl: HTMLElement | null = null;
  let tickTimer: number | null = null;
  let lastTick = 0;
  let active = new Set<string>();
  // distinct label 按首次 shown 顺序：稳定槽位索引 + 主窗口兜底
  let labelOrder: string[] = [];
  // focus 时间线 {t,label} 升序，驱动自动主窗口
  let focusTimeline: { t: number; label: string }[] = [];
  // 主窗口
  const mainLabel = ref<string | null>(null);
  const autoFollow = ref(true); // true=跟随 focus 时间线；false=锁定手动选择
  let manualMain: string | null = null;
  // 布局缓存，避免每帧重设 grid 模板
  let lastMain: string | null = null;
  let lastSideCount = -1;
  // 时间轴结构（供进度条色带 + focus 标记渲染）：load 后填充
  const timeline = ref<{
    labels: string[];
    bands: TimelineBand[];
    focusMarks: number[];
  }>({ labels: [], bands: [], focusMarks: [] });

  async function load() {
    const data = await invoke<SessionData>("read_session", { id: sessionId });
    segs = [];
    slots = new Map();
    for (const [segmentId, events] of Object.entries(data.segments)) {
      if (!events.length) continue;
      const shown = data.windows.find(
        (w) => w.type === "shown" && w.segmentId === segmentId,
      );
      const hidden = data.windows.find(
        (w) => w.type === "hidden" && w.segmentId === segmentId,
      );
      segs.push({
        segmentId,
        label: shown?.label ?? segmentId,
        events,
        firstTs: events[0].timestamp,
        lastTs: events[events.length - 1].timestamp,
        shownAt: shown?.t ?? events[0].timestamp,
        hiddenAt: hidden?.t ?? null,
        replayer: null,
        container: null,
      });
    }
    // 按首次 shown 时间排序，使 label 首现顺序 = 显示顺序（稳定槽位索引）
    segs.sort((a, b) => a.shownAt - b.shownAt);
    labelOrder = [];
    const seenLabel = new Set<string>();
    for (const seg of segs) {
      if (!seenLabel.has(seg.label)) {
        seenLabel.add(seg.label);
        labelOrder.push(seg.label);
      }
    }
    focusTimeline = data.windows
      .filter((w) => w.type === "focus")
      .map((w) => ({ t: w.t, label: w.label }))
      .sort((a, b) => a.t - b.t);
    mainLabel.value = null;
    manualMain = null;
    autoFollow.value = true;
    lastMain = null;
    lastSideCount = -1;
    startTs = data.session.startedAt;
    const end =
      data.session.endedAt ??
      Math.max(startTs, ...segs.map((s) => s.hiddenAt ?? s.lastTs));
    totalTime.value = Math.max(0, end - startTs);
    currentTime.value = 0;
    const bands: TimelineBand[] = segs.map((seg) => ({
      labelIdx: labelOrder.indexOf(seg.label),
      start: Math.max(0, seg.shownAt - startTs),
      end: Math.min(totalTime.value, (seg.hiddenAt ?? seg.lastTs) - startTs),
    }));
    const focusMarks = focusTimeline
      .map((f) => f.t - startTs)
      .filter((t) => t >= 0 && t <= totalTime.value);
    timeline.value = { labels: labelOrder.slice(), bands, focusMarks };
    ready.value = true;
  }

  function attachGrid(el: HTMLElement) {
    gridEl = el;
    for (const label of labelOrder) {
      const slot = document.createElement("div");
      slot.className = "tile-slot";
      slot.dataset.label = label;
      slot.style.setProperty(
        "--lane-color",
        LANE_COLORS[labelOrder.indexOf(label) % LANE_COLORS.length],
      );

      const header = document.createElement("div");
      header.className = "tile-header";
      header.textContent = label;
      header.title = "点击设为主窗口";
      header.addEventListener("click", () => selectMain(label));
      slot.appendChild(header);

      const placeholder = document.createElement("div");
      placeholder.className = "tile-placeholder";
      placeholder.textContent = "已隐藏";
      slot.appendChild(placeholder);

      gridEl.appendChild(slot);
      slots.set(label, slot);
    }
    // 每个 segment 的 replayer 容器挂到对应 label 的 slot 内；同 label 多段共存，按时只显一个
    for (const seg of segs) {
      if (seg.container) continue;
      const slot = slots.get(seg.label);
      if (!slot) continue;
      const root = document.createElement("div");
      root.className = "tile-root";
      root.style.display = "none";
      slot.appendChild(root);

      seg.container = root;
      seg.replayer = new Replayer(seg.events, {
        root,
        speed: speed.value,
      });
      seg.replayer.pause(0);
    }
    syncVisibility(startTs + currentTime.value);
  }

  function activeAt(absT: number): SegmentInfo[] {
    return segs.filter(
      (s) => absT >= s.shownAt && (s.hiddenAt === null || absT < s.hiddenAt),
    );
  }

  function syncVisibility(absT: number) {
    // 每个 label 当前活跃的段（至多一个）
    const activeByLabel = new Map<string, SegmentInfo>();
    for (const seg of segs) {
      if (absT >= seg.shownAt && (seg.hiddenAt === null || absT < seg.hiddenAt)) {
        activeByLabel.set(seg.label, seg);
      }
    }
    for (const [label, slot] of slots) {
      const act = activeByLabel.get(label);
      slot.classList.toggle("is-empty", !act);
      for (const seg of segs) {
        if (seg.label !== label || !seg.container) continue;
        seg.container.style.display = seg === act ? "" : "none";
      }
    }
    active = new Set(
      [...activeByLabel.values()].map((s) => s.segmentId),
    );
    mainLabel.value = computeMainLabel(absT, activeByLabel);
    applyLayout();
  }

  /** 主窗口选择：手动锁定 > 自动(focus 时间线) > 最近 shown 活跃窗口 > 兜底 */
  function computeMainLabel(
    absT: number,
    activeByLabel: Map<string, SegmentInfo>,
  ): string | null {
    if (segs.length === 0) return null;
    // 手动模式：锁定所选 label（即使当前隐藏，主区显示占位）
    if (!autoFollow.value && manualMain) return manualMain;
    // 自动：最后一条 focus（t<=absT）且该 label 当前活跃
    let picked: string | null = null;
    for (const f of focusTimeline) {
      if (f.t > absT) break;
      if (activeByLabel.has(f.label)) picked = f.label;
    }
    if (picked) return picked;
    // 兜底 1：最近 shown 的活跃窗口
    let latest: SegmentInfo | null = null;
    for (const seg of activeByLabel.values()) {
      if (!latest || seg.shownAt > latest.shownAt) latest = seg;
    }
    if (latest) return latest.label;
    // 兜底 2：无活跃窗口，保留上次主窗口或首个 label（主区占位）
    return mainLabel.value ?? labelOrder[0] ?? null;
  }

  /** spotlight 布局：主槽占大格，其余占侧槽。仅在变化时改 DOM/style，避免每帧抖动 */
  function applyLayout() {
    if (!gridEl) return;
    const sideCount = Math.max(0, labelOrder.length - 1);
    if (mainLabel.value !== lastMain) {
      for (const [label, slot] of slots) {
        slot.classList.toggle("is-main", label === mainLabel.value);
      }
      lastMain = mainLabel.value;
    }
    if (sideCount !== lastSideCount) {
      if (sideCount === 0) {
        gridEl.style.gridTemplateColumns = "1fr";
        gridEl.style.gridTemplateRows = "1fr";
      } else {
        gridEl.style.gridTemplateColumns = "2fr 1fr";
        gridEl.style.gridTemplateRows = `repeat(${sideCount}, minmax(120px, 1fr))`;
      }
      lastSideCount = sideCount;
    }
  }

  function selectMain(label: string) {
    manualMain = label;
    autoFollow.value = false;
    syncVisibility(startTs + currentTime.value);
  }

  function setAutoFollow(v: boolean) {
    if (!v) {
      manualMain = mainLabel.value; // 切到手动：锁定当前主窗口
    } else {
      manualMain = null;
    }
    autoFollow.value = v;
    syncVisibility(startTs + currentTime.value);
  }

  function play() {
    if (!ready.value || playing.value) return;
    playing.value = true;
    const absT = startTs + currentTime.value;
    for (const seg of activeAt(absT)) {
      seg.replayer?.play(Math.max(0, absT - seg.firstTs));
    }
    lastTick = performance.now();
    tickTimer = window.setInterval(tick, 50);
  }

  function tick() {
    const now = performance.now();
    const dt = now - lastTick;
    lastTick = now;
    currentTime.value += dt * speed.value;
    if (currentTime.value >= totalTime.value) {
      currentTime.value = totalTime.value;
      pause();
      return;
    }
    const absT = startTs + currentTime.value;
    const activeSegs = activeAt(absT);
    const next = new Set(activeSegs.map((s) => s.segmentId));
    for (const seg of activeSegs) {
      if (!active.has(seg.segmentId)) {
        // 新激活的段：从主时钟位置开始播放
        seg.replayer?.play(Math.max(0, absT - seg.firstTs));
      } else if (seg.replayer) {
        // 已在播放的段：与主时钟对齐，漂移超阈值则 re-seek 拉回，防止长录制漂移
        const expect = absT - seg.firstTs;
        const actual = seg.replayer.getCurrentTime();
        if (Math.abs(actual - expect) > DRIFT_THRESHOLD) {
          seg.replayer.play(Math.max(0, expect));
        }
      }
    }
    // 已失活的段：暂停
    for (const seg of segs) {
      if (active.has(seg.segmentId) && !next.has(seg.segmentId)) {
        seg.replayer?.pause();
      }
    }
    syncVisibility(absT);
  }

  function pause() {
    playing.value = false;
    if (tickTimer != null) {
      clearInterval(tickTimer);
      tickTimer = null;
    }
    for (const seg of segs) {
      seg.replayer?.pause();
    }
  }

  function seek(ms: number) {
    currentTime.value = ms;
    const absT = startTs + ms;
    syncVisibility(absT);
    for (const seg of activeAt(absT)) {
      seg.replayer?.pause(Math.max(0, absT - seg.firstTs));
    }
  }

  function setSpeed(s: number) {
    speed.value = s;
    for (const seg of segs) {
      seg.replayer?.setConfig({ speed: s });
    }
  }

  function destroy() {
    pause();
    for (const seg of segs) {
      seg.replayer?.destroy();
    }
    segs = [];
    slots = new Map();
    labelOrder = [];
    focusTimeline = [];
    mainLabel.value = null;
    manualMain = null;
    lastMain = null;
    lastSideCount = -1;
    timeline.value = { labels: [], bands: [], focusMarks: [] };
  }

  return {
    ready,
    playing,
    currentTime,
    totalTime,
    speed,
    mainLabel,
    autoFollow,
    timeline,
    load,
    attachGrid,
    play,
    pause,
    seek,
    setSpeed,
    selectMain,
    setAutoFollow,
    destroy,
  };
}
