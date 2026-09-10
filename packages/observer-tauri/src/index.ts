/**
 * tauri-plugin-observer 的采集端驱动。
 *
 * 在被观测 Tauri 2 应用的每个窗口调用一次：监听插件 `recording-session` / `segment`
 * 事件驱动 `SegmentRecorder`。
 *
 * 两种模式（P16，`mode` 默认 `"remote"`）：
 * - `"remote"`：经 `HttpSink` 上报 console 本地 server（跨进程靠墙上时钟对齐）；sessionId
 *   由主窗口从 console server 取得后经插件 `bind_session` 广播，子窗口共享；窗口生命周期
 *   （hidden/focus）由 Rust 检测后 emit 事件，前端转发为 HttpSink lifecycle 上报。
 * - `"local"`：插件 Rust 直接落盘到宿主应用自己的 `appDataDir/recordings/`。Sink 为
 *   `TauriSink`（进程内 invoke，零序列化）；会话状态在插件 Rust 侧，前端无需 sessionId
 *   注入；hidden/focus 由 Rust 直接落 windows.jsonl。只读命令 `list_sessions`/
 *   `export_session` 供闭环：导出 `prism-session` bundle -> console 导入回放。
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  HttpSink,
  SegmentRecorder,
  type RecordingOptions,
  type SessionMeta,
  type SignalPlugin,
  type SignalSet,
  type Sink,
} from "@prism-obs/observer-sdk";
import { TauriSink } from "./sink";

export interface InitTauriOptions {
  /**
   * 部署模式。默认 `"remote"`：经 HttpSink 上报 console。
   * `"local"`：插件 Rust 直接落盘到本应用 appDataDir/recordings/。
   */
  mode?: "remote" | "local";
  /**
   * 应用标识。remote：随会话上报，console 侧区分来源；local：仅运行时标识——本地
   * session.json 的 source/appId 由宿主 Rust 侧 ObserverConfig 决定（D8 保持必填）。
   */
  appId: string;
  /** console 本地 HTTP server 地址，如 http://127.0.0.1:1421。remote 必填，local 忽略。 */
  endpoint?: string;
  /** 本地鉴权 token（console 设置页可查）。remote 用，local 忽略。 */
  token?: string;
  env?: string;
  release?: string;
  /**
   * 主窗口传 true：启动会话。remote 走 HttpSink.startSession + 插件 bind_session 广播；
   * local 走插件 start_session（Rust 建目录并广播 recording-session，各窗口自启）。
   * 子窗口不传：等广播或兜底自启。
   */
  autoStart?: boolean;
  signals?: SignalSet;
  /** 录制量控（P14）：档位 + domBlocks 免录区。缺省 = 全量录制。 */
  recording?: RecordingOptions;
  /**
   * 段门控（P17 手动档）。`"manual"`：`recording-session{active}` 广播与挂载兜底
   * 不再自动开段（只置会话态/绑 sessionId），段边界由宿主经 controller 的
   * `startSegment`/`stopSegment`/`signal` 驱动（交互/判闲/异常兜底策略全在宿主）；
   * 窗口可见性驱动的 `segment{start/stop}` 事件不受影响（复用显示/隐藏仍自动开停段）。
   * 缺省 = 现行为（会话活跃即持续录制），存量零 diff。
   */
  gating?: "manual";
  /** 透传到 session meta 的额外字段。 */
  meta?: Partial<SessionMeta>;
}

export interface TauriController {
  /**
   * 显式停止：卸载监听、flush 残留事件。Local 模式主窗口额外 stop_session
   * （Rust 写 endedAt 并广播停各窗）；remote 保持原行为（会话结束由 stop_session
   * 广播 -> 主窗口 endSession 链路驱动）。
   */
  stop(): Promise<void>;
  /** 当前段是否活跃（P17）：宿主门控策略的判据（如空闲异常兜底只在段未活跃时注入）。 */
  readonly active: boolean;
  /** 仅 Local 模式可用（remote 抛错）：列出本应用本地会话元信息。 */
  listSessions(): Promise<unknown[]>;
  /** 仅 Local 模式可用（remote 抛错）：导出 prism-session bundle JSON，宿主自行落盘/传输。 */
  exportSession(sessionId: string): Promise<unknown>;
  /**
   * 手动开段（P17 D4，幂等）：段已活跃时早退。门控策略（交互监听/判闲/异常兜底）
   * 由宿主实现——本方法只提供机制。配 `gating: "manual"` 使用；不开 gating 时
   * 调用也无害（事件驱动路径仍在，见各宿主职责权衡）。
   */
  startSegment(): Promise<void>;
  /**
   * 手动停段（P17 D4/D5，幂等）：段未活跃时早退。不补 hidden lifecycle——
   * 窗口隐藏语义归 Rust on_window_event（Local 落 windows.jsonl）与
   * `segment{stop}` 事件（Remote 补报），手动停段是宿主策略边界、非窗口隐藏。
   */
  stopSegment(): Promise<void>;
  /**
   * 注入 type:6 诊断信号进当前段流（P17 D1）：与信号 hook 产出同构，交错进
   * 同一条事件流。段未活跃时丢弃——「空闲异常兜底」应先 `startSegment()` 再注入。
   */
  signal(plugin: SignalPlugin, payload: unknown): void;
}

export async function initTauri(opts: InitTauriOptions): Promise<TauriController> {
  const label = getCurrentWebviewWindow().label;
  const isMain = !!opts.autoStart;
  const local = opts.mode === "local";
  const manualGating = opts.gating === "manual";
  if (!local && !opts.endpoint) {
    throw new Error("[observer-tauri] endpoint is required in remote mode");
  }

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
  // remote 专属调用（useSessionId 等）走 httpSink（local 下为 null，分支已隔离）；
  // Sink 接口共有的方法走 sink。
  const httpSink = local
    ? null
    : new HttpSink({ endpoint: opts.endpoint!, token: opts.token, meta });
  const sink: Sink = local ? new TauriSink() : httpSink!;
  const rec = new SegmentRecorder({
    sink,
    label,
    signals: opts.signals,
    recording: opts.recording,
  });

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
      // local：TauriSink.appendLifecycle 为 no-op（hidden 已由 Rust 落 windows.jsonl），
      // 调用无害且保持 remote 路径逐行为等价
      await sink.appendLifecycle({
        type: "hidden",
        label,
        segmentId: currentSegmentId,
        t: Date.now(),
      });
    }
    currentSegmentId = null;
  }

  // 会话广播：active -> 开段（remote 注入 bind_session 广播的 sessionId；local 忽略会话
  // 字段——TauriSink 无需 sessionId，会话状态与 id 生命周期在插件 Rust 侧）。
  // !active -> 停段 +（仅 remote 主窗口）endSession：local 的会话结束即 stop_session
  // 本身（endedAt 已由 Rust 落盘），再调只会空转第二次 stop_session。
  unlistens.push(
    await listen<{ active: boolean; sessionId?: string; id?: string }>(
      "recording-session",
      async (e) => {
        if (e.payload.active) {
          if (!local && e.payload.sessionId && !sessionBound) {
            httpSink?.useSessionId(e.payload.sessionId);
            sessionBound = true;
          }
          // manual 门控（P17）：会话活跃只置态/绑 sessionId，段由宿主策略开
          if (!manualGating) await startSegment();
        } else {
          await stopSegment(false);
          if (isMain && !local) await sink.endSession();
        }
      },
    ),
  );

  // 段事件：start 开新段；stop 停段。remote 需补报 hidden（server 侧无 Rust 落盘）；
  // local 同路径调用 no-op appendLifecycle，保持逐行为等价。
  unlistens.push(
    await listen<{ action: "start" | "stop" }>("segment", async (e) => {
      if (e.payload.action === "start") {
        await startSegment();
      } else {
        await stopSegment(true);
      }
    }),
  );

  // 窗口聚焦：仅 remote 注册。local 的 focus 由 Rust on_window_event 直接落盘，
  // 插件 Local 分支不 emit observer-lifecycle，监听了也收不到。
  if (!local) {
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
  }

  // 兜底：挂载时会话已进行（主窗口已 start/bind），取会话状态自启。
  // remote：is_recording_active + session_id（bind 广播过的 sessionId）；
  // local：session_id 恒 null（Remote 专用命令，读 remote_session_id）——勿调，
  // 只查 is_recording_active 即开段。
  try {
    const active = await invoke<boolean>("plugin:observer|is_recording_active");
    if (active) {
      if (!local) {
        const sid = await invoke<string | null>("plugin:observer|session_id");
        if (sid) {
          httpSink?.useSessionId(sid);
          sessionBound = true;
          if (!manualGating) await startSegment();
        }
      } else if (!manualGating) {
        await startSegment();
      }
    }
  } catch (e) {
    console.error("[observer-tauri] fallback start failed", e);
  }

  // 主窗口 autoStart。remote：HttpSink.startSession 拿 sessionId + bind_session 广播；
  // local：插件 start_session 建目录并广播 recording-session{active:true,id}
  // （各窗口含主窗自启），无需 bind_session。
  if (opts.autoStart) {
    try {
      if (local) {
        await invoke("plugin:observer|start_session");
      } else {
        const sid = await sink.startSession();
        sessionBound = true;
        await invoke("plugin:observer|bind_session", { sessionId: sid });
      }
    } catch (e) {
      console.error("[observer-tauri] autoStart failed", e);
    }
  }

  return {
    get active() {
      return rec.active;
    },
    async stop() {
      unlistens.forEach((fn) => fn?.());
      unlistens.length = 0;
      await rec.destroy();
      // Local 主窗口：显式结束会话（stop_session 写 endedAt + 广播停各窗）。
      // remote 保持原行为不动：其会话结束由 stop_session 广播 -> 主窗口 endSession 驱动。
      if (local && isMain) {
        try {
          await invoke("plugin:observer|stop_session");
        } catch (e) {
          console.error("[observer-tauri] stop_session failed", e);
        }
      }
    },
    async listSessions() {
      if (!local) throw new Error("[observer-tauri] listSessions 仅 Local 模式可用");
      return invoke<unknown[]>("plugin:observer|list_sessions");
    },
    async exportSession(sessionId: string) {
      if (!local) throw new Error("[observer-tauri] exportSession 仅 Local 模式可用");
      return invoke<unknown>("plugin:observer|export_session", { sessionId });
    },
    // 手动档（P17 D4/D5）：方法体引用的是上方同名局部闭包（对象方法名不入词法
    // 作用域，无递归风险）。startSegment/stopSegment 闭包自带 rec.active 幂等守卫。
    async startSegment() {
      await startSegment();
    },
    async stopSegment() {
      // 手动停段是宿主策略边界：不补 hidden lifecycle（D5）
      await stopSegment(false);
    },
    signal(plugin: SignalPlugin, payload: unknown) {
      rec.signal(plugin, payload);
    },
  };
}

// 复用 SDK 构件导出 + Local 模式 Sink（P16 下沉，console 侧 re-export 复用）
export { TauriSink } from "./sink";
export { HttpSink, SegmentRecorder } from "@prism-obs/observer-sdk";
export type { InitOptions, RecordingOptions, RecordProfile } from "@prism-obs/observer-sdk";
