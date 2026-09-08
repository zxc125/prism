# Core Concepts

Read this page once before your first run and you'll understand Prism's entire data model: sessions, segments, interleaved signals, sources, bundles — and where recording profiles fit in. Come back whenever a term is unclear.

## First: it records structure, not video

Prism's screen capture is built on [rrweb](https://github.com/rrweb-io/rrweb): instead of capturing pixels, it **serializes the DOM tree and its changes into a structured event stream** — a full snapshot first, then only deltas (this node changed, the mouse moved, this was typed). Replay rebuilds the picture inside a sandboxed iframe from that stream.

Three consequences that differ fundamentally from screen recording:

- **Small and searchable**: structured JSON, not video frames;
- **Privacy-controllable**: sensitive regions can be declared as "blocked areas" via CSS selectors and never enter the stream;
- **Signals share the timeline**: because everything is structured events, errors and requests align precisely with the picture (see the interleaved event model below).

The trade-off: rrweb records DOM-declared styles only, not the browser canvas default color (dark pages beware — see the [background color note in the Web SDK](./web#the-observed-page-needs-an-explicit-background-color)).

## Session

**One session = one continuous observation.** For the Web SDK a session is one page visit; SPA route changes stay in the same session, a full reload opens a new one. A session is stored as a directory:

```
recordings/<sessionId>/
  session.json          # { id, startedAt, endedAt?, source?, appId?, ... }
  windows.jsonl         # window lifecycle: shown / hidden / focus, with segmentId
  segments/<label>#<n>.jsonl   # one rrweb event stream per segment
  annotations.jsonl     # user annotations (session-level, separate from events)
```

## Segment

One window's continuous event stream between "shown → hidden" = **one segment**, named `<label>#<n>` (e.g. `web#0`, `main#1`).

In a web single-page app there is usually one implicit "window", so a segment is roughly "one continuous stretch after a reload". In **multi-window apps** (say a Tauri desktop app with 3 windows) each window has its own segments; all windows share the wall clock, events carry absolute timestamps, and replay drives every segment on one master timeline according to its shown/hidden span — hence the multi-lane timeline.

## Interleaved event model (type:6 signals)

Prism doesn't store error / console / network as separate logs. It wraps them as rrweb plugin events (`type: 6`), **interleaved into the same event stream** as the DOM, sharing one timeline:

```jsonc
{
  "type": 6,
  "timestamp": 1754000000000,
  "data": {
    "plugin": "network",   // error | console | network
    "payload": { "url": "/api/order", "method": "POST", "status": 500, "duration": 42 }
  }
}
```

On replay, the picture and the diagnostic signals render in lockstep — you see the button click and, at the same instant, the failing request and the thrown error. This is the "prism splitting light" metaphor: one beam of user behavior refracts into a DOM stream and a signal stream.

## Recording profiles

High-frequency pages (market-data ticks, polling re-renders, dense animation) generate huge event volumes. The SDK's `recording` option offers `full` (default) / `balanced` (throttled high-frequency interactions) / `minimal` (key interactions only), plus `domBlocks` blocked areas (matched elements and subtrees never enter the stream; replay shows a placeholder). Details in [Web SDK · recording](./web#recording).

## Source

Each session is tagged with a source; the console renders each on a differently colored lane:

| Source | Meaning | Lane color |
| --- | --- | --- |
| `self` | console self-recording | amber |
| `web` | Web SDK | teal |
| `tauri` | Tauri Plugin | green |

## Bundle contract

A session serializes to a `prism-session` bundle — **one JSON file containing the whole session** (metadata + window lifecycle + all event streams + annotations) and the **sole contract for moving a session across processes or machines**:

```jsonc
{ "format": "prism-session", "version": 1,
  "session": {}, "windows": [], "segments": {}, "annotations": [] }
```

Three topologies share it: local file sharing, local server streaming, and offline-record-then-upload. Offline capture (`recordOffline` in the [Web SDK](./web)) buffers to browser IndexedDB, then `export`s a bundle to download or upload.
