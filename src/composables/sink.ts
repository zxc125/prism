import { invoke } from "@tauri-apps/api/core";
import type {
  LifecycleEvent,
  RREvent,
  SessionMeta,
  Sink,
} from "@rrweb-demo/observer-sdk";

// 传输抽象与外部 Sink 实现下沉到 observer-sdk 包，本文件只保留 self-obs 专用的 TauriSink。
export type { LifecycleEvent, RREvent, SessionMeta, Sink } from "@rrweb-demo/observer-sdk";
export { HttpSink, IndexedDBSink } from "@rrweb-demo/observer-sdk";
export type { HttpSinkOptions } from "@rrweb-demo/observer-sdk";

/**
 * console 自录 Sink：包装 tauri-plugin-observer 的命令（`plugin:observer|*`）。
 * appendLifecycle 为空（Rust on_window_event 直接落盘）。
 */
export class TauriSink implements Sink {
  async startSession(_meta?: SessionMeta): Promise<string> {
    return invoke<string>("plugin:observer|start_session");
  }
  async beginSegment(_label?: string): Promise<string> {
    // label 由 Rust 按调用窗口推导，无需前端传
    return invoke<string>("plugin:observer|begin_segment");
  }
  async appendEvents(segmentId: string, events: RREvent[]): Promise<void> {
    await invoke("plugin:observer|append_events", { segmentId, events });
  }
  async appendLifecycle(_ev: LifecycleEvent): Promise<void> {
    // self-obs 窗口生命周期由 Rust on_window_event 直接落 windows.jsonl，前端不上报
  }
  async endSession(): Promise<void> {
    await invoke("plugin:observer|stop_session");
  }
  async isRecordingActive(): Promise<boolean> {
    return invoke<boolean>("plugin:observer|is_recording_active");
  }
}
