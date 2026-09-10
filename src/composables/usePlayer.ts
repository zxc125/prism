import { Replayer, type eventWithTime } from "rrweb";
import { ref, computed } from "vue";
import { getBackend, type SessionMetaPayload } from "./backend";

type RREvent = eventWithTime;
type WindowEvt = {
  type: "shown" | "hidden" | "focus";
  label: string;
  segmentId?: string;
  t: number;
};

/** 段加载状态（P19 D1/D3）：cold=只有索引，loading=取数中，ready=已挂载 Replayer。 */
type SegState = "cold" | "loading" | "ready" | "error";

type SegmentInfo = {
  segmentId: string;
  label: string;
  events: RREvent[] | null; // cold 时为 null（未按需取回）
  state: SegState;
  firstTs: number;
  lastTs: number;
  shownAt: number;
  hiddenAt: number | null; // null = 持续到会话结束
  replayer: Replayer | null;
  container: HTMLDivElement | null;
  origW: number; // 录制时视口宽（Meta 事件），用于等比缩放
  origH: number;
  /** 诊断信号是否已并入全量流：段被回收后重载不得重复追加（P19 回归修复）。 */
  signalsCollected: boolean;
  /** 加载失败重试计数与下次可重试时刻（退避，见 maybeRetry）。 */
  retries: number;
  retryAt: number;
};


// 预取窗口：播放头进入 shownAt - PRELOAD_MS 就拉该段（P19 D3）
const PRELOAD_MS = 3000;
// 同时存活的 Replayer 上限；超出后淘汰离播放头最远、且不在预取窗口内的段（P19 D3）
const MAX_LIVE_SEGMENTS = 6;
// 段加载失败的重试退避（P19 O5）
const RETRY_DELAY_MS = 3000;
const MAX_RETRIES = 3;

type TimelineBand = { labelIdx: number; start: number; end: number };

// 诊断信号（交错事件模型 type:6 的回放侧投影）。
// text 在 load() 时一次算好（P19 D7）：避免模板每次重渲染重复 JSON.stringify。
type SignalBase = { t: number; text: string };
export type Signal =
  | (SignalBase & { plugin: "error"; payload: { message: string; stack?: string; kind: string } })
  | (SignalBase & { plugin: "console"; payload: { level: string; args: unknown[] } })
  | (SignalBase & { plugin: "network"; payload: { url: string; method: string; status: number; duration: number; kind: string } });

function formatArg(a: unknown): string {
  if (a === null) return "null";
  if (typeof a === "object") {
    try {
      return JSON.stringify(a);
    } catch {
      return String(a);
    }
  }
  return String(a);
}

/** 信号单行文本（load 时预计算，见 P19 D7）。 */
function signalText(plugin: string, payload: unknown): string {
  if (plugin === "console") {
    const p = payload as { args: unknown[] };
    return p.args.map(formatArg).join(" ");
  }
  if (plugin === "network") {
    const p = payload as {
      method: string;
      url: string;
      status: number;
      duration: number;
    };
    return `${p.method} ${p.url}  ${p.status}  ${p.duration}ms`;
  }
  return (payload as { message: string }).message;
}

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
  // 拖动进度条时的预览位置（P19 D6）：非 null 时只驱动显示，不做任何 Replayer 工作；
  // 松手（commitPreview）才真正 seek。seek() 自身会清空预览。
  const previewTime = ref<number | null>(null);
  const displayTime = computed(() => previewTime.value ?? currentTime.value);

  let segs: SegmentInfo[] = [];
  // label -> 稳定槽位容器：每个窗口一个固定位置，show/hide 不 reflow
  let slots = new Map<string, HTMLDivElement>();
  // label -> 槽位占位元素（"载入中"/"已隐藏"），避免每 tick querySelector
  let placeholders = new Map<string, HTMLElement>();
  let startTs = 0;
  let gridEl: HTMLElement | null = null;
  let tickTimer: number | null = null;
  let lastTick = 0;
  let ro: ResizeObserver | null = null; // 监听 tile-root 尺寸变化，重算等比缩放
  let active = new Set<string>();
  // 上次活跃映射 label -> segmentId：diff 用，无变化时完全跳过 DOM 写（P19 D8）
  let lastActiveByLabel = new Map<string, string>();
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

  // 诊断信号流：从各 segment 的 type:6 事件收集（P2 真实采集；旧会话无则空）
  const signals = ref<Signal[]>([]);
  const errorMarks = computed(() =>
    signals.value.filter((s) => s.plugin === "error").map((s) => s.t),
  );

  async function load() {
    // P19 D1：首屏只取元信息 + 段索引（O(段数)），段内事件在进入预取窗口时才拉。
    // 时间轴 bands / 槽位 / 总时长全部来自索引，不需要事件正文。
    const data = (await getBackend().readSessionMeta(
      sessionId,
    )) as unknown as SessionMetaPayload;
    segs = [];
    slots = new Map();
    const shownBySeg = new Map<string, WindowEvt>();
    const hiddenBySeg = new Map<string, WindowEvt>();
    for (const w of data.windows) {
      if (!w.segmentId) continue;
      if (w.type === "shown") shownBySeg.set(w.segmentId, w);
      else if (w.type === "hidden") hiddenBySeg.set(w.segmentId, w);
    }
    for (const entry of data.segments) {
      // 空段（无首行时间戳）跳过——与旧实现 `!events.length` 跳过等价
      if (entry.firstTs == null) continue;
      const shown = shownBySeg.get(entry.segmentId);
      const hidden = hiddenBySeg.get(entry.segmentId);
      segs.push({
        segmentId: entry.segmentId,
        label: shown?.label ?? entry.segmentId,
        events: null,
        state: "cold",
        firstTs: entry.firstTs,
        lastTs: entry.lastTs ?? entry.firstTs,
        shownAt: shown?.t ?? entry.firstTs,
        hiddenAt: hidden?.t ?? null,
        replayer: null,
        container: null,
        origW: entry.width ?? 0,
        origH: entry.height ?? 0,
        signalsCollected: false,
        retries: 0,
        retryAt: 0,
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

    // 诊断信号：首屏为空，后台一次性取回**全库** type:6（不依赖段是否加载），
    // 取回前由各段加载时增量兜底（见 collectSignals）
    signals.value = [];
    allSignalsLoaded = false;
    void loadAllSignals();

    ready.value = true;
  }

  /**
   * 全库诊断信号（P19 回归修复）：走只回 type:6 行的廉价读路径（约为全量事件的 1%），
   * 使信号流/错误标记/时间轴红点在首屏就是完整的，与门控式按需加载解耦。
   * 失败不影响回放——增量路径仍会随段加载补齐已访问部分。
   */
  async function loadAllSignals() {
    try {
      const events = (await getBackend().readSessionSignals(sessionId)) as RREvent[];
      if (destroyed) return;
      const sigs: Signal[] = [];
      for (const e of events) {
        if (e.type !== 6) continue;
        const d = e.data as { plugin?: string; payload?: unknown };
        if (!d.plugin) continue;
        sigs.push({
          t: e.timestamp - startTs,
          plugin: d.plugin as Signal["plugin"],
          payload: d.payload,
          text: signalText(d.plugin, d.payload),
        } as unknown as Signal);
      }
      sigs.sort((a, b) => a.t - b.t);
      signals.value = sigs;
      allSignalsLoaded = true;
    } catch (e) {
      console.error("[player] read_session_signals failed", e);
    }
  }

  /** 从已加载段中收集交错 type:6 诊断信号，并入全量信号流（按相对会话起点排序）。 */
  function collectSignals(seg: SegmentInfo) {
    // 全量信号已就位时不再增量并入；段被回收后会重载，故还需 per-段标志位防重复
    if (allSignalsLoaded) return;
    if (!seg.events || seg.signalsCollected) return;
    seg.signalsCollected = true;
    const add: Signal[] = [];
    for (const e of seg.events) {
      if (e.type !== 6) continue;
      const d = e.data as { plugin?: string; payload?: unknown };
      if (!d.plugin) continue;
      add.push({
        t: e.timestamp - startTs,
        plugin: d.plugin as Signal["plugin"],
        payload: d.payload,
        text: signalText(d.plugin, d.payload),
      } as unknown as Signal);
    }
    if (!add.length) return;
    signals.value = [...signals.value, ...add].sort((a, b) => a.t - b.t);
  }

  function attachGrid(el: HTMLElement) {
    gridEl = el;
    placeholders = new Map();
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
      placeholders.set(label, placeholder);

      gridEl.appendChild(slot);
      slots.set(label, slot);
    }
    // 不再 attach 期预建全部 Replayer（P19 D3）：旧实现在这里对每段 new Replayer + pause(0)，
    // 而 rrweb 的 pause(offset) 内部是 play(offset)+PAUSE——等于为每段白付一次「遍历全部
    // 事件 + 为未来事件建 castFn + 构建整棵 DOM」。改为进入预取窗口才拉事件并挂载。

    // 监听各 tile-root 尺寸变化（spotlight 切主、窗口缩放、段显隐）重算缩放
    ro?.disconnect();
    ro = new ResizeObserver(() => fitAll());
    syncVisibility(startTs + currentTime.value);
    ensureUpcoming(startTs + currentTime.value);
    fitAll();
    // 兜底：rrweb wrapper 若在下一帧才就绪，再 fit 一次
    requestAnimationFrame(() => fitAll());
  }

  /** 把已加载的段挂到对应槽位并建 Replayer，按当前时间定位（P19 D3）。 */
  function mountSegment(seg: SegmentInfo) {
    if (seg.replayer || !seg.events) return;
    const slot = slots.get(seg.label);
    if (!slot) return;
    const root = document.createElement("div");
    root.className = "tile-root";
    slot.appendChild(root);
    seg.container = root;
    seg.replayer = new Replayer(seg.events, { root, speed: speed.value });
    ro?.observe(root);

    const absT = startTs + currentTime.value;
    const offset = Math.max(0, absT - seg.firstTs);
    const isActive = absT >= seg.shownAt && (seg.hiddenAt === null || absT < seg.hiddenAt);
    root.style.display = isActive ? "" : "none";
    if (playing.value && isActive) {
      seg.replayer.setConfig({ speed: speed.value });
      seg.replayer.play(offset);
    } else {
      seg.replayer.pause(offset);
    }
    fitSegment(seg);
  }

  /** 卸载一段：销毁 Replayer、移除容器、释放事件数组（P19 D3 回收）。 */
  function destroySegment(seg: SegmentInfo) {
    seg.replayer?.destroy();
    seg.replayer = null;
    if (seg.container) {
      ro?.unobserve(seg.container);
      seg.container.remove();
      seg.container = null;
    }
    seg.events = null;
    seg.state = "cold";
  }

  /** 段是否已完全过去：优先用 hidden 事件；缺失时（外部导入会话常见）用索引里的
   *  lastTs。少了这个判据，被回收的旧段会在下一 tick 立刻被重新预取，形成
   *  load→mount→destroy 的持续抖动。 */
  function isPast(seg: SegmentInfo, absT: number): boolean {
    if (seg.hiddenAt !== null) return absT >= seg.hiddenAt;
    return absT > seg.lastTs;
  }

  /** 同时存活的 Replayer 超上限时，淘汰离播放头最远的段。 */
  function evictFar(absT: number) {
    const mounted = segs.filter((s) => s.replayer);
    if (mounted.length <= MAX_LIVE_SEGMENTS) return;
    // 保留集：当前活跃段 + 预取窗口内尚未过去的段（淘汰后者 = 下一 tick 立刻重载）
    const keep = new Set(activeAt(absT).map((s) => s.segmentId));
    for (const seg of segs) {
      if (!isPast(seg, absT) && seg.shownAt - absT <= PRELOAD_MS) {
        keep.add(seg.segmentId);
      }
    }
    const far = mounted
      .filter((s) => !keep.has(s.segmentId))
      .sort((a, b) => Math.abs(b.shownAt - absT) - Math.abs(a.shownAt - absT));
    let overflow = mounted.length - MAX_LIVE_SEGMENTS;
    for (const seg of far) {
      if (overflow <= 0) break;
      destroySegment(seg);
      overflow--;
    }
  }

  /** 失败段的退避重试（P19 O5）：一次性失败不该让该窗口整场空白。 */
  function maybeRetry(seg: SegmentInfo, absT: number) {
    if (seg.retries >= MAX_RETRIES) return;
    if (Date.now() < seg.retryAt) return;
    if (isPast(seg, absT)) return;
    seg.retries += 1;
    seg.state = "cold";
    void loadSegment(seg);
  }

  // 正在取数的段（防止同一段并发重复拉取）
  const pending = new Set<string>();
  let destroyed = false;
  // 全库信号是否已就位（就位后各段不再增量并入，避免重复）
  let allSignalsLoaded = false;

  /** 按需取回一段的事件并挂载（P19 D1/D3）。 */
  async function loadSegment(seg: SegmentInfo) {
    if (seg.state !== "cold" || pending.has(seg.segmentId)) return;
    pending.add(seg.segmentId);
    seg.state = "loading";
    try {
      const events = (await getBackend().readSegment(
        sessionId,
        seg.segmentId,
      )) as RREvent[];
      if (destroyed) return;
      seg.events = events;
      if (events.length) {
        seg.firstTs = events[0].timestamp;
        seg.lastTs = events[events.length - 1].timestamp;
      }
      seg.state = "ready";
      mountSegment(seg);
      collectSignals(seg);
      syncVisibility(startTs + currentTime.value);
      fitAll();
    } catch (e) {
      seg.state = "error";
      seg.retryAt = Date.now() + RETRY_DELAY_MS; // 退避后由 ensureUpcoming 重试
      console.error("[player] read_segment failed", seg.segmentId, e);
    } finally {
      pending.delete(seg.segmentId);
      syncVisibility(startTs + currentTime.value);
    }
  }

  /** 预取：把「已开始或将在 PRELOAD_MS 内开始」的段拉回来，并回收远处的段。 */
  function ensureUpcoming(absT: number) {
    for (const seg of segs) {
      if (seg.state === "cold") {
        if (isPast(seg, absT)) continue; // 已过去的不预取；seek 回去时由 seek 重新命中
        if (seg.shownAt - absT <= PRELOAD_MS) void loadSegment(seg);
      } else if (seg.state === "error") {
        maybeRetry(seg, absT);
      }
    }
    evictFar(absT);
  }


  /**
   * 每个 label 在当前时刻「正在显示」的段（至多一个）。
   *
   * 必须按 label 去重：段缺 `hidden` 事件时（外部应用导出的会话常见，实测目标会话
   * 27 shown / 0 hidden），同一 label 的全部历史段都满足 `absT >= shownAt` 且
   * `hiddenAt === null`。若这里返回全部命中段，tick 会把历史段当成「新激活」而每
   * 50ms 对每段调一次 `play()`——rrweb 的 `play(offset)` 在非 paused 态是一次全量
   * 重放，等于把 D4 想消灭的正反馈又请回来（实测 13 次/tick）。
   * 同 label 多段重叠时取 shownAt 最大者，与 `syncVisibility` 的覆盖语义一致。
   */
  function activeAt(absT: number): SegmentInfo[] {
    const byLabel = new Map<string, SegmentInfo>();
    for (const seg of segs) {
      if (absT < seg.shownAt) continue;
      if (seg.hiddenAt !== null && absT >= seg.hiddenAt) continue;
      const prev = byLabel.get(seg.label);
      if (!prev || seg.shownAt >= prev.shownAt) byLabel.set(seg.label, seg);
    }
    return [...byLabel.values()];
  }

  function syncVisibility(absT: number) {
    // 每个 label 当前活跃的段（至多一个）——与 activeAt 同源，避免两套判定漂移
    const activeByLabel = new Map<string, SegmentInfo>();
    for (const seg of activeAt(absT)) activeByLabel.set(seg.label, seg);
    // diff（P19 D8）：活跃集合没变就完全不碰 DOM——原先每 tick 都做
    // O(labels × segs) 的 style.display 写，是播放期的常驻开销
    let changed = activeByLabel.size !== lastActiveByLabel.size;
    if (!changed) {
      for (const [label, seg] of activeByLabel) {
        if (lastActiveByLabel.get(label) !== seg.segmentId) {
          changed = true;
          break;
        }
      }
    }
    if (changed) {
      for (const label of slots.keys()) {
        const act = activeByLabel.get(label);
        for (const seg of segs) {
          if (seg.label !== label || !seg.container) continue;
          seg.container.style.display = seg === act ? "" : "none";
        }
      }
      lastActiveByLabel = new Map(
        [...activeByLabel].map(([l, s]) => [l, s.segmentId]),
      );
    }
    // 占位/加载态：O(labels)，与上面的重活分开——段 state 变化不改变活跃映射，
    // 所以这段不能放进 changed 分支
    for (const [label, slot] of slots) {
      const act = activeByLabel.get(label);
      slot.classList.toggle("is-empty", act?.state !== "ready");
      const ph = placeholders.get(label);
      if (ph) {
        ph.textContent = act
          ? act.state === "error"
            ? "载入失败"
            : "载入中"
          : "已隐藏";
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
        gridEl.style.gridTemplateColumns = "minmax(0, 1fr)";
        gridEl.style.gridTemplateRows = "minmax(0, 1fr)";
      } else {
        gridEl.style.gridTemplateColumns = "minmax(0, 2fr) minmax(0, 1fr)";
        gridEl.style.gridTemplateRows = `repeat(${sideCount}, minmax(0, 1fr))`;
      }
      lastSideCount = sideCount;
    }
  }

  /**
   * 等比缩放：rrweb Replayer 按录制时原始像素渲染，需把 .replayer-wrapper
   * 固定为原始尺寸，再 transform: scale() fit-contain 到 tile-root，并居中。
   * spotlight 主区放大/窗口尺寸变化时由 ResizeObserver 触发重算。
   */
  function fitSegment(seg: SegmentInfo) {
    const root = seg.container;
    if (!root || !seg.origW || !seg.origH) return;
    const rw = root.clientWidth;
    const rh = root.clientHeight;
    if (!rw || !rh) return; // display:none 或未布局
    const s = Math.min(rw / seg.origW, rh / seg.origH);
    const sw = seg.origW * s;
    const sh = seg.origH * s;
    const ox = (rw - sw) / 2;
    const oy = (rh - sh) / 2;
    const wrapper = root.querySelector<HTMLElement>(".replayer-wrapper");
    if (!wrapper) return;
    wrapper.style.width = `${seg.origW}px`;
    wrapper.style.height = `${seg.origH}px`;
    wrapper.style.transformOrigin = "top left";
    wrapper.style.transform = `translate(${ox}px, ${oy}px) scale(${s})`;
  }

  function fitAll() {
    for (const seg of segs) {
      if (seg.container && seg.container.style.display !== "none") {
        fitSegment(seg);
      }
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
    previewTime.value = null; // 播放优先于拖动预览
    playing.value = true;
    const absT = startTs + currentTime.value;
    for (const seg of activeAt(absT)) {
      seg.replayer?.setConfig({ speed: speed.value });
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
        // 新激活的段：同步当前倍速 + 从主时钟位置开始播放。
        // 不做漂移纠偏（P19 D4）：rrweb 的 play(offset) 在非 paused 态会先 PAUSE 再 PLAY，
        // 等于一次 O(段首→offset) 的全量同步重放——在大会话上是「越纠越卡」的正反馈。
        // 对齐只发生在段激活 / 用户 seek / 暂停 / 切倍速四个时点，段边界天然是同步点。
        seg.replayer?.setConfig({ speed: speed.value });
        seg.replayer?.play(Math.max(0, absT - seg.firstTs));
      }
    }
    // 已失活的段：暂停
    for (const seg of segs) {
      if (active.has(seg.segmentId) && !next.has(seg.segmentId)) {
        seg.replayer?.pause();
      }
    }
    ensureUpcoming(absT);
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
    previewTime.value = null; // 即时 seek 立即退出预览态
    currentTime.value = ms;
    const absT = startTs + ms;
    syncVisibility(absT);
    ensureUpcoming(absT);
    for (const seg of activeAt(absT)) {
      const offset = Math.max(0, absT - seg.firstTs);
      // 播放中 seek：定位后立即续播，否则拖完进度条画面会停在原地
      if (playing.value) {
        seg.replayer?.setConfig({ speed: speed.value });
        seg.replayer?.play(offset);
      } else {
        seg.replayer?.pause(offset);
      }
    }
  }

  /** 拖动中的预览（P19 D6）：只改显示位置，不触碰任何 Replayer。 */
  function preview(ms: number) {
    previewTime.value = ms;
  }

  /** 松手提交：真正 seek 到预览位置。 */
  function commitPreview() {
    const ms = previewTime.value;
    if (ms == null) return;
    seek(ms);
  }

  function setSpeed(s: number) {
    speed.value = s;
    // 只对当前活跃段下发；后激活的段由 tick/play 的激活分支补 setConfig（P19 D8）
    for (const seg of segs) {
      if (active.has(seg.segmentId)) seg.replayer?.setConfig({ speed: s });
    }
  }

  function destroy() {
    destroyed = true;
    pause();
    previewTime.value = null;
    lastActiveByLabel = new Map();
    pending.clear();
    placeholders = new Map();
    allSignalsLoaded = false;
    ro?.disconnect();
    ro = null;
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
    signals.value = [];
  }

  return {
    ready,
    playing,
    currentTime,
    displayTime,
    totalTime,
    speed,
    mainLabel,
    autoFollow,
    timeline,
    signals,
    errorMarks,
    load,
    attachGrid,
    play,
    pause,
    seek,
    preview,
    commitPreview,
    setSpeed,
    selectMain,
    setAutoFollow,
    destroy,
  };
}
