//! bundle 契约的 TS 实现：rrweb-demo-session 格式的 build / parse / 校验。
//! 与 Rust 侧 build_export_bundle / write_import_bundle 对齐，规范见
//! docs/架构/bundle-规范.md。三条投递路径（本地文件 / 云端上传 / 离线 SDK）共用。

import type { OfflineSessionData, RREvent } from "./types";

/** bundle 标识与版本。改字段语义时 bump version，import 侧按版本分支。 */
export const BUNDLE_FORMAT = "rrweb-demo-session" as const;
export const BUNDLE_VERSION = 1 as const;

/**
 * segmentId 合法性：<label>#<n>，label 仅允许 [A-Za-z0-9_-]。
 * segment key 会成为文件名（segments/<key>.jsonl），此校验是路径穿越防护的核心：
 * 拒绝任何含路径分隔符 / `..` 的 key。
 */
export const SEGMENT_ID_RE = /^[A-Za-z0-9_-]+#[0-9]+$/;

export function validateSegmentId(id: string): boolean {
  return SEGMENT_ID_RE.test(id);
}

/** rrweb-demo-session bundle 的 TS 类型。 */
export interface Bundle {
  format: typeof BUNDLE_FORMAT;
  version: number;
  exportedAt: number;
  session: OfflineSessionData["session"];
  windows: OfflineSessionData["windows"];
  segments: Record<string, RREvent[]>;
  annotations: OfflineSessionData["annotations"];
}

/** 把离线会话内存形态序列化成 bundle。 */
export function buildBundle(data: OfflineSessionData): Bundle {
  return {
    format: BUNDLE_FORMAT,
    version: BUNDLE_VERSION,
    exportedAt: Date.now(),
    session: data.session,
    windows: data.windows,
    segments: data.segments,
    annotations: data.annotations,
  };
}

export interface ParseResult {
  ok: boolean;
  data?: OfflineSessionData;
  error?: string;
}

/**
 * 解析并校验 bundle。失败返回 ok:false + error。
 *
 * 校验：format/version 头、segment key 合法性（路径穿越防护）、必要字段存在。
 * 不校验 rrweb 事件内部结构（事件 schema 由 replay 侧宽容处理）。
 */
export function parseBundle(input: unknown): ParseResult {
  if (typeof input !== "object" || input === null) {
    return { ok: false, error: "bundle 不是对象" };
  }
  const b = input as Record<string, unknown>;

  if (b.format !== BUNDLE_FORMAT) {
    return { ok: false, error: `format 不符：期望 ${BUNDLE_FORMAT}` };
  }
  const version = b.version;
  if (typeof version !== "number" || version > BUNDLE_VERSION) {
    return {
      ok: false,
      error: `不支持的 bundle 版本：${version}（当前支持 ≤${BUNDLE_VERSION}）`,
    };
  }

  const session = b.session;
  if (typeof session !== "object" || session === null) {
    return { ok: false, error: "缺少 session" };
  }

  const windows = Array.isArray(b.windows) ? b.windows : [];
  const annotations = Array.isArray(b.annotations) ? b.annotations : [];

  const segments: Record<string, RREvent[]> = {};
  const segs = b.segments;
  if (typeof segs === "object" && segs !== null) {
    for (const [k, v] of Object.entries(segs as Record<string, unknown>)) {
      if (!validateSegmentId(k)) {
        return { ok: false, error: `非法 segmentId（拒绝以防路径穿越）：${k}` };
      }
      if (!Array.isArray(v)) {
        return { ok: false, error: `segment ${k} 事件不是数组` };
      }
      segments[k] = v as RREvent[];
    }
  }

  return {
    ok: true,
    data: {
      session: session as OfflineSessionData["session"],
      windows: windows as OfflineSessionData["windows"],
      segments,
      annotations: annotations as OfflineSessionData["annotations"],
    },
  };
}
