import type { RecordingOptions, SignalPlugin, SignalSet } from "@prism-obs/observer-sdk";

/**
 * 采集设置（P14）：录制档位 / 免录区域 / 诊断信号开关的 localStorage 持久化（D5）。
 * useRecorder（每次开段前刷新）与设置页 CaptureTab（保存）共用同一组 key 与读写函数；
 * 全缺省 = full 档位 + 信号全开，与 P14 之前的默认行为等价。
 */

const PROFILES = ["full", "balanced", "minimal"] as const;
type Profile = (typeof PROFILES)[number];

const KEYS = {
  profile: "prism.capture.profile",
  domBlocks: "prism.capture.domBlocks",
  signals: "prism.capture.signals",
} as const;

export interface CaptureSettings {
  /** 录制量控（档位 + 免录区），传给 SegmentRecorder。 */
  recording: RecordingOptions;
  /** 诊断信号开关（对象形式，缺省键 = 开；空 = 全开）。 */
  signals?: SignalSet;
}

function isProfile(v: unknown): v is Profile {
  return typeof v === "string" && (PROFILES as readonly string[]).includes(v);
}

function readDomBlocks(raw: string | null): string[] {
  if (!raw) return [];
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((s): s is string => typeof s === "string" && s.trim().length > 0);
  } catch {
    return [];
  }
}

function readSignals(raw: string | null): SignalSet | undefined {
  if (!raw) return undefined;
  try {
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) return undefined;
    const obj = parsed as Record<string, unknown>;
    const out: Partial<Record<SignalPlugin, boolean>> = {};
    for (const k of ["error", "console", "network"] as const) {
      if (obj[k] === false) out[k] = false;
    }
    // 只落 false 键：缺省键 = 开（installSignalHooks 语义），存储最小化
    return Object.keys(out).length ? out : undefined;
  } catch {
    return undefined;
  }
}

/** 读采集设置（带容错：非法值按缺省处理，不打断录制）。 */
export function loadCaptureSettings(): CaptureSettings {
  const profile = localStorage.getItem(KEYS.profile);
  return {
    recording: {
      profile: isProfile(profile) ? profile : "full",
      domBlocks: readDomBlocks(localStorage.getItem(KEYS.domBlocks)),
    },
    signals: readSignals(localStorage.getItem(KEYS.signals)),
  };
}

/** 写采集设置（CaptureTab 保存按钮调用）。 */
export function saveCaptureSettings(s: CaptureSettings): void {
  localStorage.setItem(KEYS.profile, s.recording.profile ?? "full");
  localStorage.setItem(KEYS.domBlocks, JSON.stringify(s.recording.domBlocks ?? []));
  localStorage.setItem(KEYS.signals, JSON.stringify(s.signals ?? {}));
}
