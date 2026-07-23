import type { LifecycleEvent, RREvent, SessionMeta, Sink } from "./types";

// ---- HttpSink：外部 SDK 上报到 console 本地 HTTP server ----

export interface HttpSinkOptions {
  endpoint: string; // e.g. "http://127.0.0.1:1421"
  token?: string;
  meta?: SessionMeta;
  batchSize?: number; // 达到即 flush，默认 50
  flushInterval?: number; // 定时 flush，默认 1000ms
}

/**
 * 外部采集器上报 Sink：批量 + 定时 flush 到 console 本地 HTTP server；
 * 失败放回队列重试；页面 unload 用 sendBeacon 兜底。
 *
 * 与 SegmentRecorder 配合时存在两层缓冲：recorder 每 flushInterval 把事件
 * 批量交给 appendEvents（这里再缓冲），本 Sink 再按 batchSize/定时发网络。
 */
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

  /**
   * 注入外部已存在的 sessionId（不调 /ingest/session），用于 Tauri 多窗口：
   * 主窗口 startSession 拿到 sessionId 后经插件广播，子窗口收到后用此方法注入共享。
   */
  useSessionId(sessionId: string): void {
    this.sessionId = sessionId;
    this.startAutoFlush();
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

  /** flush 所有缓冲事件；失败放回队列，下次重试。 */
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

  /** 页面卸载时用 sendBeacon 兜底 flush。公开以便 SDK 在 unload 路径调用。 */
  flushBeacon(): void {
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

// ---- IndexedDBSink：纯 web 独立回放骨架（预留，读取路径待补）----

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
    // TODO: 标记 endedAt；独立回放的读取路径在纯 web 场景补
    this.sessionId = null;
  }

  async isRecordingActive(): Promise<boolean> {
    return this.sessionId != null;
  }
}
