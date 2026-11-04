//! bundle 上传（P24）：把 prism-session bundle POST 到观测端 `/sessions/import`。
//!
//! 接收端为 observer-server 既有端点（console 内嵌与自托管同一份代码），鉴权为可选
//! Bearer token；契约零变更（format/version 由服务端校验）。本模块不做脱敏——需要
//! 脱敏的宿主自行经 `export(id, redactOpts)` → `uploadBundle` 组合（见 决策/一键打包上传.md D3）。
//!
//! 请求侧 gzip（D5）：`gzip: true` 时请求体压缩并带 `Content-Encoding: gzip`。
//! 老 server 对压缩 body 会静默 400，必须先经 `probeRequestGzip`（/whoami 的
//! `requestGzip` 能力标记）确认后启用；SDK 默认不压缩。

import type { Bundle } from "./bundle";

export interface UploadOptions {
  /** 观测端地址（去尾斜杠），如 "http://127.0.0.1:1421" 或自托管 server。 */
  endpoint: string;
  /** 观测端鉴权 token；缺省不带 Authorization 头（本机回环免鉴权）。 */
  token?: string;
  /**
   * 请求体 gzip 压缩。默认 false：老 server 对压缩 body 静默 400，
   * 仅在 probeRequestGzip() 返回 true 时启用。
   */
  gzip?: boolean;
  /** fetch 注入（测试用）；缺省全局 fetch。 */
  fetchImpl?: typeof fetch;
}

export interface UploadResult {
  /** 观测端分配的新会话 id（import 语义：原 id 被替换，不跨机保留）。 */
  sessionId: string;
}

/** 去尾斜杠归一 endpoint（对齐 HttpSink 行为）。 */
function normalizeEndpoint(endpoint: string): string {
  return endpoint.replace(/\/$/, "");
}

async function gzipString(input: string): Promise<Uint8Array> {
  if (typeof CompressionStream === "undefined") {
    throw new Error("当前环境不支持 CompressionStream，无法压缩请求体");
  }
  const stream = new Blob([input]).stream().pipeThrough(new CompressionStream("gzip"));
  const buf = await new Response(stream).arrayBuffer();
  return new Uint8Array(buf);
}

/**
 * 上传完整 bundle 到观测端 `POST /sessions/import`。
 *
 * 非 2xx 抛错（优先带服务端 {error} 文案）；成功返回观测端分配的新 sessionId。
 * 重复上传同一 bundle 会在观测端产生重复会话（import 换新 id 语义，接受重复，
 * 见 决策/一键打包上传.md D4）。
 */
export async function uploadBundle(
  bundle: Bundle,
  opts: UploadOptions,
): Promise<UploadResult> {
  const fetchImpl = opts.fetchImpl ?? fetch.bind(globalThis);
  const url = `${normalizeEndpoint(opts.endpoint)}/sessions/import`;
  const bodyStr = JSON.stringify(bundle);

  const headers: Record<string, string> = { "Content-Type": "application/json" };
  if (opts.token) headers["Authorization"] = `Bearer ${opts.token}`;
  let body: BodyInit = bodyStr;
  if (opts.gzip) {
    body = await gzipString(bodyStr);
    headers["Content-Encoding"] = "gzip";
  }

  const res = await fetchImpl(url, { method: "POST", headers, body });
  if (!res.ok) {
    let msg = `HTTP ${res.status}`;
    try {
      const j = (await res.json()) as { error?: string };
      if (j?.error) msg = `HTTP ${res.status} · ${j.error}`;
    } catch {
      // 非 JSON 错误体，保留状态码文案
    }
    throw new Error(`上传失败：${msg}`);
  }
  const data = (await res.json().catch(() => null)) as { sessionId?: unknown } | null;
  if (!data || typeof data.sessionId !== "string") {
    throw new Error("上传响应缺少 sessionId");
  }
  return { sessionId: data.sessionId };
}

/**
 * 探测观测端是否支持请求侧 gzip（GET /whoami 的 `requestGzip` 标记）。
 *
 * 任何失败（网络错误、鉴权失败、老 server 无该字段）都返回 false——调用方据此
 * 回退到明文上传。探测结果可缓存复用（server 能力不随请求变化）。
 */
export async function probeRequestGzip(
  endpoint: string,
  token?: string,
  fetchImpl?: typeof fetch,
): Promise<boolean> {
  const f = fetchImpl ?? fetch.bind(globalThis);
  const url = `${normalizeEndpoint(endpoint)}/whoami`;
  const headers: Record<string, string> = {};
  if (token) headers["Authorization"] = `Bearer ${token}`;
  try {
    const res = await f(url, { method: "GET", headers });
    if (!res.ok) return false;
    const data = (await res.json()) as { requestGzip?: unknown };
    return data?.requestGzip === true;
  } catch {
    return false;
  }
}
