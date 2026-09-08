# Quick Start

**What is Prism?** A local-first frontend observation platform. It records what users do in your app as a **replayable DOM snapshot stream**, and interleaves errors, console output, and network requests as diagnostic signals on the same timeline — when something breaks, you replay "what the user saw" while seeing "what the code did". All data stays on your own machine.

This page gets the minimal loop running from zero in about 5 minutes:

1. Start the console (receiver) → ✅ main window appears
2. Run the bundled sample app (SDK already wired) → ✅ page shows "recording"
3. Open the session in the console → ✅ session list shows a record
4. (Optional) integrate your own app

## Prerequisites

| Tool | Version | Needed for | Check |
| --- | --- | --- | --- |
| Node.js | ≥ 18 | all frontend parts | `node -v` |
| pnpm | ≥ 8 | package manager | `pnpm -v` (missing: `npm i -g pnpm`) |
| Rust toolchain | stable | step 1 only (console is a Tauri desktop app) | `rustc --version` (missing: install [rustup](https://rustup.rs)) |
| Browser | modern | open the sample app | — |

## Step 1: Start the console

The console is where sessions land and get replayed — a Tauri desktop app:

```sh
git clone https://github.com/zxc125/prism.git
cd prism
pnpm install
pnpm tauri dev
```

**✅ Check**: the Prism main window appears.

Open **Settings** (side navigation → Settings) and note two things for later:

- **Local server address**: default `http://127.0.0.1:1421`
- **Token** (optional): auth is off by default; once enabled, observed apps must send the same token

## Step 2: Run the sample app (zero code)

The repo ships [examples/web-demo](https://github.com/zxc125/prism/tree/main/examples/web-demo), a sample web app with the SDK already integrated — no code needed:

```sh
# new terminal, still at the repo root
pnpm dev:web-demo
```

Open `http://localhost:1422` in your browser.

**✅ Check**: the badge in the top-right corner shows "采集中" (recording) in green.

Click around — buttons, inputs, toggles. It's all being recorded.

## Step 3: Open the session in the console

Back in the Prism main window, open the **session browser**.

**✅ Check**: a session with source `web` and appId `web-demo` appears. Click it to replay: drag the playhead along the timeline while the DOM picture rebuilds, with error / console / network signals aligned to the exact moments they happened.

## Step 4 (optional): Integrate your own app

Install the SDK and call `init()` once at your app entry:

```sh
pnpm add @prism-obs/observer-sdk
```

```ts
import { init } from "@prism-obs/observer-sdk";

init({
  appId: "my-app",
  endpoint: "http://127.0.0.1:1421", // the address you noted in Settings
  // token: "…",                     // required if console auth is on
});
```

Reload your app and check the console — your app's session should be in the list. For more (recording profiles, offline capture, redaction, frameworks), see the [Web SDK](./web).

## Troubleshooting

| Symptom | Likely cause | Fix |
| --- | --- | --- |
| `pnpm tauri dev` fails with Rust / cargo errors | Rust toolchain missing | install [rustup](https://rustup.rs), retry |
| Sample badge shows "连接失败" (red) | console not running / wrong endpoint / token mismatch | confirm the main window is up; match the address in Settings; if auth is on, send the same token |
| Session list stays empty | endpoint or token mismatch | open DevTools → Network on the observed page, find the `/ingest` request to `127.0.0.1:1421` and read the status (401 = wrong token, unreachable = console down) |
| Startup fails with port in use | 1420 / 1421 / 1422 taken by another process | free the port, or change the server port in Settings and update the app's endpoint |

## Next steps

- Session / segment / signal model → [Core Concepts](./concepts)
- Full web integration: recording profiles, offline, redaction, frameworks → [Web SDK](./web)
- Tauri desktop apps with multiple windows → [Tauri Plugin](./tauri)
- Team / private cloud deployment → [Self-Hosting](./deploy)
