/**
 * Local 模式 Sink：包装 tauri-plugin-observer 的命令（`plugin:observer|*`）。
 *
 * Rust 直接落盘到宿主应用自己的 `appDataDir/recordings/`；窗口生命周期（hidden/focus）
 * 由插件 on_window_event 直接落 windows.jsonl，故 appendLifecycle 为 no-op。
 *
 * P16 起由 console（src/composables/sink.ts）下沉至此：console self-obs 与外部应用
 * Local 模式共用同一实现，经 index.ts re-export。
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  LifecycleEvent,
  RREvent,
  SessionMeta,
  Sink,
} from "@prism-obs/observer-sdk";

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
    // Local 模式窗口生命周期由 Rust on_window_event 直接落 windows.jsonl，前端不上报
  }
  async endSession(): Promise<void> {
    await invoke("plugin:observer|stop_session");
  }
  async isRecordingActive(): Promise<boolean> {
    return invoke<boolean>("plugin:observer|is_recording_active");
  }
}
