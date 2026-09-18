/**
 * Backend 抽象：console 读/管理会话的统一接口，隔离「本地 invoke」与「云端 HTTP」。
 *
 * P8 起 console 可连自托管 observer-server：默认 [`TauriBackend`]（invoke 本地命令），
 * 设置页切到 [`HttpBackend`] 后所有读/管理走云端 HTTP。**录制 Sink 与 Backend 正交**——
 * 录制仍走 HttpSink（指向本地或云端），Backend 只管读/管理。
 *
 * 见 docs/阶段路径/P8-云端server抽取.md / P10-浏览器版console.md。
 */
import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "./tauri";

// ---- 共享类型（与 SDK / Rust 侧对齐）----

export type Source = "self" | "web" | "tauri";

/** 会话列表项（list_sessions 返回的 session.json 元信息）。 */
export interface SessionMeta {
  id: string;
  startedAt: number;
  endedAt?: number;
  source?: Source;
  appId?: string;
  /** 会话归属用户（P22）：宿主 startSession 注入，仅外部 tauri/web 会话可能有。 */
  user?: { id: string; name?: string };
  name?: string;
  note?: string;
  tags?: string[];
  importedAt?: number;
}

/** 窗口生命周期事件（windows.jsonl 一行）。 */
export interface WindowEvent {
  type: "shown" | "hidden" | "focus";
  label: string;
  segmentId?: string;
  t: number;
}

/** 回放用：会话完整数据（read_session 返回）。 */
export interface SessionData {
  session: SessionMeta;
  windows: WindowEvent[];
  segments: Record<string, unknown[]>;
  annotations: Annotation[];
}

/** 段索引条目：段元信息，不含段内事件（`read_session_meta` 返回，O(段数) 读取）。 */
export interface SegmentIndexEntry {
  segmentId: string;
  bytes: number;
  /** 录制时视口尺寸（取自段首行 Meta 事件）；缺失为 null（回放侧不缩放）。 */
  width: number | null;
  height: number | null;
  /** 首/尾事件的 timestamp。`lastTs` 是时长兜底：外部应用导出的会话常常既无
   *  `endedAt` 也无 `hidden` 事件，段尾时间戳是唯一的时长来源。 */
  firstTs: number | null;
  lastTs: number | null;
}

/** 回放首屏数据：元信息 + 段索引，**不含段内事件**（P19 D1）。 */
export interface SessionMetaPayload {
  session: SessionMeta;
  windows: WindowEvent[];
  annotations: Annotation[];
  segments: SegmentIndexEntry[];
}

/** 用户标注（session 级，与事件流分离）。 */
export interface Annotation {
  id: string;
  t: number;
  label?: string;
  text: string;
  author: string;
  createdAt: number;
}

/** prism-session bundle（export/import 契约，见 docs/架构/bundle-规范.md）。 */
export interface Bundle {
  format: "prism-session";
  version: number;
  exportedAt?: number;
  session: SessionMeta;
  windows: WindowEvent[];
  segments: Record<string, unknown[]>;
  annotations: Annotation[];
}

/** 限流配置（服务端 RateLimitConfig 镜像）。 */
export interface RateLimitConfig {
  maxRpm?: number | null;
}

/** GET /whoami 返回：多租户带 tenant 上下文 + 配额；单租户仅 multiTenant:false。 */
export interface Whoami {
  multiTenant: boolean;
  tenantId?: string;
  appIds?: string[];
  quotaBytes?: number | null;
  usageBytes?: number;
  rateLimit?: RateLimitConfig;
  retention?: { maxAgeDays?: number | null; maxSessions?: number | null };
}

// ---- Backend 接口 ----

export interface Backend {
  listSessions(): Promise<SessionMeta[]>;
  readSession(id: string): Promise<SessionData>;
  /** P19 D1：首屏取元信息 + 段索引（不带段内事件）。 */
  readSessionMeta(id: string): Promise<SessionMetaPayload>;
  /** P19 D1：按需取单段事件。 */
  readSegment(id: string, segmentId: string): Promise<unknown[]>;
  /** 全库诊断信号（type:6）：完整信号流，不依赖各段是否已加载。 */
  readSessionSignals(id: string): Promise<unknown[]>;
  listAnnotations(id: string): Promise<Annotation[]>;
  saveAnnotations(id: string, annotations: Annotation[]): Promise<void>;
  updateSessionMeta(id: string, meta: Record<string, unknown>): Promise<SessionMeta>;
  exportSession(id: string): Promise<Bundle>;
  /** 从 bundle JSON 字符串导入（小文件 / 云端上传路径）。返回新 session id。 */
  importBundleContent(content: string): Promise<string>;
  /** 从本地文件路径导入（Rust 侧读文件，避免大 JSON 过 IPC）。返回新 session id。 */
  importBundlePath(path: string): Promise<string>;
  deleteSession(id: string): Promise<void>;
  /** P10：tenant 上下文 + 配额余量（顶栏 / /tenants 用）。 */
  whoami(): Promise<Whoami>;
}

// ---- TauriBackend：invoke 本地命令（默认）----

export class TauriBackend implements Backend {
  async listSessions() {
    return invoke<SessionMeta[]>("list_sessions");
  }
  async readSession(id: string) {
    return invoke<SessionData>("read_session", { id });
  }
  async readSessionMeta(id: string) {
    return invoke<SessionMetaPayload>("read_session_meta", { id });
  }
  async readSegment(id: string, segmentId: string) {
    // read_segment 是 raw IPC（P19 D2）：custom protocol 路径下到 JS 为 ArrayBuffer
    // （Rust 侧 Content-Type=application/octet-stream → ipc-protocol.js 走 arrayBuffer()）。
    // 其余形态做兜底，避免环境差异（postMessage 回退路径）让整段加载失败。
    return decodeEvents(await invoke<unknown>("read_segment", { id, segmentId }));
  }
  async readSessionSignals(id: string) {
    return decodeEvents(await invoke<unknown>("read_session_signals", { id }));
  }
  async listAnnotations(id: string) {
    return invoke<Annotation[]>("list_annotations", { id });
  }
  async saveAnnotations(id: string, annotations: Annotation[]) {
    await invoke("save_annotations", { id, annotations });
  }
  async updateSessionMeta(id: string, meta: Record<string, unknown>) {
    return invoke<SessionMeta>("update_session_meta", { id, meta });
  }
  async exportSession(id: string) {
    return invoke<Bundle>("export_session", { id });
  }
  async importBundleContent(content: string) {
    return invoke<string>("import_session", { content });
  }
  async importBundlePath(path: string) {
    return invoke<string>("import_session_path", { path });
  }
  async deleteSession(id: string) {
    await invoke("delete_session", { id });
  }
  async whoami(): Promise<Whoami> {
    // Tauri 桌面默认隐式单租户：console 内嵌 server 不暴露 tenant 上下文。
    // 浏览器化场景由 HttpBackend 处理；这里返回单租户标记，顶栏据此隐藏 tenant 切换器。
    return { multiTenant: false };
  }
}

// ---- HttpBackend：调云端 observer-server ----

export interface HttpBackendOptions {
  endpoint: string; // e.g. "https://obs.example.com" 或 "http://1.2.3.4:8080"
  apiKey: string;
}

export class HttpBackend implements Backend {
  private endpoint: string;
  private apiKey: string;

  constructor(opts: HttpBackendOptions) {
    this.endpoint = opts.endpoint.replace(/\/$/, "");
    this.apiKey = opts.apiKey;
  }

  async listSessions() {
    return this.get<SessionMeta[]>("/sessions");
  }
  async readSession(id: string) {
    return this.get<SessionData>(`/sessions/${enc(id)}`);
  }
  async readSessionMeta(id: string) {
    return this.get<SessionMetaPayload>(`/sessions/${enc(id)}/meta`);
  }
  async readSegment(id: string, segmentId: string) {
    // segmentId 形如 `main#0`——enc 会编码 `#`/`%`，服务端 percent-decode 还原
    return this.get<unknown[]>(`/sessions/${enc(id)}/segments/${enc(segmentId)}`);
  }
  async readSessionSignals(id: string) {
    return this.get<unknown[]>(`/sessions/${enc(id)}/signals`);
  }
  async listAnnotations(id: string) {
    return this.get<Annotation[]>(`/sessions/${enc(id)}/annotations`);
  }
  async saveAnnotations(id: string, annotations: Annotation[]) {
    await this.post(`/sessions/${enc(id)}/annotations`, annotations);
  }
  async updateSessionMeta(id: string, meta: Record<string, unknown>) {
    return this.patch<SessionMeta>(`/sessions/${enc(id)}`, meta);
  }
  async exportSession(id: string) {
    return this.get<Bundle>(`/sessions/${enc(id)}/export`);
  }
  async importBundleContent(content: string) {
    const res = await this.post<{ sessionId: string }>("/sessions/import", content);
    return res.sessionId;
  }
  async importBundlePath(path: string) {
    // Tauri 桌面：用 Rust 命令读文件内容再上传云端（避免大 JSON 过 IPC）。
    // 浏览器模式不应走到这里（pickBundleFile 已直接返回 content，调 importBundleContent）。
    if (!isTauri()) {
      throw new Error("浏览器模式请用 importBundleContent");
    }
    const content = await invoke<string>("read_text_file", { path });
    return this.importBundleContent(content);
  }
  async deleteSession(id: string) {
    await this.del(`/sessions/${enc(id)}`);
  }
  async whoami(): Promise<Whoami> {
    return this.get<Whoami>("/whoami");
  }

  private headers(): Record<string, string> {
    return this.apiKey
      ? { Authorization: `Bearer ${this.apiKey}` }
      : {};
  }

  private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const res = await fetch(`${this.endpoint}${path}`, {
      method,
      headers: {
        "Content-Type": "application/json",
        ...this.headers(),
      },
      body: body !== undefined ? (typeof body === "string" ? body : JSON.stringify(body)) : undefined,
    });
    if (!res.ok) {
      let msg = `${method} ${path} failed: ${res.status}`;
      try {
        const err = await res.json();
        if (err.error) msg = `${msg} · ${err.error}`;
      } catch {
        /* ignore parse error */
      }
      throw new Error(msg);
    }
    if (res.status === 204) return undefined as T;
    const text = await res.text();
    return (text ? JSON.parse(text) : undefined) as T;
  }

  private get<T>(path: string) {
    return this.request<T>("GET", path);
  }
  private post<T>(path: string, body?: unknown) {
    return this.request<T>("POST", path, body);
  }
  private patch<T>(path: string, body: unknown) {
    return this.request<T>("PATCH", path, body);
  }
  private del(path: string) {
    return this.request<void>("DELETE", path);
  }
}

function enc(id: string) {
  return encodeURIComponent(id);
}

/** `read_segment` 的 raw IPC 载荷 → 事件数组。正常路径是 ArrayBuffer（见 TauriBackend
 *  readSegment 注释），字符串/视图形态一并兼容。 */
function decodeEvents(raw: unknown): unknown[] {
  if (typeof raw === "string") return JSON.parse(raw) as unknown[];
  if (raw instanceof ArrayBuffer) {
    return JSON.parse(new TextDecoder().decode(raw)) as unknown[];
  }
  if (ArrayBuffer.isView(raw)) {
    return JSON.parse(new TextDecoder().decode(raw as Uint8Array)) as unknown[];
  }
  return raw as unknown[];
}

// ---- 配置 + 单例 ----

export type BackendMode = "tauri" | "http";

export interface BackendConfig {
  mode: BackendMode;
  endpoint: string;
  apiKey: string;
}

const STORAGE_KEY = "observer-backend";

export function defaultBackendConfig(): BackendConfig {
  return { mode: "tauri", endpoint: "", apiKey: "" };
}

export function loadBackendConfig(): BackendConfig {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return { ...defaultBackendConfig(), ...JSON.parse(raw) };
  } catch {
    /* ignore */
  }
  return defaultBackendConfig();
}

export function saveBackendConfig(cfg: BackendConfig) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(cfg));
}

let _backend: Backend | null = null;
let _backendKey = "";

/** 取当前 Backend（按 localStorage 配置缓存，配置变更后下次调用生效）。
 *  P10：浏览器（非 Tauri）强制 HttpBackend--无本地 invoke 可用；若未配 endpoint
 *  返回一个会抛错的占位，LoginGate 拦截后用户必须先填 endpoint+key。 */
export function getBackend(): Backend {
  const cfg = loadBackendConfig();
  const key = `${cfg.mode}|${cfg.endpoint}|${cfg.apiKey}`;
  if (_backend && key === _backendKey) return _backend;
  if (!isTauri()) {
    // 浏览器：必须有 endpoint+key 才能工作；LoginGate 保证已配置
    if (cfg.endpoint) {
      _backend = new HttpBackend({ endpoint: cfg.endpoint, apiKey: cfg.apiKey });
    } else {
      _backend = new UnconfiguredBackend();
    }
  } else {
    _backend =
      cfg.mode === "http" && cfg.endpoint
        ? new HttpBackend({ endpoint: cfg.endpoint, apiKey: cfg.apiKey })
        : new TauriBackend();
  }
  _backendKey = key;
  return _backend;
}

/** 浏览器未配置时占位：所有方法抛错，提示先登录。 */
class UnconfiguredBackend implements Backend {
  private boom() {
    return Promise.reject(new Error("未配置云端连接，请先登录"));
  }
  listSessions() { return this.boom(); }
  readSession() { return this.boom(); }
  readSessionMeta() { return this.boom(); }
  readSegment() { return this.boom(); }
  readSessionSignals() { return this.boom(); }
  listAnnotations() { return this.boom(); }
  saveAnnotations() { return this.boom(); }
  updateSessionMeta() { return this.boom(); }
  exportSession() { return this.boom(); }
  importBundleContent() { return this.boom(); }
  importBundlePath() { return this.boom(); }
  deleteSession() { return this.boom(); }
  async whoami(): Promise<Whoami> { return { multiTenant: false }; }
}

/** 强制重置缓存（设置页保存后调用，确保下次 getBackend 重建）。 */
export function resetBackend() {
  _backend = null;
  _backendKey = "";
}
