import { HttpSink, IndexedDBSink } from "./sinks";
import { SegmentRecorder } from "./segment-recorder";
import { buildBundle, type Bundle } from "./bundle";
import { redact, type RedactionOptions } from "./redact";
import type { OfflineSessionData, SessionMeta, SignalSet } from "./types";

export interface InitOptions {
  /** 应用标识，随会话上报，用于 console 侧区分来源。 */
  appId: string;
  /** console 本地 HTTP server 地址，如 http://127.0.0.1:1421 */
  endpoint: string;
  /** 本地鉴权 token（console 设置页可查）。 */
  token?: string;
  env?: string;
  release?: string;
  /** 段标签，默认 "web"。SPA 路由连续，整页刷新会开新段。 */
  label?: string;
  signals?: SignalSet;
  /** 透传到 session meta 的额外字段。 */
  meta?: Partial<SessionMeta>;
}

export interface Controller {
  /** 显式停止：flush 残留事件并结束会话。 */
  stop(): Promise<void>;
}

/**
 * 在被观测 web 应用中调用一次：启动 rrweb record + 诊断信号 hook，
 * 经 HttpSink 上报到 console 本地 server。会话 = 一次页面访问。
 *
 * 页面卸载时自动用 sendBeacon 兜底 flush 已缓冲事件（会话结束标记 best-effort）。
 */
export async function init(opts: InitOptions): Promise<Controller> {
  const meta: SessionMeta = {
    source: "web",
    appId: opts.appId,
    env: opts.env,
    release: opts.release,
    userAgent: navigator.userAgent,
    viewport: `${window.innerWidth}x${window.innerHeight}`,
    url: location.href,
    ...opts.meta,
  };
  const sink = new HttpSink({
    endpoint: opts.endpoint,
    token: opts.token,
    meta,
  });
  const rec = new SegmentRecorder({
    sink,
    label: opts.label ?? "web",
    signals: opts.signals,
  });

  await sink.startSession();
  await rec.start();

  const onUnload = () => {
    rec.stopSync();
    sink.flushBeacon();
  };
  window.addEventListener("beforeunload", onUnload);

  return {
    async stop() {
      window.removeEventListener("beforeunload", onUnload);
      await rec.stop();
      await sink.endSession();
    },
  };
}

// ---- 离线采集：不依赖 console 在线，录到 IndexedDB，可导出 bundle ----

export interface RecordOfflineOptions {
  /** 应用标识，随会话上报，用于 console 侧区分来源。 */
  appId: string;
  env?: string;
  release?: string;
  /** 段标签，默认 "web"。 */
  label?: string;
  signals?: SignalSet;
  /** 透传到 session meta 的额外字段。 */
  meta?: Partial<SessionMeta>;
}

export interface OfflineController {
  /** 当前会话 id。 */
  readonly sessionId: string;
  /** 显式停止：flush 残留事件并结束会话。返回会话 id。 */
  stop(): Promise<string>;
  /** 序列化指定会话为 bundle（默认当前会话；导出当前会话会自动 stop）。可选脱敏。 */
  export(id?: string, redactOpts?: RedactionOptions): Promise<Bundle>;
  /** export + 触发浏览器下载。 */
  download(id?: string, filename?: string, redactOpts?: RedactionOptions): Promise<void>;
  /** 列出本机所有离线会话 meta（按开始时间倒序）。 */
  list(): Promise<OfflineSessionData["session"][]>;
  /** 删除指定会话；不传 id = 清空全部。 */
  clear(id?: string): Promise<void>;
  /** 销毁控制器：移除 unload 钩子并停止录制（保留已录数据）。 */
  destroy(): Promise<void>;
}

/**
 * 在被观测 web 应用中离线录制：SegmentRecorder + IndexedDBSink，不连 console。
 * 事件实时落 IndexedDB，后续可 export 成 prism-session bundle，下载或上传 console。
 *
 * 与 init()（HttpSink 实时上报）并列，差别仅在 Sink。
 * 注意：rrweb 事件经 SegmentRecorder 缓冲（默认 1s flush），页面突然关闭可能丢失
 * 末尾 <1s 的事件；正常用 stop() 收尾可避免。已落盘的会话可经 list() 找回再导出。
 */
export async function recordOffline(
  opts: RecordOfflineOptions,
): Promise<OfflineController> {
  const meta: SessionMeta = {
    source: "web",
    appId: opts.appId,
    env: opts.env,
    release: opts.release,
    userAgent: navigator.userAgent,
    viewport: `${window.innerWidth}x${window.innerHeight}`,
    url: location.href,
    ...opts.meta,
  };
  const sink = new IndexedDBSink();
  const rec = new SegmentRecorder({
    sink,
    label: opts.label ?? "web",
    signals: opts.signals,
  });
  const sessionId = await sink.startSession(meta);
  await rec.start();

  let stopped = false;
  const ensureStopped = async (): Promise<void> => {
    if (stopped) return;
    await rec.stop();
    await sink.endSession();
    stopped = true;
  };

  const onUnload = () => {
    rec.stopSync();
  };
  window.addEventListener("beforeunload", onUnload);

  const doExport = async (
    id: string,
    redactOpts?: RedactionOptions,
  ): Promise<Bundle> => {
    // 导出当前会话需先 stop（补 endedAt/hidden）；导出历史会话不打扰当前录制
    if (id === sessionId) await ensureStopped();
    const raw = await sink.readSession(id);
    if (!raw) throw new Error(`离线会话不存在：${id}`);
    const data = redactOpts ? redact(raw, redactOpts) : raw;
    return buildBundle(data);
  };

  return {
    sessionId,
    async stop() {
      await ensureStopped();
      return sessionId;
    },
    async export(id = sessionId, redactOpts) {
      return doExport(id, redactOpts);
    },
    async download(id = sessionId, filename, redactOpts) {
      const bundle = await doExport(id, redactOpts);
      const blob = new Blob([JSON.stringify(bundle)], {
        type: "application/json",
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = filename ?? `session-${sessionId}.json`;
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
    },
    async list() {
      return sink.listSessions();
    },
    async clear(id) {
      if (id) await sink.clearSession(id);
      else await sink.clearAll();
    },
    async destroy() {
      window.removeEventListener("beforeunload", onUnload);
      await ensureStopped();
    },
  };
}

// 类型与构件一并导出，供 self-obs（useRecorder）与高级用法复用。
export { HttpSink, IndexedDBSink } from "./sinks";
export type { HttpSinkOptions } from "./sinks";
export { SegmentRecorder } from "./segment-recorder";
export { installSignalHooks, emitSignal, serializeArg } from "./signals";
export {
  BUNDLE_FORMAT,
  BUNDLE_VERSION,
  SEGMENT_ID_RE,
  validateSegmentId,
  buildBundle,
  parseBundle,
} from "./bundle";
export type { Bundle, ParseResult } from "./bundle";
export { redact } from "./redact";
export type { RedactionOptions } from "./redact";
export type {
  RREvent,
  SessionMeta,
  LifecycleEvent,
  Sink,
  SignalPlugin,
  SignalSet,
  Source,
  Annotation,
  OfflineSessionData,
} from "./types";
