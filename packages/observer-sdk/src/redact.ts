//! 导出前脱敏：剥离/scrub 会话中的 PII（network body/headers、console args、url 等）。
//! 离线 bundle 上传或分享前应过一遍 redact，避免把 token/密码/邮箱带出去。
//! DOM 文本节点本阶段不处理（rrweb 事件结构复杂，留后续；如需可 dropConsole + 录制时关 network body）。

import type { OfflineSessionData, RREvent } from "./types";

export interface RedactionOptions {
  /** 剥离 network 请求/响应 body（默认 true，PII 压力最大）。 */
  stripNetworkBody?: boolean;
  /** 剥离 network 请求/响应 headers（默认 true）。 */
  stripNetworkHeaders?: boolean;
  /** 正则 scrubber：在字符串字段中匹配并替换为 [REDACTED]。 */
  scrubbers?: RegExp[];
  /** 完全丢弃 network 信号事件（默认 false）。 */
  dropNetwork?: boolean;
  /** 完全丢弃 console 信号事件（默认 false）。 */
  dropConsole?: boolean;
}

interface ResolvedOpts {
  stripNetworkBody: boolean;
  stripNetworkHeaders: boolean;
  dropNetwork: boolean;
  dropConsole: boolean;
}

const DEFAULT_OPTS: ResolvedOpts = {
  stripNetworkBody: true,
  stripNetworkHeaders: true,
  dropNetwork: false,
  dropConsole: false,
};

function scrubStr(s: string, scrubbers: RegExp[]): string {
  let out = s;
  for (const re of scrubbers) out = out.replace(re, "[REDACTED]");
  return out;
}

/** 递归对字符串值套用 scrubbers；非字符串原样返回。 */
function scrubValue(v: unknown, scrubbers: RegExp[]): unknown {
  if (typeof v === "string") return scrubStr(v, scrubbers);
  if (Array.isArray(v)) return v.map((x) => scrubValue(x, scrubbers));
  if (v && typeof v === "object") {
    const o: Record<string, unknown> = {};
    for (const [k, val] of Object.entries(v as Record<string, unknown>)) {
      o[k] = scrubValue(val, scrubbers);
    }
    return o;
  }
  return v;
}

/** 单个事件脱敏；返回 null 表示丢弃该事件。 */
function redactEvent(
  e: RREvent,
  o: ResolvedOpts,
  scrubbers: RegExp[],
): RREvent | null {
  // type:6 是诊断信号；其他（DOM 快照等）本阶段不动
  if (e.type !== 6) return e;
  const data = e.data as
    | { plugin?: string; payload?: Record<string, unknown> }
    | undefined;
  const plugin = data?.plugin;
  if (plugin === "network" && o.dropNetwork) return null;
  if (plugin === "console" && o.dropConsole) return null;
  if (!data?.payload) return e;

  const payload: Record<string, unknown> = { ...data.payload };
  if (plugin === "network") {
    if (o.stripNetworkBody) {
      delete payload.reqBody;
      delete payload.resBody;
    }
    if (o.stripNetworkHeaders) {
      delete payload.reqHeaders;
      delete payload.resHeaders;
    }
  }
  const newPayload = scrubbers.length
    ? (scrubValue(payload, scrubbers) as Record<string, unknown>)
    : payload;
  return { ...e, data: { ...data, payload: newPayload } };
}

/**
 * 对离线会话数据脱敏，返回新对象（不改原对象）。
 * 典型 scrubbers：`/Bearer\s+[\w.-]+/g`、`/[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+/g`。
 */
export function redact(
  data: OfflineSessionData,
  opts: RedactionOptions = {},
): OfflineSessionData {
  const o: ResolvedOpts = { ...DEFAULT_OPTS, ...opts };
  const scrubbers = opts.scrubbers ?? [];

  const segments: Record<string, RREvent[]> = {};
  for (const [segId, events] of Object.entries(data.segments)) {
    segments[segId] = events
      .map((e) => redactEvent(e, o, scrubbers))
      .filter((e): e is RREvent => e !== null);
  }

  // session.url 可能带 query token，过一遍 scrubbers
  const session = { ...data.session };
  if (scrubbers.length && typeof session.url === "string") {
    session.url = scrubStr(session.url, scrubbers);
  }

  return { ...data, segments, session };
}
