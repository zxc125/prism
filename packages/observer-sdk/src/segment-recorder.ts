import { record } from "rrweb";
import { emitSignal, installSignalHooks } from "./signals";
import { resolveRecordOptions } from "./recording-profile";
import type {
  RecordingOptions,
  RREvent,
  SignalPlugin,
  SignalSet,
  Sink,
} from "./types";

export interface SegmentRecorderOptions {
  sink: Sink;
  /** 段标签，作为 segmentId 前缀：self-obs 用窗口 label，web SDK 用 "web"。 */
  label: string;
  signals?: SignalSet;
  /** 录制量控（P14）：档位 + 免录区。缺省 = full 档，与全量录制等价。 */
  recording?: RecordingOptions;
  flushInterval?: number; // 默认 1000ms
}

/**
 * 单段录制器：beginSegment -> 启动 rrweb record + 信号 hook + 定时 flush；
 * stop -> 卸载 hook、flush 残留事件。同一实例可多次 start/stop（self-obs 复用窗口）。
 *
 * self-obs（useRecorder）与外部 SDK 共用此类：差别仅在 Sink 注入与驱动方式
 *（self-obs 由 Rust 事件驱动 start/stop；SDK 自驱）。
 */
export class SegmentRecorder {
  private segmentId: string | null = null;
  private stopFn: (() => void) | null = null;
  private stopHooks: (() => void) | null = null;
  private buffer: RREvent[] = [];
  /** 在途 flush（P17 D2）：非 null 表示有一批 appendEvents 未完成，新事件留在 buffer 并入下一批。 */
  private flushing: Promise<void> | null = null;
  private flushTimer: number | null = null;
  private segStart = 0;
  private destroyed = false;

  constructor(private opts: SegmentRecorderOptions) {}

  get active(): boolean {
    return this.segmentId != null;
  }

  /**
   * 更新录制量控/信号配置（P14）：对下一段生效（start() 时才读取）。
   * self-obs 在每次开段前从持久化设置刷新，无需重建实例。
   */
  configure(next: { recording?: RecordingOptions; signals?: SignalSet }): void {
    if ("recording" in next) this.opts.recording = next.recording;
    if ("signals" in next) this.opts.signals = next.signals;
  }

  async start(): Promise<string> {
    if (this.destroyed) return "";
    // 防止重复开段：先停掉已有录制与信号 hook
    if (this.stopFn || this.stopHooks) {
      this.stopFn?.();
      this.stopFn = null;
      this.stopHooks?.();
      this.stopHooks = null;
    }
    const id = await this.opts.sink.beginSegment(this.opts.label);
    if (this.destroyed) return id;
    this.segmentId = id;
    this.buffer = [];
    this.segStart = Date.now();
    const emit = (e: RREvent) => {
      this.buffer.push(e);
    };
    // 量控参数在 start() 时 resolve（P14）：configure() 的变更对下一段生效
    const stop = record({ emit, ...resolveRecordOptions(this.opts.recording) });
    this.stopFn = typeof stop === "function" ? (stop as () => void) : null;
    this.stopHooks = installSignalHooks(emit, this.segStart, this.opts.signals);
    if (this.flushTimer == null) {
      this.flushTimer = window.setInterval(
        () => {
          void this.flush();
        },
        this.opts.flushInterval ?? 1000,
      );
    }
    return id;
  }

  async stop(): Promise<void> {
    this.stopHooks?.();
    this.stopHooks = null;
    if (this.stopFn) {
      this.stopFn();
      this.stopFn = null;
    }
    // 先等在途批完成（P17 D2）：在途批在 flush 起点已抓取段 id、归属当前段；
    // 不等则两批并发 invoke，async 化后的 append_events 到达序不再保证
    if (this.flushing) {
      try {
        await this.flushing;
      } catch {
        // flush() 内已记日志
      }
    }
    await this.flush();
    this.segmentId = null;
  }

  /**
   * 注入 type:6 诊断信号进当前段流（P17 D1）：与 installSignalHooks 的产出同构
   * （同一条 emit buffer、共享时间轴）。段未活跃时丢弃——宿主门控的「空闲异常兜底」
   * 应先 startSegment() 再 signal()。
   */
  signal(plugin: SignalPlugin, payload: unknown): void {
    if (this.segmentId == null) return;
    emitSignal((e) => this.buffer.push(e), this.segStart, plugin, payload);
  }

  /**
   * 同步停止：卸载 hook 并把残留事件塞进 Sink 的缓冲（HttpSink.appendEvents 的
   * 入队是同步的），用于 beforeunload 等无法 await 的路径。配合 HttpSink.flushBeacon。
   */
  stopSync(): void {
    this.stopHooks?.();
    this.stopHooks = null;
    if (this.stopFn) {
      this.stopFn();
      this.stopFn = null;
    }
    if (this.segmentId && this.buffer.length) {
      const events = this.buffer;
      this.buffer = [];
      void this.opts.sink.appendEvents(this.segmentId, events);
    }
    this.segmentId = null;
  }

  async flush(): Promise<void> {
    // 串行化（P17 D2）：在途批未完成时早退，buffer 里的新事件并入下一批（调用方
    // 不能把「await flush() 返回」理解为「已送达」——在途时本调用只入队等待）；
    // 因此批次 N+1 的 invoke 发生在批次 N 响应之后，到达序 = 发送序——
    // Sink 侧（TauriSink -> async 命令）即使脱离主线程执行也不会乱序
    if (this.flushing || !this.segmentId || this.buffer.length === 0) return;
    const events = this.buffer;
    this.buffer = [];
    const segmentId = this.segmentId;
    try {
      // Promise.resolve 包一层：同步抛异常的自定义 Sink 也走 catch（与 P17 前语义一致），
      // 不让定时器路径（void this.flush()）变成 unhandled rejection
      const pending = Promise.resolve(
        this.opts.sink.appendEvents(segmentId, events),
      );
      this.flushing = pending.finally(() => {
        this.flushing = null;
      });
      await this.flushing;
    } catch (e) {
      console.error("[recorder] append_events failed", e);
    }
  }

  destroy(): void {
    this.destroyed = true;
    void this.stop();
    if (this.flushTimer != null) {
      clearInterval(this.flushTimer);
      this.flushTimer = null;
    }
  }
}
