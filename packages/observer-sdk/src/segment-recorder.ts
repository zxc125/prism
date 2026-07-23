import { record } from "rrweb";
import { installSignalHooks } from "./signals";
import type { RREvent, SignalSet, Sink } from "./types";

export interface SegmentRecorderOptions {
  sink: Sink;
  /** 段标签，作为 segmentId 前缀：self-obs 用窗口 label，web SDK 用 "web"。 */
  label: string;
  signals?: SignalSet;
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
  private flushTimer: number | null = null;
  private segStart = 0;
  private destroyed = false;

  constructor(private opts: SegmentRecorderOptions) {}

  get active(): boolean {
    return this.segmentId != null;
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
    const stop = record({ emit });
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
    await this.flush();
    this.segmentId = null;
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
    if (!this.segmentId || this.buffer.length === 0) return;
    const events = this.buffer;
    this.buffer = [];
    try {
      await this.opts.sink.appendEvents(this.segmentId, events);
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
