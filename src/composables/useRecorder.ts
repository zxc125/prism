import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { SegmentRecorder } from "@rrweb-demo/observer-sdk";
import { TauriSink } from "./sink";

/**
 * 每个窗口挂载时调用一次。负责：
 * - 监听全局 recording-session 广播（start/stop 会话）
 * - 监听定向 segment 事件（复用显示 start / 隐藏 stop）
 * - 兜底：挂载时若会话已进行，自启一段
 * 事件缓冲后每秒 flush 到 Sink 落盘。player-* 窗口不录制（避免回放被录进会话）。
 * 段录制期间安装 error/console/network 信号 hook，emit type:6 交错进同一段事件流。
 *
 * P10：player 走 in-app 路由 /s/:id（不再开独立窗口），需额外按 hash 路由判断--
 * 在 /s/:id 页面时跳过录制，避免回放 DOM 被录进会话。旧 player-* 窗口仍按 label 跳过。
 *
 * 段录制器 SegmentRecorder 来自 observer-sdk，与外部 Web SDK 共用同一份采集逻辑；
 * 差别仅在 Sink 注入（这里用 TauriSink）与驱动方式（这里由 Rust 事件驱动 start/stop）。
 */
export function useRecorder(sink: TauriSink = new TauriSink()) {
  const label = getCurrentWebviewWindow().label;
  const skip = label.startsWith("player-");

  // P10：in-app player 路由时也跳过（同窗口回放不应被录进会话）
  function isPlayerRoute(): boolean {
    return window.location.hash.startsWith("#/s/");
  }

  const rec = new SegmentRecorder({ sink, label });
  let unlistenSession: UnlistenFn | null = null;
  let unlistenSegment: UnlistenFn | null = null;
  let unlistenHash: (() => void) | null = null;

  async function setup() {
    if (skip) return;
    unlistenSession = await listen<{ active: boolean }>(
      "recording-session",
      (e) => {
        if (e.payload.active) {
          if (!isPlayerRoute()) void rec.start();
        } else {
          void rec.stop();
        }
      },
    );
    unlistenSegment = await listen<{ action: "start" | "stop" }>(
      "segment",
      (e) => {
        if (e.payload.action === "start") {
          if (!isPlayerRoute()) void rec.start();
        } else {
          void rec.stop();
        }
      },
    );
    // P10：in-app 路由变化时，进入 /s/:id 暂停当前段（避免录回放），离开时若会话仍活跃则开新段
    const onHash = () => {
      if (isPlayerRoute()) {
        void rec.stop();
      } else {
        sink.isRecordingActive()
          .then((active) => {
            if (active) void rec.start();
          })
          .catch(() => {});
      }
    };
    window.addEventListener("hashchange", onHash);
    unlistenHash = () => window.removeEventListener("hashchange", onHash);

    // 兜底：会话开始后才创建的窗口，挂载时自启（player 路由下不启）
    try {
      const active = await sink.isRecordingActive();
      if (active && !isPlayerRoute()) void rec.start();
    } catch (e) {
      console.error("[recorder] is_recording_active failed", e);
    }
  }

  void setup();

  return {
    destroy() {
      rec.destroy();
      unlistenSession?.();
      unlistenSegment?.();
      unlistenHash?.();
    },
  };
}
