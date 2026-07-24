import type {
  LifecycleEvent,
  OfflineSessionData,
  RREvent,
  SessionMeta,
  Sink,
} from "./types";

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

// ---- IndexedDBSink：纯 web 离线采集 + 导出 bundle ----
// 不依赖 console 在线：事件落 IndexedDB，可后续 readSession -> buildBundle 导出/上传。
// 与 HttpSink 行为对齐：beginSegment 记 shown、endSession 为开段补 hidden，
// 使离线会话的 windows 数据与 server 落盘一致，回放侧无感。

interface SegmentEntry {
  segmentId: string;
  sessionId: string;
  label: string;
  startedAt: number;
}
interface EventEntry {
  id: number;
  segmentId: string;
  event: RREvent;
}
interface LifecycleEntry {
  id: number;
  sessionId: string;
  event: LifecycleEvent;
}

export class IndexedDBSink implements Sink {
  private sessionId: string | null = null;
  private db: IDBDatabase | null = null;
  private segCounter = 0;
  private openSegments: string[] = [];

  private open(): Promise<IDBDatabase> {
    if (this.db) return Promise.resolve(this.db);
    return new Promise((resolve, reject) => {
      const req = indexedDB.open("observer-sessions", 2);
      req.onupgradeneeded = () => {
        const db = req.result;
        // 旧 stub（v1 单 events store）无 sessions store：清空重建
        if (
          db.objectStoreNames.contains("events") &&
          !db.objectStoreNames.contains("sessions")
        ) {
          db.deleteObjectStore("events");
        }
        if (!db.objectStoreNames.contains("sessions")) {
          db.createObjectStore("sessions", { keyPath: "id" });
        }
        if (!db.objectStoreNames.contains("segments")) {
          const s = db.createObjectStore("segments", { keyPath: "segmentId" });
          s.createIndex("sessionId", "sessionId", { unique: false });
        }
        if (!db.objectStoreNames.contains("events")) {
          const e = db.createObjectStore("events", { keyPath: "id", autoIncrement: true });
          e.createIndex("segmentId", "segmentId", { unique: false });
        }
        if (!db.objectStoreNames.contains("lifecycle")) {
          const l = db.createObjectStore("lifecycle", { keyPath: "id", autoIncrement: true });
          l.createIndex("sessionId", "sessionId", { unique: false });
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

  private get<T>(store: string, key: IDBValidKey): Promise<T | undefined> {
    return new Promise((resolve, reject) => {
      if (!this.db) return reject(new Error("db not open"));
      const req = this.db.transaction(store, "readonly").objectStore(store).get(key);
      req.onsuccess = () => resolve(req.result as T | undefined);
      req.onerror = () => reject(req.error);
    });
  }

  private getAllByIndex<T>(
    store: string,
    index: string,
    key: IDBValidKey,
  ): Promise<T[]> {
    return new Promise((resolve, reject) => {
      if (!this.db) return reject(new Error("db not open"));
      const req = this.db
        .transaction(store, "readonly")
        .objectStore(store)
        .index(index)
        .getAll(key);
      req.onsuccess = () => resolve(req.result as T[]);
      req.onerror = () => reject(req.error);
    });
  }

  async startSession(meta?: SessionMeta): Promise<string> {
    await this.open();
    this.sessionId = `local-${Date.now()}`;
    this.segCounter = 0;
    this.openSegments = [];
    const rec: OfflineSessionData["session"] = {
      ...(meta as SessionMeta),
      id: this.sessionId,
      startedAt: Date.now(),
    };
    await this.tx("sessions", "readwrite", (s) => {
      s.put(rec);
    });
    return this.sessionId;
  }

  async beginSegment(label?: string): Promise<string> {
    if (!this.sessionId) throw new Error("session not started");
    const label_ = label ?? "web";
    const segmentId = `${label_}#${++this.segCounter}`;
    await this.tx("segments", "readwrite", (s) => {
      s.put({
        segmentId,
        sessionId: this.sessionId!,
        label: label_,
        startedAt: Date.now(),
      });
    });
    // 与 /ingest/segment 对齐：开段即记 shown
    await this.appendLifecycle({
      type: "shown",
      label: label_,
      segmentId,
      t: Date.now(),
    });
    this.openSegments.push(segmentId);
    return segmentId;
  }

  async appendEvents(segmentId: string, events: RREvent[]): Promise<void> {
    await this.open();
    await this.tx("events", "readwrite", (s) => {
      for (const e of events) {
        s.put({ segmentId, event: e });
      }
    });
  }

  async appendLifecycle(ev: LifecycleEvent): Promise<void> {
    if (!this.sessionId) return;
    await this.open();
    await this.tx("lifecycle", "readwrite", (s) => {
      s.put({ sessionId: this.sessionId, event: ev });
    });
  }

  async endSession(): Promise<void> {
    if (!this.sessionId) return;
    const endedAt = Date.now();
    const sid = this.sessionId;
    // 与 /ingest/session/end 对齐：为每个开段补 hidden
    for (const seg of this.openSegments) {
      const label = seg.split("#")[0];
      await this.appendLifecycle({ type: "hidden", label, segmentId: seg, t: endedAt });
    }
    // 标记 endedAt
    await new Promise<void>((resolve, reject) => {
      if (!this.db) return reject(new Error("db not open"));
      const t = this.db.transaction("sessions", "readwrite");
      const store = t.objectStore("sessions");
      const getReq = store.get(sid);
      getReq.onsuccess = () => {
        const rec = getReq.result as OfflineSessionData["session"] | undefined;
        if (rec) {
          rec.endedAt = endedAt;
          store.put(rec);
        }
      };
      t.oncomplete = () => resolve();
      t.onerror = () => reject(t.error);
    });
    this.openSegments = [];
    this.sessionId = null;
  }

  async isRecordingActive(): Promise<boolean> {
    return this.sessionId != null;
  }

  // ---- 离线专属：读取 / 列举 / 清理（不在 Sink 接口上）----

  /** 读单个会话全部数据，供 buildBundle 序列化。不存在返回 null。 */
  async readSession(sessionId: string): Promise<OfflineSessionData | null> {
    await this.open();
    const session = await this.get<OfflineSessionData["session"]>("sessions", sessionId);
    if (!session) return null;
    const wins = await this.getAllByIndex<LifecycleEntry>("lifecycle", "sessionId", sessionId);
    const segRecs = await this.getAllByIndex<SegmentEntry>("segments", "sessionId", sessionId);
    const segments: Record<string, RREvent[]> = {};
    for (const seg of segRecs) {
      const evs = await this.getAllByIndex<EventEntry>("events", "segmentId", seg.segmentId);
      segments[seg.segmentId] = evs.map((e) => e.event);
    }
    return {
      session,
      windows: wins.map((w) => w.event),
      segments,
      annotations: [], // 离线采集阶段无标注（标注在 console 回放侧加）
    };
  }

  /** 列出所有离线会话 meta（按开始时间倒序）。 */
  async listSessions(): Promise<OfflineSessionData["session"][]> {
    await this.open();
    return new Promise((resolve, reject) => {
      if (!this.db) return reject(new Error("db not open"));
      const req = this.db.transaction("sessions", "readonly").objectStore("sessions").getAll();
      req.onsuccess = () =>
        resolve(
          (req.result as OfflineSessionData["session"][]).sort(
            (a, b) => b.startedAt - a.startedAt,
          ),
        );
      req.onerror = () => reject(req.error);
    });
  }

  /** 删单个会话及其所有段/事件/生命周期。 */
  async clearSession(sessionId: string): Promise<void> {
    await this.open();
    const segs = await this.getAllByIndex<SegmentEntry>("segments", "sessionId", sessionId);
    const segIds = segs.map((s) => s.segmentId);
    await new Promise<void>((resolve, reject) => {
      if (!this.db) return reject(new Error("db not open"));
      const t = this.db.transaction(
        ["sessions", "segments", "events", "lifecycle"],
        "readwrite",
      );
      t.objectStore("sessions").delete(sessionId);
      for (const id of segIds) t.objectStore("segments").delete(id);
      // events / lifecycle 按索引游标删
      for (const sid of segIds) {
        const req = t.objectStore("events").index("segmentId").openCursor(sid);
        req.onsuccess = () => {
          const cur = req.result;
          if (cur) {
            cur.delete();
            cur.continue();
          }
        };
      }
      const lcReq = t.objectStore("lifecycle").index("sessionId").openCursor(sessionId);
      lcReq.onsuccess = () => {
        const cur = lcReq.result;
        if (cur) {
          cur.delete();
          cur.continue();
        }
      };
      t.oncomplete = () => resolve();
      t.onerror = () => reject(t.error);
    });
  }

  /** 清空所有离线会话。 */
  async clearAll(): Promise<void> {
    await this.open();
    for (const store of ["sessions", "segments", "events", "lifecycle"]) {
      await this.tx(store, "readwrite", (s) => s.clear());
    }
  }
}
