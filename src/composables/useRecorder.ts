import { record } from "rrweb";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

type RREvent = { timestamp: number } & Record<string, unknown>;

/**
 * 每个窗口挂载时调用一次。负责：
 * - 监听全局 recording-session 广播（start/stop 会话）
 * - 监听定向 segment 事件（复用显示 start / 隐藏 stop）
 * - 兜底：挂载时若会话已进行，自启一段
 * 事件缓冲后每秒 flush 到 Rust 落盘。player-* 窗口不录制（避免回放被录进会话）。
 */
export function useRecorder() {
  const label = getCurrentWebviewWindow().label;
  const skip = label.startsWith("player-");

  let segmentId: string | null = null;
  let stopFn: (() => void) | null = null;
  let buffer: RREvent[] = [];
  let flushTimer: number | null = null;
  let unlistenSession: UnlistenFn | null = null;
  let unlistenSegment: UnlistenFn | null = null;
  let destroyed = false;

  async function flush() {
    if (!segmentId || buffer.length === 0) return;
    const events = buffer;
    buffer = [];
    try {
      await invoke("append_events", { segmentId, events });
    } catch (e) {
      console.error("[recorder] append_events failed", e);
    }
  }

  function startSegment() {
    if (skip || destroyed) return;
    // 防止重复开段
    if (stopFn) {
      stopFn();
      stopFn = null;
    }
    invoke<string>("begin_segment")
      .then((id) => {
        if (destroyed) return;
        segmentId = id;
        buffer = [];
        const stop = record({
          emit(event) {
            buffer.push(event as RREvent);
          },
        });
        stopFn = typeof stop === "function" ? (stop as () => void) : null;
        if (flushTimer == null) {
          flushTimer = window.setInterval(flush, 1000);
        }
      })
      .catch((e) => console.error("[recorder] begin_segment failed", e));
  }

  async function stopSegment() {
    if (stopFn) {
      stopFn();
      stopFn = null;
    }
    await flush();
    segmentId = null;
  }

  async function setup() {
    if (skip) return;
    unlistenSession = await listen<{ active: boolean }>(
      "recording-session",
      (e) => {
        if (e.payload.active) startSegment();
        else void stopSegment();
      },
    );
    unlistenSegment = await listen<{ action: "start" | "stop" }>(
      "segment",
      (e) => {
        if (e.payload.action === "start") startSegment();
        else void stopSegment();
      },
    );
    // 兜底：会话开始后才创建的窗口，挂载时自启
    try {
      const active = await invoke<boolean>("is_recording_active");
      if (active) startSegment();
    } catch (e) {
      console.error("[recorder] is_recording_active failed", e);
    }
  }

  void setup();

  return {
    destroy() {
      destroyed = true;
      void stopSegment();
      if (flushTimer != null) {
        clearInterval(flushTimer);
        flushTimer = null;
      }
      unlistenSession?.();
      unlistenSegment?.();
    },
  };
}
