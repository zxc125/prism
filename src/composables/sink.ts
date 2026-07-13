import { invoke } from "@tauri-apps/api/core";

/** rrweb 事件 + 交错 type:6 诊断信号的统一事件类型（采集/传输/回放共用）。 */
export type RREvent = { timestamp: number } & Record<string, unknown>;

export interface SessionMeta {
  appId?: string;
  env?: string;
  release?: string;
  userAgent?: string;
  viewport?: string;
  url?: string;
}

export interface LifecycleEvent {
  type: "shown" | "hidden" | "focus";
  label: string;
  segmentId?: string;
  t: number;
}

/**
 * 采集端与落盘/上报之间的传输抽象。同一份采集逻辑（rrweb record + 信号 hook + 缓冲 flush）
 * 可对接不同 Sink：
 * - TauriSink：console 自录，进程内 invoke，零序列化；
 * - HttpSink：外部 SDK（web / tauri plugin）上报到 console 本地 HTTP server（P4 联调）；
 * - IndexedDBSink：纯 web 独立回放场景，本地缓存（预留）。
 *
 * 会话级命令（startSession/endSession/appendLifecycle）在 self-obs 由 Rust/MainView 驱动，
 * useRecorder 只用 beginSegment/appendEvents/isRecordingActive；外部 SDK 用完整接口。
 */
export interface Sink {
  startSession(meta?: SessionMeta): Promise<string>;
  beginSegment(label?: string): Promise<string>;
  appendEvents(segmentId: string, events: RREvent[]): Promise<void>;
  appendLifecycle(ev: LifecycleEvent): Promise<void>;
  endSession(): Promise<void>;
  isRecordingActive(): Promise<boolean>;
}

/** console 自录 Sink：包装现有 Tauri command。appendLifecycle 为空（Rust on_window_event 直接落盘）。 */
export class TauriSink implements Sink {
  async startSession(_meta?: SessionMeta): Promise<string> {
    return invoke<string>("start_session");
  }
  async beginSegment(_label?: string): Promise<string> {
    // label 由 Rust 按调用窗口推导，无需前端传
    return invoke<string>("begin_segment");
  }
  async appendEvents(segmentId: string, events: RREvent[]): Promise<void> {
    await invoke("append_events", { segmentId, events });
  }
  async appendLifecycle(_ev: LifecycleEvent): Promise<void> {
    // self-obs 窗口生命周期由 Rust on_window_event 直接落 windows.jsonl，前端不上报
  }
  async endSession(): Promise<void> {
    await invoke("stop_session");
  }
  async isRecordingActive(): Promise<boolean> {
    return invoke<boolean>("is_recording_active");
  }
}

// ---- HttpSink：外部 SDK 上报（P4 接入真实 server 后联调）----

export interface HttpSinkOptions {
  endpoint: string; // e.g. "http://127.0.0.1:1421"
  token?: string;
  meta?: SessionMeta;
  batchSize?: number; // 达到即 flush，默认 50
  flushInterval?: number; // 定时 flush，默认 1000ms
}

export class HttpSink implements Sink {
  private endpoint: string;
  private token?: string;
  private meta: SessionMeta;
  private batchSize: number;
  private flushInterval: number;
  private sessionId: string | null = null;
  private segCounter = 0;
  private buffer = new Map<string, RREvent[]>();
  private flushTimer: number | null = null;
  private unloadHandler: (() => void) | null = null;

  constructor(opts: HttpSinkOptions) {
    this.endpoint = opts.endpoint.replace(/\/$/, "");
    this.token = opts.token;
    this.meta = opts.meta ?? {};
    this.batchSize = opts.batchSize ?? 50;
    this.flushInterval = opts.flushInterval ?? 1000;
  }

  async startSession(meta?: SessionMeta): Promise<string> {
    const res = await this.post<{ sessionId: string }>("/ingest/session", {
      ...this.meta,
      ...meta,
    });
    this.sessionId = res.sessionId;
    this.startAutoFlush();
    return this.sessionId;
  }

  async beginSegment(label?: string): Promise<string> {
    const segmentId = `${label ?? "web"}#${++this.segCounter}`;
    await this.post("/ingest/segment", {
      sessionId: this.sessionId,
      label: label ?? "web",
      segmentId,
      startedAt: Date.now(),
    });
    return segmentId;
  }

  async appendEvents(segmentId: string, events: RREvent[]): Promise<void> {
    const arr = this.buffer.get(segmentId) ?? [];
    arr.push(...events);
    this.buffer.set(segmentId, arr);
    if (arr.length >= this.batchSize) await this.flush();
  }

  async appendLifecycle(ev: LifecycleEvent): Promise<void> {
    await this.post("/ingest/lifecycle", { sessionId: this.sessionId, ...ev });
  }

  async endSession(): Promise<void> {
    await this.flush();
    await this.post("/ingest/session/end", {
      sessionId: this.sessionId,
      endedAt: Date.now(),
    });
    this.stopAutoFlush();
  }

  async isRecordingActive(): Promise<boolean> {
    return this.sessionId != null;
  }

  private startAutoFlush(): void {
    if (this.flushTimer != null) return;
    this.flushTimer = window.setInterval(() => {
      void this.flush();
    }, this.flushInterval);
    this.unloadHandler = () => this.flushBeacon();
    window.addEventListener("beforeunload", this.unloadHandler);
  }

  private stopAutoFlush(): void {
    if (this.flushTimer != null) {
      clearInterval(this.flushTimer);
      this.flushTimer = null;
    }
    if (this.unloadHandler) {
      window.removeEventListener("beforeunload", this.unloadHandler);
      this.unloadHandler = null;
    }
  }

  /** flush 所有缓冲事件；失败放回队列，下次重试。P4 可加退避/容量上限。 */
  async flush(): Promise<void> {
    if (!this.buffer.size || !this.sessionId) return;
    const batch = this.buffer;
    this.buffer = new Map();
    for (const [segmentId, events] of batch) {
      try {
        await this.post("/ingest/events", {
          sessionId: this.sessionId,
          segmentId,
          events,
        });
      } catch (e) {
        const arr = this.buffer.get(segmentId) ?? [];
        arr.unshift(...events);
        this.buffer.set(segmentId, arr);
        console.error("[HttpSink] flush failed, will retry", e);
      }
    }
  }

  /** 页面卸载时用 sendBeacon 兜底 flush。 */
  private flushBeacon(): void {
    if (!this.buffer.size || !this.sessionId) return;
    for (const [segmentId, events] of this.buffer) {
      const payload = JSON.stringify({
        sessionId: this.sessionId,
        segmentId,
        events,
      });
      navigator.sendBeacon(`${this.endpoint}/ingest/events`, payload);
    }
    this.buffer = new Map();
  }

  private async post<T = unknown>(path: string, body: unknown): Promise<T> {
    const res = await fetch(`${this.endpoint}${path}`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        ...(this.token ? { Authorization: `Bearer ${this.token}` } : {}),
      },
      body: JSON.stringify(body),
    });
    if (!res.ok) throw new Error(`ingest ${path} failed: ${res.status}`);
    return res.status === 204 ? (undefined as T) : ((await res.json()) as T);
  }
}

// ---- IndexedDBSink：纯 web 独立回放骨架（预留，P4+ 纯 web 场景完善读取路径）----

export class IndexedDBSink implements Sink {
  private sessionId: string | null = null;
  private db: IDBDatabase | null = null;
  private segCounter = 0;

  private open(): Promise<IDBDatabase> {
    if (this.db) return Promise.resolve(this.db);
    return new Promise((resolve, reject) => {
      const req = indexedDB.open("observer-sessions", 1);
      req.onupgradeneeded = () => {
        const db = req.result;
        if (!db.objectStoreNames.contains("events")) {
          db.createObjectStore("events", { autoIncrement: true });
        }
      };
      req.onsuccess = () => {
        this.db = req.result;
        resolve(this.db);
      };
      req.onerror = () => reject(req.error);
    });
  }

  private tx(
    store: string,
    mode: IDBTransactionMode,
    fn: (s: IDBObjectStore) => void,
  ): Promise<void> {
    return new Promise((resolve, reject) => {
      if (!this.db) return reject(new Error("db not open"));
      const t = this.db.transaction(store, mode);
      fn(t.objectStore(store));
      t.oncomplete = () => resolve();
      t.onerror = () => reject(t.error);
    });
  }

  async startSession(_meta?: SessionMeta): Promise<string> {
    await this.open();
    this.sessionId = `local-${Date.now()}`;
    return this.sessionId;
  }

  async beginSegment(label?: string): Promise<string> {
    return `${label ?? "web"}#${++this.segCounter}`;
  }

  async appendEvents(segmentId: string, events: RREvent[]): Promise<void> {
    await this.open();
    await this.tx("events", "readwrite", (s) => {
      for (const e of events) {
        s.put({ sessionId: this.sessionId, segmentId, event: e });
      }
    });
  }

  async appendLifecycle(ev: LifecycleEvent): Promise<void> {
    await this.open();
    await this.tx("events", "readwrite", (s) => {
      s.put({ sessionId: this.sessionId, lifecycle: ev });
    });
  }

  async endSession(): Promise<void> {
    // TODO: 标记 endedAt；独立回放的读取路径在 P4+ 纯 web 场景补
    this.sessionId = null;
  }

  async isRecordingActive(): Promise<boolean> {
    return this.sessionId != null;
  }
}
