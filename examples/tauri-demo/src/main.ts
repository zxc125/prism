import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { parseBundle, buildBundle, probeRequestGzip, uploadBundle } from "@prism-obs/observer-sdk";
import { initTauri, type TauriController } from "@prism-obs/observer-tauri";

// 双模式开关（P16 D10）：`VITE_OBSERVER_MODE=local pnpm tauri dev` 进入本地落盘模式
//（Rust 同读此变量切 Mode::Local，单一来源）；缺省 = Remote 上报 console。
const LOCAL = import.meta.env.VITE_OBSERVER_MODE === "local";

// 手动档门控开关（P17 D7）：`VITE_OBSERVER_GATING=manual pnpm tauri dev` 时，
// `recording-session{active}` 广播与挂载兜底不再自动开段，段边界由下方
// installGatingPolicy 的宿主策略（交互开段/判闲停段/空闲异常兜底）驱动；缺省常录。
const GATING = import.meta.env.VITE_OBSERVER_GATING === "manual";

// 上报目标可热切：localStorage 持久化 + 应用内配置 UI（默认本地 server，可切云端 observer-server）。
const STORAGE_KEY = "observer-tauri-demo";
const DEFAULT_ENDPOINT = "http://127.0.0.1:1421";

interface DemoConfig {
  endpoint: string;
  token: string;
}

function loadConfig(): DemoConfig {
  // URL 参数优先（浏览器直开 / `?endpoint=...&token=...`），命中即落 localStorage 持久化
  const params = new URLSearchParams(window.location.search);
  const ep = params.get("endpoint");
  if (ep) {
    const cfg: DemoConfig = { endpoint: ep, token: params.get("token") || "" };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(cfg));
    return cfg;
  }
  // 其次 localStorage
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return { endpoint: DEFAULT_ENDPOINT, token: "", ...JSON.parse(raw) };
  } catch {
    /* ignore */
  }
  return { endpoint: DEFAULT_ENDPOINT, token: "" };
}

const cfg = loadConfig();
const ENDPOINT = cfg.endpoint;

const label = getCurrentWebviewWindow().label;
const isMain = !window.location.hash || window.location.hash === "#/";

document.getElementById("win")!.textContent = label;
document.getElementById("endpoint")!.textContent = LOCAL
  ? `本地落盘 · appDataDir/recordings/（上传目标 ${ENDPOINT || "未配置"}）`
  : ENDPOINT;

// 连接配置 UI（P24 起 Local 模式同样显示：endpoint/token = 一键上传目标）。
// 回填当前值，「应用」= 存 localStorage + reload（重新 initTauri 到新 endpoint）。
const configEl = document.querySelector(".config") as HTMLDetailsElement | null;
if (configEl) {
  const epInput = document.getElementById("cfg-endpoint") as HTMLInputElement;
  const tkInput = document.getElementById("cfg-token") as HTMLInputElement;
  epInput.value = cfg.endpoint;
  tkInput.value = cfg.token;
  document.getElementById("cfg-apply")!.addEventListener("click", () => {
    const ep = epInput.value.trim();
    if (!ep) {
      alert("endpoint 不能为空");
      return;
    }
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ endpoint: ep, token: tkInput.value.trim() }),
    );
    location.reload();
  });
}

const dotEl = document.getElementById("dot")!;
const statusEl = document.getElementById("status")!;

initTauri({
  appId: "tauri-demo",
  mode: LOCAL ? "local" : undefined,
  endpoint: LOCAL ? undefined : ENDPOINT,
  token: LOCAL ? undefined : cfg.token || undefined,
  env: "dev",
  release: "0.1.0",
  autoStart: isMain,
  gating: GATING ? "manual" : undefined,
})
  .then((ctrl: TauriController) => {
    dotEl.classList.add("on");
    statusEl.textContent = GATING
      ? "手动档 · 等待交互开段（空闲异常自动兜底）"
      : isMain
        ? LOCAL
          ? "采集中 · 主窗口（本地落盘）"
          : "采集中 · 主窗口"
        : "采集中 · 子窗口";
    (window as any).__ctrl = ctrl;
    if (GATING) installGatingPolicy(ctrl);
  })
  .catch((e: unknown) => {
    dotEl.classList.add("err");
    statusEl.textContent = "连接失败";
    console.error("[tauri-demo] init failed", e);
  });

declare global {
  interface Window {
    openChild: () => void;
    logThings: () => void;
    fetchThing: () => void;
    boom: () => void;
    exportBundle: () => Promise<void>;
    uploadToConsole: () => Promise<void>;
    uploadBundleFile: () => Promise<void>;
    stopObs: () => void;
  }
}

window.openChild = () => {
  invoke("open_window", { route: `/child/${Date.now()}` }).catch((e) =>
    console.error("[tauri-demo] open_window failed", e),
  );
};

window.logThings = () => {
  console.log("hello from tauri-demo", { time: Date.now(), nested: { a: 1 } });
  console.warn("这是一条 warn");
  console.error("这是一条 error（仍会被记录为信号）");
  const li = document.createElement("li");
  li.textContent = `log @ ${new Date().toLocaleTimeString()}`;
  document.getElementById("list")!.appendChild(li);
};

window.fetchThing = () => {
  fetch("https://httpbin.org/get?from=tauri-demo")
    .then((r) => r.json())
    .then(() => console.log("fetch done"))
    .catch(() => {});
};

window.boom = () => {
  throw new Error("tauri-demo 故意抛错");
};

window.exportBundle = async () => {
  const ctrl = (window as any).__ctrl as TauriController | undefined;
  if (!ctrl) {
    alert("观测未初始化");
    return;
  }
  try {
    const sessions = (await ctrl.listSessions()) as Array<{
      id: string;
      startedAt?: number;
    }>;
    if (!sessions.length) {
      alert("本地暂无会话");
      return;
    }
    const latest = [...sessions].sort(
      (a, b) => (b.startedAt ?? 0) - (a.startedAt ?? 0),
    )[0];
    const bundle = await ctrl.exportSession(latest.id);
    const blob = new Blob([JSON.stringify(bundle, null, 2)], {
      type: "application/json",
    });
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = `${latest.id}.prism-session.json`;
    a.click();
    URL.revokeObjectURL(a.href);
    console.log("[tauri-demo] 导出完成", latest.id, `共 ${sessions.length} 个会话`);
  } catch (e) {
    console.error("[tauri-demo] 导出失败", e);
    alert(`导出失败：${e}`);
  }
};

// 网络类报错（fetch 连接失败）翻译成可行动的提示，避免只显原始异常（两个上传入口共用）。
const friendlyError = (raw: string): string =>
  /failed to fetch|networkerror|load failed|could not connect/i.test(raw)
    ? "无法连接上传目标（检查地址/端口与 server 是否启动；console 默认 http://127.0.0.1:1421）"
    : raw;

window.uploadToConsole = async () => {
  const ctrl = (window as any).__ctrl as TauriController | undefined;
  if (!ctrl) {
    alert("观测未初始化");
    return;
  }
  if (!LOCAL) {
    alert("仅 Local 模式可上传（Remote 数据已实时在观测端）");
    return;
  }
  if (!cfg.endpoint) {
    alert("先在「连接配置」里填上传目标（观测端 console / server 地址）");
    return;
  }
  const btn = document.getElementById("upload-btn") as HTMLButtonElement;
  const head = document.getElementById("upload-status") as HTMLElement;
  const list = document.getElementById("upload-list") as HTMLUListElement;
  btn.disabled = true;
  list.innerHTML = "";
  head.hidden = false;

  // 注意：不用 demo 旧的 ?token= URL 注入（token 会进浏览器历史），只走配置 UI。
  try {
    const results = await ctrl.uploadSessions(undefined, {
      endpoint: cfg.endpoint,
      token: cfg.token || undefined,
      onProgress: ({ index, total }) => {
        head.textContent = `上传中 ${index + 1}/${total}`;
      },
      onResult: (r) => {
        const li = document.createElement("li");
        if (r.ok) {
          li.innerHTML = `<span class="ul-ok">✓ ${r.sessionId} → 已导入（观测端 id ${r.remoteId}）</span>`;
        } else {
          li.innerHTML = `<span class="ul-fail">✗ ${r.sessionId} → 失败：${friendlyError(r.error ?? String(r.error))}</span>`;
        }
        list.appendChild(li);
      },
    });
    const ok = results.filter((r) => r.ok).length;
    const fail = results.length - ok;
    head.textContent =
      ok === 0 && fail > 0
        ? `上传失败 ${fail} 个：请检查上传目标地址/端口（当前 ${cfg.endpoint}；console 默认 http://127.0.0.1:1421）`
        : `导入完成：成功 ${ok} · 失败 ${fail}（重复点击会在观测端产生重复会话）`;
    if (!results.length) head.textContent = "本地暂无会话";
    console.log("[tauri-demo] 上传完成", { ok, fail });
  } catch (e) {
    head.textContent = "上传失败";
    console.error("[tauri-demo] 上传失败", e);
  } finally {
    btn.disabled = false;
  }
};

// 选择 bundle 文件上传（P24 补充入口）：文件 → parseBundle 预校验 → uploadBundle。
// 用 <input type=file> 读内容（无需 plugin-dialog / capabilities），浏览器与 Tauri webview 通用。
window.uploadBundleFile = async () => {
  if (!cfg.endpoint) {
    alert("先在「连接配置」里填上传目标（观测端 console / server 地址）");
    return;
  }
  const btn = document.getElementById("upload-file-btn") as HTMLButtonElement;
  const head = document.getElementById("upload-status") as HTMLElement;
  const list = document.getElementById("upload-list") as HTMLUListElement;
  const input = document.createElement("input");
  input.type = "file";
  input.accept = ".json,application/json";
  input.onchange = async () => {
    const file = input.files?.[0];
    if (!file) return;
    btn.disabled = true;
    head.hidden = false;
    head.textContent = `上传中：${file.name}`;
    const failLine = (msg: string) => {
      list.innerHTML = "";
      const li = document.createElement("li");
      li.innerHTML = `<span class="ul-fail">✗ ${file.name} → 失败：${msg}</span>`;
      list.appendChild(li);
    };
    try {
      const parsed = parseBundle(JSON.parse(await file.text()));
      if (!parsed.ok || !parsed.data) {
        failLine(`不是有效的会话 bundle：${parsed.error ?? "未知原因"}`);
        return;
      }
      const useGzip = await probeRequestGzip(cfg.endpoint, cfg.token || undefined);
      const { sessionId } = await uploadBundle(buildBundle(parsed.data), {
        endpoint: cfg.endpoint,
        token: cfg.token || undefined,
        gzip: useGzip,
      });
      list.innerHTML = "";
      const li = document.createElement("li");
      li.innerHTML = `<span class="ul-ok">✓ ${file.name} → 已导入（观测端 id ${sessionId}）</span>`;
      list.appendChild(li);
      head.textContent = "导入完成（重复上传会在观测端产生重复会话）";
      console.log("[tauri-demo] 文件上传完成", file.name, sessionId);
    } catch (e) {
      failLine(e instanceof SyntaxError ? "文件不是合法 JSON" : friendlyError(String(e)));
      head.textContent = "上传失败";
      console.error("[tauri-demo] 文件上传失败", e);
    } finally {
      btn.disabled = false;
    }
  };
  input.click();
};

window.stopObs = () => {
  // 触发插件 stop_session：广播 recording-session{active:false}，各窗口停段；
  // Local 模式同时由 Rust 写 endedAt，Remote 模式由主窗口收广播后 endSession
  invoke("plugin:observer|stop_session").catch((e) =>
    console.error("[tauri-demo] stop failed", e),
  );
  dotEl.classList.remove("on");
  statusEl.textContent = "已停止";
};

/**
 * 手动档门控策略（P17，仅演示）：机制（controller.startSegment/stopSegment/signal/active）
 * 由上游提供，策略留宿主——交互开段、30s 判闲停段、空闲期异常兜底注入。
 * 生产宿主（bond/frontend）按自身产品语义实现，此函数不进包。
 */
function installGatingPolicy(ctrl: TauriController): void {
  const IDLE_STOP_MS = 30_000;
  let lastActiveAt = Date.now();
  // 队列而非单值：空闲期连续多次 error 都不丢
  let errorPending: unknown[] = [];

  // 空闲异常兜底：开段后注入（signal 段未活跃时丢弃，须先 startSegment）
  const flushPendingError = (): void => {
    for (const payload of errorPending.splice(0)) ctrl.signal("error", payload);
  };

  const onInteraction = (): void => {
    lastActiveAt = Date.now();
    if (!ctrl.active) {
      void ctrl.startSegment().then(() => {
        statusEl.textContent = "手动档 · 采集中（交互驱动）";
        flushPendingError();
      });
    }
  };
  for (const type of ["pointerdown", "keydown", "wheel", "touchstart"] as const) {
    window.addEventListener(type, onInteraction, { capture: true, passive: true });
  }

  // 段活跃时信号 hook 已捕获 error，这里只兜底空闲期异常
  window.addEventListener("error", (ev) => {
    if (ctrl.active) return;
    errorPending.push({
      message: ev.message,
      source: ev.filename,
      lineno: ev.lineno,
      stack: ev.error?.stack,
      kind: "onerror",
    });
    void ctrl.startSegment().then(flushPendingError);
  });

  // 判闲节拍：空闲超阈值停段，下次交互开新段（每段以全量快照开头，可独立回放）
  window.setInterval(() => {
    if (ctrl.active && Date.now() - lastActiveAt > IDLE_STOP_MS) {
      void ctrl.stopSegment();
      statusEl.textContent = "手动档 · 段空闲（交互续录）";
    }
  }, 1_000);
}
