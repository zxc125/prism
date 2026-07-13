import { record } from "rrweb";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

type RREvent = { timestamp: number } & Record<string, unknown>;

type SignalPlugin = "error" | "console" | "network";

/** 构造一条 type:6 诊断信号事件，交错进 rrweb 事件流（与 DOM 共享时间轴）。 */
function emitSignal(
  emit: (e: RREvent) => void,
  segStart: number,
  plugin: SignalPlugin,
  payload: unknown,
) {
  const now = Date.now();
  emit({
    type: 6,
    timestamp: now,
    delay: now - segStart,
    data: { plugin, payload },
  });
}

/** 序列化 console 参数：基本类型直传、对象 JSON 克隆（截断循环引用）、Node 转简述、Error 转结构。 */
function serializeArg(a: unknown): unknown {
  if (a === null || typeof a !== "object") return a;
  if (a instanceof Error)
    return { __error: true, name: a.name, message: a.message, stack: a.stack };
  if (typeof Node !== "undefined" && a instanceof Node)
    return `<${a.nodeName.toLowerCase()}>`;
  try {
    return JSON.parse(JSON.stringify(a));
  } catch {
    return String(a);
  }
}

function setupErrorHook(emit: (e: RREvent) => void, segStart: number): () => void {
  const onError = (ev: ErrorEvent) => {
    emitSignal(emit, segStart, "error", {
      message: ev.message,
      source: ev.filename,
      lineno: ev.lineno,
      colno: ev.colno,
      stack: ev.error?.stack,
      kind: "onerror",
    });
  };
  const onRejection = (ev: PromiseRejectionEvent) => {
    const r = ev.reason;
    emitSignal(emit, segStart, "error", {
      message: r instanceof Error ? r.message : String(r),
      stack: r instanceof Error ? r.stack : undefined,
      kind: "unhandledrejection",
    });
  };
  window.addEventListener("error", onError);
  window.addEventListener("unhandledrejection", onRejection);
  return () => {
    window.removeEventListener("error", onError);
    window.removeEventListener("unhandledrejection", onRejection);
  };
}

function setupConsoleHook(emit: (e: RREvent) => void, segStart: number): () => void {
  const levels = ["log", "warn", "error", "info", "debug"] as const;
  const orig: Record<string, (...args: unknown[]) => void> = {};
  const c = console as unknown as Record<string, (...args: unknown[]) => void>;
  for (const level of levels) {
    orig[level] = c[level].bind(console);
    c[level] = (...args: unknown[]) => {
      orig[level](...args);
      emitSignal(emit, segStart, "console", {
        level,
        args: args.map(serializeArg),
      });
    };
  }
  return () => {
    for (const level of levels) c[level] = orig[level];
  };
}

function setupNetworkHook(emit: (e: RREvent) => void, segStart: number): () => void {
  const origFetch = window.fetch;
  const patchedFetch: typeof fetch = (input, init) => {
    const url =
      typeof input === "string"
        ? input
        : input instanceof URL
          ? input.href
          : input.url;
    const method = (init?.method ?? "GET").toString().toUpperCase();
    const start = performance.now();
    return origFetch(input, init).then(
      (res) => {
        emitSignal(emit, segStart, "network", {
          url,
          method,
          status: res.status,
          duration: Math.round(performance.now() - start),
          kind: "fetch",
        });
        return res;
      },
      (err) => {
        emitSignal(emit, segStart, "network", {
          url,
          method,
          status: 0,
          duration: Math.round(performance.now() - start),
          kind: "fetch",
          error: err instanceof Error ? err.message : String(err),
        });
        throw err;
      },
    );
  };
  window.fetch = patchedFetch;

  const origOpen = XMLHttpRequest.prototype.open;
  const origSend = XMLHttpRequest.prototype.send;
  const xhrMeta = new WeakMap<
    XMLHttpRequest,
    { method: string; url: string; start: number }
  >();

  XMLHttpRequest.prototype.open = function (
    this: XMLHttpRequest,
    method: string,
    url: string,
    ...rest: unknown[]
  ) {
    xhrMeta.set(this, { method: method.toUpperCase(), url, start: 0 });
    return (origOpen as (...a: unknown[]) => void).call(
      this,
      method,
      url,
      ...rest,
    );
  } as XMLHttpRequest["open"];

  XMLHttpRequest.prototype.send = function (
    this: XMLHttpRequest,
    ...args: unknown[]
  ) {
    const m = xhrMeta.get(this);
    if (m) {
      m.start = performance.now();
      this.addEventListener("loadend", () => {
        emitSignal(emit, segStart, "network", {
          url: m.url,
          method: m.method,
          status: this.status,
          duration: Math.round(performance.now() - m.start),
          kind: "xhr",
        });
      });
    }
    return (origSend as (...a: unknown[]) => void).apply(this, args);
  } as XMLHttpRequest["send"];

  return () => {
    window.fetch = origFetch;
    XMLHttpRequest.prototype.open = origOpen;
    XMLHttpRequest.prototype.send = origSend;
  };
}

/** 安装三类诊断信号 hook，返回卸载函数。仅在段录制期间活跃。 */
function installSignalHooks(
  emit: (e: RREvent) => void,
  segStart: number,
): () => void {
  const cleanups = [
    setupErrorHook(emit, segStart),
    setupConsoleHook(emit, segStart),
    setupNetworkHook(emit, segStart),
  ];
  return () => cleanups.forEach((fn) => fn());
}

/**
 * 每个窗口挂载时调用一次。负责：
 * - 监听全局 recording-session 广播（start/stop 会话）
 * - 监听定向 segment 事件（复用显示 start / 隐藏 stop）
 * - 兜底：挂载时若会话已进行，自启一段
 * 事件缓冲后每秒 flush 到 Rust 落盘。player-* 窗口不录制（避免回放被录进会话）。
 * 段录制期间安装 error/console/network 信号 hook，emit type:6 交错进同一段事件流。
 */
export function useRecorder() {
  const label = getCurrentWebviewWindow().label;
  const skip = label.startsWith("player-");

  let segmentId: string | null = null;
  let stopFn: (() => void) | null = null;
  let stopHooks: (() => void) | null = null;
  let buffer: RREvent[] = [];
  let flushTimer: number | null = null;
  let unlistenSession: UnlistenFn | null = null;
  let unlistenSegment: UnlistenFn | null = null;
  let destroyed = false;

  async function flush() {
    if (!segmentId || buffer.length === 0) return;
    const events = buffer;
    buffer = [];
    try {
      await invoke("append_events", { segmentId, events });
    } catch (e) {
      console.error("[recorder] append_events failed", e);
    }
  }

  function startSegment() {
    if (skip || destroyed) return;
    // 防止重复开段：先停掉已有录制与信号 hook
    if (stopFn || stopHooks) {
      stopFn?.();
      stopFn = null;
      stopHooks?.();
      stopHooks = null;
    }
    invoke<string>("begin_segment")
      .then((id) => {
        if (destroyed) return;
        segmentId = id;
        buffer = [];
        const segStart = Date.now();
        const emit = (e: RREvent) => {
          buffer.push(e);
        };
        const stop = record({ emit });
        stopFn = typeof stop === "function" ? (stop as () => void) : null;
        stopHooks = installSignalHooks(emit, segStart);
        if (flushTimer == null) {
          flushTimer = window.setInterval(flush, 1000);
        }
      })
      .catch((e) => console.error("[recorder] begin_segment failed", e));
  }

  async function stopSegment() {
    stopHooks?.();
    stopHooks = null;
    if (stopFn) {
      stopFn();
      stopFn = null;
    }
    await flush();
    segmentId = null;
  }

  async function setup() {
    if (skip) return;
    unlistenSession = await listen<{ active: boolean }>(
      "recording-session",
      (e) => {
        if (e.payload.active) startSegment();
        else void stopSegment();
      },
    );
    unlistenSegment = await listen<{ action: "start" | "stop" }>(
      "segment",
      (e) => {
        if (e.payload.action === "start") startSegment();
        else void stopSegment();
      },
    );
    // 兜底：会话开始后才创建的窗口，挂载时自启
    try {
      const active = await invoke<boolean>("is_recording_active");
      if (active) startSegment();
    } catch (e) {
      console.error("[recorder] is_recording_active failed", e);
    }
  }

  void setup();

  return {
    destroy() {
      destroyed = true;
      void stopSegment();
      if (flushTimer != null) {
        clearInterval(flushTimer);
        flushTimer = null;
      }
      unlistenSession?.();
      unlistenSegment?.();
    },
  };
}
