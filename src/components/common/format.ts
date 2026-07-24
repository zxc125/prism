/**
 * P10 app shell + 视图组件共享的工具：格式化时间 / 时长 / 来源色映射。
 * 从旧 MainView 抽出，避免在多个组件里重复定义。
 */
import type { SessionMeta, Source } from "../../composables/backend";

export const SRC_FILTERS = [
  { key: "all", label: "全部" },
  { key: "self", label: "本机" },
  { key: "web", label: "web" },
  { key: "tauri", label: "tauri" },
] as const;

export const SRC_COLOR: Record<Source, string> = {
  self: "var(--src-self)",
  web: "var(--src-web)",
  tauri: "var(--src-tauri)",
};

export const SRC_LABEL: Record<Source, string> = {
  self: "本机",
  web: "web",
  tauri: "tauri",
};

/** 来源取自 session.json 的 source 字段：self-obs 写 "self"，web SDK 写 "web"。 */
export function sourceOf(s: SessionMeta): Source {
  return (s.source as Source) ?? "self";
}

/** 时长格式化：mm:ss 或 h:mm:ss。 */
export function fmtDur(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  const m = Math.floor(s / 60);
  const h = Math.floor(m / 60);
  const p = (n: number) => n.toString().padStart(2, "0");
  return h > 0 ? `${h}:${p(m % 60)}:${p(s % 60)}` : `${m}:${p(s % 60)}`;
}

/** 时钟格式化：MM/DD HH:MM。 */
export function fmtClock(ts?: number): string {
  if (!ts) return "-";
  const d = new Date(ts);
  const p = (n: number) => n.toString().padStart(2, "0");
  return `${p(d.getMonth() + 1)}/${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}

/** 会话时长：endedAt - startedAt，未结束用 now。 */
export function sessionDur(s: SessionMeta): string {
  return fmtDur((s.endedAt ?? Date.now()) - s.startedAt);
}

/** 字节格式化：B/KB/MB/GB（配额条用）。 */
export function fmtBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}
