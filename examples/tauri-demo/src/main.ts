import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { initTauri, type TauriController } from "@rrweb-demo/observer-tauri";

// 指向 console 的本地 HTTP server（端口/token 见 console 设置页）。
const ENDPOINT = "http://127.0.0.1:1421";

const label = getCurrentWebviewWindow().label;
const isMain = !window.location.hash || window.location.hash === "#/";

document.getElementById("win")!.textContent = label;
document.getElementById("endpoint")!.textContent = ENDPOINT;

const dotEl = document.getElementById("dot")!;
const statusEl = document.getElementById("status")!;

initTauri({
  appId: "tauri-demo",
  endpoint: ENDPOINT,
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
