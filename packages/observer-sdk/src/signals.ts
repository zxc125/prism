import type { RREvent, SignalPlugin, SignalSet } from "./types";

/** 构造一条 type:6 诊断信号事件，交错进 rrweb 事件流（与 DOM 共享时间轴）。 */
export function emitSignal(
  emit: (e: RREvent) => void,
  segStart: number,
  plugin: SignalPlugin,
  payload: unknown,
): void {
  const now = Date.now();
  emit({
    type: 6,
    timestamp: now,
    delay: now - segStart,
    data: { plugin, payload },
  });
}

/** 序列化 console 参数：基本类型直传、对象 JSON 克隆（截断循环引用）、Node 转简述、Error 转结构。 */
export function serializeArg(a: unknown): unknown {
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

/** 安装诊断信号 hook，返回卸载函数。仅在段录制期间活跃。按 set 过滤默认全开。 */
export function installSignalHooks(
  emit: (e: RREvent) => void,
  segStart: number,
  set: SignalSet = "all",
): () => void {
  const want = (k: SignalPlugin): boolean =>
    set === "all" || (typeof set === "object" && set[k] !== false);
  const cleanups: Array<() => void> = [];
  if (want("error")) cleanups.push(setupErrorHook(emit, segStart));
  if (want("console")) cleanups.push(setupConsoleHook(emit, segStart));
  if (want("network")) cleanups.push(setupNetworkHook(emit, segStart));
  return () => cleanups.forEach((fn) => fn());
}
