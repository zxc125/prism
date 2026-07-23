import { HttpSink } from "./sinks";
import { SegmentRecorder } from "./segment-recorder";
import type { SessionMeta, SignalSet } from "./types";

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

// 类型与构件一并导出，供 self-obs（useRecorder）与高级用法复用。
export { HttpSink, IndexedDBSink } from "./sinks";
export type { HttpSinkOptions } from "./sinks";
export { SegmentRecorder } from "./segment-recorder";
export { installSignalHooks, emitSignal, serializeArg } from "./signals";
export type {
  RREvent,
  SessionMeta,
  LifecycleEvent,
  Sink,
  SignalPlugin,
  SignalSet,
  Source,
} from "./types";
