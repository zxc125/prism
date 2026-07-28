/**
 * tauri-plugin-observer 的采集端驱动（Remote 模式）。
 *
 * 在被观测 Tauri 2 应用的每个窗口调用一次：监听插件 `recording-session` / `segment`
 * 事件驱动 `SegmentRecorder` 开/停段，经 `HttpSink` 上报到 console 本地 server。
 *
 * 与 self-obs（`useRecorder` + `TauriSink`）的差别：
 * - Sink 用 `HttpSink`（外部进程，跨进程靠墙上时钟）而非 `TauriSink`（进程内 invoke）；
 * - 窗口生命周期（hidden/focus）由 Rust 检测后 emit 事件，前端转发为 HttpSink lifecycle 上报；
 * - sessionId 由主窗口从 console server 取得后经插件 `bind_session` 广播，子窗口共享。
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  HttpSink,
  SegmentRecorder,
  type SessionMeta,
  type SignalSet,
} from "@prism/observer-sdk";

export interface InitTauriOptions {
  /** 应用标识，随会话上报，用于 console 侧区分来源。 */
  appId: string;
  /** console 本地 HTTP server 地址，如 http://127.0.0.1:1421 */
  endpoint: string;
  /** 本地鉴权 token（console 设置页可查）。 */
  token?: string;
  env?: string;
  release?: string;
  /**
   * 主窗口传 true：启动会话（HttpSink.startSession + 插件 bind_session 广播）。
   * 子窗口不传：等待 recording-session 广播拿到 sessionId 后自启。
   */
  autoStart?: boolean;
  signals?: SignalSet;
  /** 透传到 session meta 的额外字段。 */
  meta?: Partial<SessionMeta>;
}

export interface TauriController {
  /** 显式停止：卸载监听、flush 残留事件、结束会话（仅主窗口）。 */
  stop(): Promise<void>;
}

export async function initTauri(opts: InitTauriOptions): Promise<TauriController> {
  const label = getCurrentWebviewWindow().label;
  const isMain = !!opts.autoStart;

  const meta: SessionMeta = {
    source: "tauri",
    appId: opts.appId,
    env: opts.env,
    release: opts.release,
    userAgent: navigator.userAgent,
    viewport: `${window.innerWidth}x${window.innerHeight}`,
    url: location.href,
    ...opts.meta,
  };
  const sink = new HttpSink({ endpoint: opts.endpoint, token: opts.token, meta });
  const rec = new SegmentRecorder({ sink, label, signals: opts.signals });

  let currentSegmentId: string | null = null;
  let sessionBound = false;
  const unlistens: UnlistenFn[] = [];

  async function startSegment() {
    if (rec.active) return;
    const id = await rec.start();
    currentSegmentId = id;
  }
  async function stopSegment(reportHidden: boolean) {
    if (!rec.active) return;
    await rec.stop();
    if (reportHidden && currentSegmentId) {
      await sink.appendLifecycle({
        type: "hidden",
        label,
        segmentId: currentSegmentId,
        t: Date.now(),
      });
    }
    currentSegmentId = null;
  }

  // 会话广播：active+sessionId -> 注入 sessionId 并开段；!active -> 停段 +（主窗口）endSession
  unlistens.push(
    await listen<{ active: boolean; sessionId?: string }>(
      "recording-session",
      async (e) => {
        if (e.payload.active) {
          if (e.payload.sessionId && !sessionBound) {
            sink.useSessionId(e.payload.sessionId);
            sessionBound = true;
          }
          await startSegment();
        } else {
          await stopSegment(false);
          if (isMain) await sink.endSession();
        }
      },
    ),
  );

  // 段事件：start 开新段；stop 停段 + 报 hidden（窗口隐藏）
  unlistens.push(
    await listen<{ action: "start" | "stop" }>("segment", async (e) => {
      if (e.payload.action === "start") {
        await startSegment();
      } else {
        await stopSegment(true);
      }
    }),
  );

  // 窗口聚焦：Rust 检测后 emit observer-lifecycle，经 HttpSink 上报 focus
  unlistens.push(
    await listen<{ type: "focus"; label: string; t: number }>(
      "observer-lifecycle",
      async (e) => {
        if (e.payload.type === "focus") {
          await sink.appendLifecycle({
            type: "focus",
            label: e.payload.label,
            t: e.payload.t,
          });
        }
      },
    ),
  );

  // 兜底：挂载时会话已进行（主窗口已 bind），取 sessionId 自启
  try {
    const active = await invoke<boolean>("plugin:observer|is_recording_active");
    if (active) {
      const sid = await invoke<string | null>("plugin:observer|session_id");
      if (sid) {
        sink.useSessionId(sid);
        sessionBound = true;
        await startSegment();
      }
    }
  } catch (e) {
    console.error("[observer-tauri] fallback start failed", e);
  }

  // 主窗口 autoStart：建会话 + bind 广播（各窗口收到后自启）
  if (opts.autoStart) {
    try {
      const sid = await sink.startSession();
      sessionBound = true;
      await invoke("plugin:observer|bind_session", { sessionId: sid });
    } catch (e) {
      console.error("[observer-tauri] autoStart failed", e);
    }
  }

  return {
    async stop() {
      unlistens.forEach((fn) => fn?.());
      unlistens.length = 0;
      await rec.destroy();
    },
  };
}

// 复用 SDK 构件导出
export { HttpSink, SegmentRecorder } from "@prism/observer-sdk";
export type { InitOptions } from "@prism/observer-sdk";
