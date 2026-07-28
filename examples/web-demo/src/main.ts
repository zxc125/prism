import { init, type Controller } from "@prism/observer-sdk";

// 指向 console 的本地 HTTP server（端口/ token 见 console 设置页）。
// 若设置了 token，把同一 token 传进来。
const ENDPOINT = "http://127.0.0.1:1421";

const statusEl = document.getElementById("status")!;
document.getElementById("endpoint")!.textContent = ENDPOINT;

let ctrl: Controller | null = null;

init({
  appId: "web-demo",
  endpoint: ENDPOINT,
  env: "dev",
  release: "0.1.0",
})
  .then((c) => {
    ctrl = c;
    statusEl.textContent = "采集中";
    statusEl.style.background = "#8AB36A";
  })
  .catch((e) => {
    statusEl.textContent = "连接失败";
    statusEl.style.background = "#B5383A";
    console.error("[web-demo] init failed", e);
  });

// 暴露到 window 供按钮调用
declare global {
  interface Window {
    logThings: () => void;
    fetchThing: () => void;
    boom: () => void;
    rejectThing: () => void;
    stopSdk: () => void;
  }
}

window.logThings = () => {
  console.log("hello from web-demo", { time: Date.now(), nested: { a: 1 } });
  console.warn("这是一条 warn");
  console.error("这是一条 error（仍会被记录为信号）");
  const li = document.createElement("li");
  li.textContent = `log @ ${new Date().toLocaleTimeString()}`;
  document.getElementById("list")!.appendChild(li);
};

window.fetchThing = () => {
  fetch("https://httpbin.org/get?from=web-demo")
    .then((r) => r.json())
    .then(() => console.log("fetch done"))
    .catch(() => {});
};

window.boom = () => {
  // 同步抛出 -> window.onerror
  throw new Error("web-demo 故意抛错");
};

window.rejectThing = () => {
  // 未处理的 rejection -> unhandledrejection
  Promise.reject(new Error("web-demo 未捕获 Promise"));
};

window.stopSdk = () => {
  ctrl?.stop().then(() => {
    statusEl.textContent = "已停止";
    statusEl.style.background = "#888";
  });
};
