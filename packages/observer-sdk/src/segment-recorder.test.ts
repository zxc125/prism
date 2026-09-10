import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SegmentRecorder } from "./segment-recorder";
import type { RREvent, Sink } from "./types";

// rrweb 桩：node 环境无 document，且单测聚焦 SegmentRecorder 自身的
// 缓冲/串行/信号逻辑，不关心 rrweb 产出的事件。record 返回 stop 函数。
vi.mock("rrweb", () => ({ record: () => () => {} }));

type Batch = { segmentId: string; events: RREvent[] };

/**
 * 可控 Sink：默认 appendEvents 立即 resolve；hold() 后返回挂起 promise，
 * release() 释放——用于构造「在途批」场景。串行化保证同时至多一批在途。
 * network hook 引用 XMLHttpRequest（node 无此全局），测试统一跳过。
 */
function createSink() {
  let holding = false;
  let held: (() => void) | null = null;
  const batches: Batch[] = [];
  const appendEvents = vi.fn((segmentId: string, events: RREvent[]) => {
    batches.push({ segmentId, events });
    if (!holding) return Promise.resolve();
    return new Promise<void>((resolve) => {
      held = resolve;
    });
  });
  const sink: Sink = {
    startSession: async () => "s",
    beginSegment: async () => "main#0",
    appendEvents,
    appendLifecycle: async () => {},
    endSession: async () => {},
    isRecordingActive: async () => false,
  };
  return {
    sink,
    appendEvents,
    batches,
    /** 之后到达的批挂起（至多一批，串行化保证） */
    hold: () => {
      holding = true;
    },
    /** 释放挂起批并恢复正常 resolve */
    release: () => {
      holding = false;
      const h = held;
      held = null;
      h?.();
    },
    signals: { network: false } as const,
  };
}

// start() 内部用 window.setInterval 起 flush 定时器（node 环境无 window）
function stubWindow() {
  vi.stubGlobal("window", {
    setInterval: globalThis.setInterval.bind(globalThis),
    clearInterval: globalThis.clearInterval.bind(globalThis),
    addEventListener: () => {},
    removeEventListener: () => {},
  });
}

describe("SegmentRecorder flush 串行化（P17 D2）", () => {
  beforeEach(stubWindow);
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.clearAllMocks();
  });

  it("在途批未完成时新事件并入下一批，invoke 到达序 = 发送序", async () => {
    const { sink, appendEvents, hold, release, signals } = createSink();
    const rec = new SegmentRecorder({ sink, label: "main", signals });
    await rec.start();

    rec.signal("console", { n: 1 });
    hold();
    const first = rec.flush();
    expect(appendEvents).toHaveBeenCalledTimes(1);

    // 在途期间注入新事件再 flush：早退不并发，事件留在 buffer
    rec.signal("console", { n: 2 });
    await rec.flush();
    expect(appendEvents).toHaveBeenCalledTimes(1);

    release();
    await first;

    // 下一批发出，且晚于第一批（到达序 = 发送序）
    await rec.flush();
    expect(appendEvents).toHaveBeenCalledTimes(2);
    expect(appendEvents.mock.calls[0][0]).toBe("main#0");
    expect(appendEvents.mock.calls[1][0]).toBe("main#0");
  });

  it("stop() 先等在途批再尾 flush，段残留不丢", async () => {
    const { sink, appendEvents, hold, release, signals } = createSink();
    const rec = new SegmentRecorder({ sink, label: "main", signals });
    await rec.start();

    rec.signal("console", { n: 1 });
    hold();
    const first = rec.flush();
    rec.signal("console", { n: 2 }); // 在途期间到达

    const stopped = rec.stop();
    // stop 不与在途批并发
    expect(appendEvents).toHaveBeenCalledTimes(1);

    release();
    await first;
    await stopped;

    // 尾 flush 补发在途期间到达的事件，仍归属原段
    expect(appendEvents).toHaveBeenCalledTimes(2);
    expect(appendEvents.mock.calls[1][1]).toHaveLength(1);
    expect(
      JSON.parse(JSON.stringify(appendEvents.mock.calls[1][1][0])),
    ).toMatchObject({
      type: 6,
      data: { plugin: "console", payload: { n: 2 } },
    });
    expect(rec.active).toBe(false);
  });
});

describe("SegmentRecorder.signal（P17 D1）", () => {
  beforeEach(stubWindow);
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.clearAllMocks();
  });

  it("段未活跃时丢弃，活跃时以 type:6 进 buffer", async () => {
    const { sink, appendEvents, signals } = createSink();
    const rec = new SegmentRecorder({ sink, label: "main", signals });

    rec.signal("error", { m: "idle" }); // 未开段：丢弃
    await rec.start();
    await rec.flush(); // buffer 空，不发送
    expect(appendEvents).not.toHaveBeenCalled();

    rec.signal("console", { n: 1 });
    await rec.flush();
    expect(appendEvents).toHaveBeenCalledTimes(1);
    expect(
      JSON.parse(JSON.stringify(appendEvents.mock.calls[0][1][0])),
    ).toMatchObject({
      type: 6,
      data: { plugin: "console", payload: { n: 1 } },
    });
  });
});
