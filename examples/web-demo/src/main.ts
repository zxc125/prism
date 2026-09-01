import { init, type Controller, type RecordingOptions } from "@prism-obs/observer-sdk";

// 指向 console 的本地 HTTP server（端口/ token 见 console 设置页）。
// 若设置了 token，把同一 token 传进来。
const ENDPOINT = "http://127.0.0.1:1421";

// ---- P14 场景 6：URL 参数控制录制配置（同页对比 full vs minimal+domBlocks）----
//   ?sim=1                        启动行情模拟（.quote-table 每 100ms 随机刷新）
//   ?profile=full|balanced|minimal  录制档位（缺省 full）
//   ?block=.quote-table           免录区 selector（逗号分隔多个）
const params = new URLSearchParams(location.search);
const profileParam = params.get("profile") ?? "full";
const domBlocks = (params.get("block") ?? "")
  .split(",")
  .map((s) => s.trim())
  .filter(Boolean);
const recording: RecordingOptions | undefined =
  profileParam === "full" && domBlocks.length === 0
    ? undefined
    : { profile: profileParam as NonNullable<RecordingOptions["profile"]>, domBlocks };

const statusEl = document.getElementById("status")!;
document.getElementById("endpoint")!.textContent = ENDPOINT;
document.getElementById("cfg")!.textContent = `recording = ${JSON.stringify(
  recording ?? { profile: "full" },
)}`;

let ctrl: Controller | null = null;

init({
  appId: "web-demo",
  endpoint: ENDPOINT,
  env: "dev",
  release: "0.2.0",
  recording,
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

// ---- 行情模拟：10 行 × 5 列报价表，每 100ms 随机刷 20 个格子 ----
// 模拟交易系统高频行情推送的核心压力：密集文本 mutation（rrweb mutation 事件的主要来源）。
function startQuoteSim(): void {
  document.getElementById("sim-section")!.hidden = false;
  const tbody = document.getElementById("quote-body")!;
  const cells: HTMLTableCellElement[] = [];
  for (let r = 0; r < 10; r++) {
    const tr = document.createElement("tr");
    const name = document.createElement("td");
    name.textContent = `SYM${1000 + r}`;
    name.className = "sym";
    tr.appendChild(name);
    for (let c = 0; c < 5; c++) {
      const td = document.createElement("td");
      td.textContent = (90 + Math.random() * 20).toFixed(2);
      tr.appendChild(td);
      cells.push(td);
    }
    tbody.appendChild(tr);
  }
  setInterval(() => {
    for (let k = 0; k < 20; k++) {
      const td = cells[Math.floor(Math.random() * cells.length)];
      td.textContent = (90 + Math.random() * 20).toFixed(2);
    }
  }, 100);
}

if (params.get("sim") === "1") startQuoteSim();
