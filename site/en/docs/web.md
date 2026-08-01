# Web SDK

`@prism-obs/observer-sdk` — embed in any web app, record DOM + diagnostic signals, report to the console.

## Install

```sh
pnpm add @prism-obs/observer-sdk
```

## Realtime: `init()`

Call once to start rrweb recording + signal hooks, reporting via `HttpSink` to the console's local server.

```ts
import { init } from "@prism-obs/observer-sdk";

const ctrl = await init({
  appId: "my-app",
  endpoint: "http://127.0.0.1:1421",
  token: "<optional token>",
  env: "dev",
  release: "1.0.0",
  label: "web",        // optional, segment label, default "web"
  signals: "all",      // optional, default all on
});

await ctrl.stop(); // explicit stop (optional)
```

- **A session = one page visit**; SPA routes are continuous, a full reload opens a new segment.
- On `beforeunload`, a `sendBeacon` best-effort flush fires automatically.
- `signals` can be partial: `{ error: true, console: true, network: false }`.

## Offline capture: `recordOffline()`

Records to IndexedDB without needing the console online; later `export` a `prism-session` bundle to download or upload.

```ts
import { recordOffline } from "@prism-obs/observer-sdk";

const ctrl = await recordOffline({ appId: "my-app", release: "1.0.0" });
// ... user interactions recorded ...
await ctrl.download();        // export + trigger browser download
const sessions = await ctrl.list();
await ctrl.clear();
```

`OfflineController`: `stop()`, `export(id?)`, `download(id?, filename?, redactOpts?)`, `list()`, `clear(id?)`, `destroy()`.

> Events buffer (~1s flush); a sudden page close may lose the last <1s. Call `stop()` to flush cleanly. Recorded sessions are recoverable via `list()`.

## Redaction: `redact()`

Strip or scrub PII (network body, headers, tokens, emails) before exporting or sharing a bundle.

```ts
import { redact } from "@prism-obs/observer-sdk";

const clean = redact(data, {
  stripNetworkBody: true,
  stripNetworkHeaders: true,
  scrubbers: [/Bearer\s+[\w.-]+/g, /[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+/g],
});
```

`download()` / `export()` accept `redactOpts` directly. Always redact before uploading or sharing.

## Diagnostic signals

Three signal kinds interleave into the event stream (`type: 6`), sharing the DOM timeline:

| Signal | Hook | Semantics |
| --- | --- | --- |
| `error` | `window.onerror` + `unhandledrejection` | sync throws and uncaught promises |
| `console` | `console.{log,warn,error,info,debug}` | args serialized (Error → struct, Node → summary, cycles truncated) |
| `network` | `fetch` + `XMLHttpRequest` | url / method / status / duration; error on failure |

## ⚠️ Set an explicit background color

rrweb records DOM styles only, **not the canvas's default colors**. If your page relies on `color-scheme: light dark` for defaults, the replay iframe is transparent and light text may land on a light player background and vanish. Give `html, body` explicit `background` and `color`:

```css
html, body { background: #fff; color: #111; }
```

## Framework integration

Call once at the app root:

::: code-group

```ts [Vue — main.ts]
import { init } from "@prism-obs/observer-sdk";
init({ appId: "my-app", endpoint: "http://127.0.0.1:1421" });
```

```ts [React — index.tsx]
import { init } from "@prism-obs/observer-sdk";
init({ appId: "my-app", endpoint: "http://127.0.0.1:1421" });
```

:::

Full runnable sample: [`examples/web-demo`](https://github.com/zxc125/prism/tree/main/examples/web-demo).
