# Quick Start

Get running in 3 minutes: install the SDK → start the console → embed in your app → replay in the console.

## 1. Start the console (receiver)

The console is where sessions land and are replayed. Pick one:

- **Desktop app**: download a build from [GitHub](https://github.com/zxc125/prism) or run `pnpm tauri dev`. Open **Settings** and note the local server address (default `http://127.0.0.1:1421`) and optional token.
- **Single binary**: `observer-server --bind 127.0.0.1:8080 --data-dir ./recordings` (see [Self-Hosting](./deploy)).

## 2. Install the SDK in your web app

```sh
pnpm add @prism-obs/observer-sdk
```

Call `init()` once at your app entry:

```ts
import { init } from "@prism-obs/observer-sdk";

const ctrl = await init({
  appId: "my-app",
  endpoint: "http://127.0.0.1:1421",
  token: "<optional token>",
  env: "dev",
  release: "1.0.0",
});

await ctrl.stop(); // explicit stop (optional)
```

## 3. Trigger signals, replay in the console

Open your app, click around, fire a request, throw an error. The console **session browser** lists the session; open it to replay — DOM changes interleaved with error / console / network signals on one timeline.

## Next steps

- Session / segment / signal model → [Core Concepts](./concepts)
- Full web integration (offline, redaction, frameworks) → [Web SDK](./web)
- Tauri desktop apps → [Tauri Plugin](./tauri)
- Private cloud / team setup → [Self-Hosting](./deploy)
