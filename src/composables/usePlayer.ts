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

/**
 * 回放一个录制会话：按 windows.jsonl 的 shown/hidden 区间，
 * 在主时间轴上驱动各 segment 的 Replayer，平铺显示当前活跃窗口。
 */
export function usePlayer(sessionId: string) {
  const ready = ref(false);
  const playing = ref(false);
  const currentTime = ref(0); // 相对会话起点 ms
  const totalTime = ref(0);
  const speed = ref(1);

  let segs: SegmentInfo[] = [];
  let startTs = 0;
  let gridEl: HTMLElement | null = null;
  let tickTimer: number | null = null;
  let lastTick = 0;
  let active = new Set<string>();

  async function load() {
    const data = await invoke<SessionData>("read_session", { id: sessionId });
    segs = [];
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
    startTs = data.session.startedAt;
    const end =
      data.session.endedAt ??
      Math.max(startTs, ...segs.map((s) => s.hiddenAt ?? s.lastTs));
    totalTime.value = Math.max(0, end - startTs);
    currentTime.value = 0;
    ready.value = true;
  }

  function attachGrid(el: HTMLElement) {
    gridEl = el;
    for (const seg of segs) {
      if (seg.container) continue;
      const container = document.createElement("div");
      container.className = "tile";
      container.style.display = "none";
      container.dataset.segmentId = seg.segmentId;

      const header = document.createElement("div");
      header.className = "tile-header";
      header.textContent = seg.label;
      container.appendChild(header);

      const root = document.createElement("div");
      root.className = "tile-root";
      container.appendChild(root);
      gridEl.appendChild(container);

      seg.container = container;
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
    const next = new Set(activeAt(absT).map((s) => s.segmentId));
    for (const seg of segs) {
      if (!seg.container) continue;
      seg.container.style.display = next.has(seg.segmentId) ? "" : "none";
    }
    active = next;
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
    const next = new Set(activeAt(absT).map((s) => s.segmentId));
    // 新激活的段：开始播放
    for (const seg of activeAt(absT)) {
      if (!active.has(seg.segmentId)) {
        seg.replayer?.play(Math.max(0, absT - seg.firstTs));
      }
    }
    // 已失活的段：暂停
    for (const seg of segs) {
      if (active.has(seg.segmentId) && !next.has(seg.segmentId)) {
        seg.replayer?.pause();
      }
    }
    active = next;
    for (const seg of segs) {
      if (seg.container) {
        seg.container.style.display = next.has(seg.segmentId) ? "" : "none";
      }
    }
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
  }

  return {
    ready,
    playing,
    currentTime,
    totalTime,
    speed,
    load,
    attachGrid,
    play,
    pause,
    seek,
    setSpeed,
    destroy,
  };
}
