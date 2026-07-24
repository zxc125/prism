/** rrweb 事件 + 交错 type:6 诊断信号的统一事件类型（采集/传输/回放共用）。 */
export type RREvent = { timestamp: number } & Record<string, unknown>;

/** 会话来源：本机自录 / web SDK / tauri plugin。 */
export type Source = "self" | "web" | "tauri";

export interface SessionMeta {
  source?: Source;
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
 * - HttpSink：外部 SDK（web / tauri plugin）上报到 console 本地 HTTP server；
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

export type SignalPlugin = "error" | "console" | "network";

/** 诊断信号开关：默认全开；外部 SDK 可按需只开部分。 */
export type SignalSet = "all" | Partial<Record<SignalPlugin, boolean>>;

/** 用户标注（与 annotations.jsonl 一行对应）。session 级，与事件流分离。 */
export interface Annotation {
  id: string;
  t: number; // 时间码（session 内 ms，相对 startedAt）
  label?: string; // 关联窗口/段标签
  text: string;
  author?: string;
  createdAt?: number;
}

/**
 * 离线会话的内存形态：IndexedDBSink 读路径产出 / parseBundle 解析后得到。
 * 与磁盘布局（session.json + windows.jsonl + segments/*.jsonl + annotations.jsonl）一一对应。
 */
export interface OfflineSessionData {
  session: SessionMeta & {
    id: string;
    startedAt: number;
    endedAt?: number;
    importedAt?: number;
  };
  windows: LifecycleEvent[];
  segments: Record<string, RREvent[]>;
  annotations: Annotation[];
}
