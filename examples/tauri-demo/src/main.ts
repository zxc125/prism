import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { initTauri, type TauriController } from "@prism-obs/observer-tauri";

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
document.getElementById("endpoint")!.textContent = ENDPOINT;

// 配置 UI：回填当前值，「应用」= 存 localStorage + reload（重新 initTauri 到新 endpoint）
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

const dotEl = document.getElementById("dot")!;
const statusEl = document.getElementById("status")!;

initTauri({
  appId: "tauri-demo",
  endpoint: ENDPOINT,
  token: cfg.token || undefined,
  env: "dev",
  release: "0.1.0",
  autoStart: isMain,
})
  .then((ctrl: TauriController) => {
    dotEl.classList.add("on");
    statusEl.textContent = isMain ? "采集中 · 主窗口" : "采集中 · 子窗口";
    (window as any).__ctrl = ctrl;
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

window.stopObs = () => {
  // 触发插件 stop_session：广播 recording-session{active:false}，各窗口停段、主窗口 endSession
  invoke("plugin:observer|stop_session").catch((e) =>
    console.error("[tauri-demo] stop failed", e),
  );
  dotEl.classList.remove("on");
  statusEl.textContent = "已停止";
};
