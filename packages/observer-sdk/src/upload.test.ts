import { afterEach, describe, expect, it, vi } from "vitest";
import {
  probeRequestGzip,
  uploadBundle,
  type UploadOptions,
} from "./upload";
import type { Bundle } from "./bundle";

function makeBundle(): Bundle {
  return {
    format: "prism-session",
    version: 1,
    exportedAt: 1,
    session: { id: "local-1", startedAt: 1, source: "web", appId: "app" },
    windows: [],
    segments: { "main#1": [] },
    annotations: [],
  };
}

type FetchCall = { url: string; init: RequestInit };

/** 可控 fetch 桩：记录调用并按预设返回 Response（可异步抛错）。 */
function stubFetch(
  response: Response | ((call: FetchCall) => Response | Promise<Response>),
) {
  const calls: FetchCall[] = [];
  const impl = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const call = { url: String(input), init: (init ?? {}) as RequestInit };
    calls.push(call);
    return typeof response === "function" ? response(call) : response;
  });
  return { impl, calls };
}

function okResponse(payload: unknown, status = 200): Response {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

const baseOpts = (over: Partial<UploadOptions> = {}): UploadOptions => ({
  endpoint: "http://127.0.0.1:1421",
  ...over,
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("uploadBundle（P24 D1/D2）", () => {
  it("POST /sessions/import，body 为 bundle JSON 原文，返回观测端新 sessionId", async () => {
    const { impl, calls } = stubFetch(okResponse({ sessionId: "1720000000001" }));
    const result = await uploadBundle(makeBundle(), baseOpts({ fetchImpl: impl }));

    expect(result).toEqual({ sessionId: "1720000000001" });
    expect(calls).toHaveLength(1);
    expect(calls[0].url).toBe("http://127.0.0.1:1421/sessions/import");
    expect(calls[0].init.method).toBe("POST");
    expect(calls[0].init.body).toBe(JSON.stringify(makeBundle()));
    expect((calls[0].init.headers as Record<string, string>)["Content-Type"]).toBe(
      "application/json",
    );
  });

  it("带 token 时附 Authorization: Bearer 头；无 token 不带头", async () => {
    const { impl, calls } = stubFetch(okResponse({ sessionId: "1" }));
    await uploadBundle(makeBundle(), baseOpts({ token: "secret", fetchImpl: impl }));
    const h = calls[0].init.headers as Record<string, string>;
    expect(h["Authorization"]).toBe("Bearer secret");

    const { impl: impl2, calls: calls2 } = stubFetch(okResponse({ sessionId: "1" }));
    await uploadBundle(makeBundle(), baseOpts({ fetchImpl: impl2 }));
    expect(
      (calls2[0].init.headers as Record<string, string>)["Authorization"],
    ).toBeUndefined();
  });

  it("endpoint 尾斜杠归一（对齐 HttpSink）", async () => {
    const { impl, calls } = stubFetch(okResponse({ sessionId: "1" }));
    await uploadBundle(makeBundle(), baseOpts({ endpoint: "http://obs.local:8080/", fetchImpl: impl }));
    expect(calls[0].url).toBe("http://obs.local:8080/sessions/import");
  });

  it("非 2xx 抛错，带状态码 + 服务端 {error} 文案", async () => {
    const { impl } = stubFetch(okResponse({ error: "不支持的 bundle 版本：9" }, 400));
    await expect(
      uploadBundle(makeBundle(), baseOpts({ fetchImpl: impl })),
    ).rejects.toThrow("上传失败：HTTP 400 · 不支持的 bundle 版本：9");
  });

  it("非 JSON 错误体回退到状态码文案", async () => {
    const { impl } = stubFetch(new Response("boom", { status: 500 }));
    await expect(
      uploadBundle(makeBundle(), baseOpts({ fetchImpl: impl })),
    ).rejects.toThrow("上传失败：HTTP 500");
  });

  it("200 但缺 sessionId 时抛错（不静默成功）", async () => {
    const { impl } = stubFetch(okResponse({ unexpected: true }));
    await expect(
      uploadBundle(makeBundle(), baseOpts({ fetchImpl: impl })),
    ).rejects.toThrow("上传响应缺少 sessionId");
  });

  it("gzip: true 时请求体压缩并带 Content-Encoding，可解压还原", async () => {
    const { impl, calls } = stubFetch(okResponse({ sessionId: "1" }));
    await uploadBundle(makeBundle(), baseOpts({ gzip: true, fetchImpl: impl }));

    const h = calls[0].init.headers as Record<string, string>;
    expect(h["Content-Encoding"]).toBe("gzip");
    const body = calls[0].init.body as Uint8Array;
    expect(body).not.toBe(JSON.stringify(makeBundle()));
    const stream = new Blob([body]).stream().pipeThrough(new DecompressionStream("gzip"));
    const text = await new Response(stream).text();
    expect(text).toBe(JSON.stringify(makeBundle()));
  });
});

describe("probeRequestGzip（P24 D5 能力协商）", () => {
  it("whoami requestGzip === true 时返回 true，带 token 时附 Bearer 头", async () => {
    const { impl, calls } = stubFetch(okResponse({ multiTenant: false, requestGzip: true }));
    const result = await probeRequestGzip("http://127.0.0.1:1421/", "secret", impl);
    expect(result).toBe(true);
    expect(calls[0].url).toBe("http://127.0.0.1:1421/whoami");
    expect(calls[0].init.method).toBe("GET");
    expect((calls[0].init.headers as Record<string, string>)["Authorization"]).toBe(
      "Bearer secret",
    );
  });

  it("老 server 无 requestGzip 字段时返回 false", async () => {
    const { impl } = stubFetch(okResponse({ multiTenant: false }));
    expect(await probeRequestGzip("http://127.0.0.1:1421", undefined, impl)).toBe(false);
  });

  it("非 2xx / 网络错误均回退 false（不抛错）", async () => {
    const { impl } = stubFetch(new Response("{}", { status: 503 }));
    expect(await probeRequestGzip("http://127.0.0.1:1421", undefined, impl)).toBe(false);

    const { impl: impl2 } = stubFetch(async () => {
      throw new Error("network down");
    });
    expect(await probeRequestGzip("http://127.0.0.1:1421", undefined, impl2)).toBe(false);
  });
});
