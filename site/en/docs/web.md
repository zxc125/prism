# Web SDK

`@prism-obs/observer-sdk` — embed in any web app, record DOM + diagnostic signals, report to the console.

> **Applies to**: `@prism-obs/observer-sdk` **0.2.x**. This page is updated together with API changes.

**In this page**: install the SDK → minimal integration → configure recording profiles / offline capture / redaction as needed. Prerequisite: the console is running (see [Quick Start](./quickstart)).

## Install

```sh
pnpm add @prism-obs/observer-sdk
```

## Quick start: `init()`

Call once at your app entry to start rrweb recording + signal hooks, reporting over HTTP to the console:

```ts
import { init } from "@prism-obs/observer-sdk";

const ctrl = await init({
  appId: "my-app",                       // required: app identifier
  endpoint: "http://127.0.0.1:1421",     // required: console local server (see Settings)
});

// ctrl.stop() for an explicit stop; a sendBeacon flush fires on unload
```

**✅ Check**: DevTools → Network on the observed page shows `/ingest` requests to the endpoint; the console session browser lists your app.

### Full `init()` options

| Option | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `appId` | `string` | ✅ | — | app identifier, reported with the session |
| `endpoint` | `string` | ✅ | — | console local HTTP server, e.g. `http://127.0.0.1:1421` |
| `token` | `string` | ➖ | — | local auth token; required when console auth is on |
| `env` | `string` | ➖ | — | environment tag (`dev` / `staging` / `prod`, your choice) |
| `release` | `string` | ➖ | — | release tag — "which version broke" during replay |
| `label` | `string` | ➖ | `"web"` | segment label. SPA routes stay in one segment; a full reload opens a new one |
| `signals` | `SignalSet` | ➖ | `"all"` | signal switches: `"all"`, or partial like `{ error: true, console: true, network: false }` |
| `recording` | `RecordingOptions` | ➖ | — | recording profile (below); omitted = full recording |
| `meta` | `object` | ➖ | — | extra fields forwarded into session meta (user id, order number…) |

Behavior notes:

- **A session = one page visit**; SPA routes are continuous, a full reload opens a new segment.
- On `beforeunload` a `sendBeacon` best-effort flush fires automatically.

## Recording

High-frequency pages (market-data ticks, polling re-renders, dense animation) generate huge event volumes. The `recording` option controls two things: the **profile** (which capture channels get throttled) and **blocked areas** (which DOM is never recorded).

```ts
init({
  appId: "my-app",
  endpoint: "http://127.0.0.1:1421",
  recording: {
    profile: "minimal",                     // default "full"
    domBlocks: [".quote-table", ".ad-banner"], // blocked-area CSS selectors, default none
  },
});
```

`RecordingOptions`:

| Option | Type | Notes |
| --- | --- | --- |
| `profile` | `"full" \| "balanced" \| "minimal"` | recording profile, default `full` |
| `domBlocks` | `string[]` | blocked areas: CSS selector list. DOM changes inside matched elements and subtrees **never enter the stream**; replay shows a placeholder |

How the profiles differ:

| Profile | mousemove | scroll | Inputs | Other mouse/touch | Minor nodes (script/comments/head meta…) | For |
| --- | --- | --- | --- | --- | --- | --- |
| `full` (default) | full (50ms aggregate) | full | full | full | kept | default, zero diff with older versions |
| `balanced` | 100ms aggregate | 200ms throttle | final value only | full | pruned | general pages, less volume |
| `minimal` | off | 500ms throttle | final value only | Click / TouchStart / TouchEnd only | fully pruned | "what did the user click" only |

::: warning Profiles don't throttle mutations
All three profiles throttle **interaction channels and minor nodes** only — **none of them reduce DOM mutation volume**. For tick-push or fast-polling pages, pair the profile with `domBlocks` — excluding the hot region from recording (it shows as a placeholder in replay) is what actually cuts the volume.
:::

Want to see the difference? The repo sample [examples/web-demo](https://github.com/zxc125/prism/tree/main/examples/web-demo) takes URL params: `?sim=1` starts a market-data simulation, `?profile=minimal&block=.quote-table` applies a tuning config in one go.

## Offline capture: `recordOffline()`

Records to browser IndexedDB without needing the console online; later `export` a `prism-session` bundle to download or upload.

```ts
import { recordOffline } from "@prism-obs/observer-sdk";

const ctrl = await recordOffline({ appId: "my-app", release: "1.0.0" });

// ... user interactions recorded ...

// export the current session as a bundle (auto-stops)
const bundle = await ctrl.export();
// or trigger a browser download directly
await ctrl.download();

// list all offline sessions on this machine
const sessions = await ctrl.list();
// clean up
await ctrl.clear();
```

Options match `init()` (`appId` required; `env` / `release` / `label` / `signals` / `recording` / `meta` optional) except there is **no** `endpoint` / `token` — offline means offline.

`OfflineController` API:

| Member | Notes |
| --- | --- |
| `sessionId` | current session id (readonly property) |
| `stop()` | explicit stop, flushes buffered events, returns the session id |
| `export(id?, redactOpts?)` | serialize to a bundle (defaults to the current session; exporting it auto-stops) |
| `download(id?, filename?, redactOpts?)` | export + trigger a browser download |
| `list()` | all offline session metas on this machine (newest first) |
| `clear(id?)` | delete one session; no id = wipe all |
| `destroy()` | destroy the controller: removes the unload hook and stops recording (keeps data) |

> Events buffer (~1s flush); a sudden page close may lose the last <1s. Call `stop()` to flush cleanly. Recorded sessions are recoverable via `list()`.

## Redaction: `redact()`

Strip or scrub PII (network body, headers, tokens, emails) before exporting or sharing a bundle.

```ts
import { redact } from "@prism-obs/observer-sdk";

const clean = redact(data, {
  stripNetworkBody: true,      // default true — the biggest PII surface
  stripNetworkHeaders: true,   // default true
  dropNetwork: false,          // drop network signal events entirely
  dropConsole: false,          // drop console signal events entirely
  scrubbers: [                 // regex scrubbers, matches replaced with [REDACTED]
    /Bearer\s+[\w.-]+/g,
    /[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+/g,
  ],
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

## The observed page needs an explicit background color

::: warning
rrweb records DOM styles only, **not the canvas's default colors**. If your page relies on `color-scheme: light dark` for defaults, the replay iframe is transparent and light text may land on the player's light background and vanish.
:::

Give `html, body` explicit `background` and `color`:

```css
html,
body {
  background: #fff;
  color: #111;
}
```

## Framework integration

Starting from scratch:

```sh
npm create vite@latest my-app -- --template vue-ts   # or react-ts / vanilla-ts
cd my-app
pnpm install
pnpm add @prism-obs/observer-sdk
```

Call `init()` once at the app root:

::: code-group

```ts [Vue — main.ts]
import { init } from "@prism-obs/observer-sdk";
init({ appId: "my-app", endpoint: "http://127.0.0.1:1421" });
// createApp(App).mount('#app')
```

```ts [React — index.tsx]
import { init } from "@prism-obs/observer-sdk";
init({ appId: "my-app", endpoint: "http://127.0.0.1:1421" });
// ReactDOM.createRoot(...).render(...)
```

:::

::: warning SSR
`init()` touches `window` / `navigator` / `document` and **must run in the browser only**. In Nuxt / Next put it in a client lifecycle (Vue `onMounted`, React `useEffect`) or guard it with `import.meta.client` / `typeof window !== "undefined"`.
:::

## Troubleshooting

| Symptom | Likely cause | Fix |
| --- | --- | --- |
| `init()` rejects / page shows "connection failed" | wrong endpoint or console down | confirm the console window is up and the address matches Settings; 401 = token mismatch |
| Blank replay / invisible text | no explicit background color | see the [background color note](#the-observed-page-needs-an-explicit-background-color) |
| Event volume / session size too big | high-frequency interactions or DOM re-renders | add `recording`; pair with `domBlocks` for hot DOM regions (see [recording](#recording)) |
| SSR build fails with `window is not defined` | `init()` called on the server | see the SSR warning above — move to a client lifecycle or add a guard |

## Full example

Full runnable sample: [`examples/web-demo`](https://github.com/zxc125/prism/tree/main/examples/web-demo) (market simulation + three-profile comparison; see its README).
