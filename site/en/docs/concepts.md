# Core Concepts

Four concepts make up Prism's entire data model.

## Session

**One session = one continuous observation.** For the Web SDK a session is one page visit; SPA route changes stay in the same session, a full reload opens a new segment. A session is stored as a directory:

```
recordings/<sessionId>/
  session.json          # { id, startedAt, endedAt?, source?, appId?, ... }
  windows.jsonl         # window lifecycle: shown / hidden / focus, with segmentId
  segments/<label>#<n>.jsonl   # one rrweb event stream per segment
  annotations.jsonl     # user annotations (session-level, separate from events)
```

## Segment

rrweb records DOM as an event stream. A window's "shown → hidden" span of events = **one segment**, named `<label>#<n>` (e.g. `web#0`, `main#1`). In multi-window apps each window has its own segments, aligned on a shared timeline via **absolute timestamps**.

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

On replay, the DOM frame and diagnostic signals render in lockstep on one timeline. This is the "prism splitting light" metaphor: one beam of user behavior refracts into a DOM stream and a signal stream.

## Source

Each session is tagged with a source; the console renders each on a differently colored lane:

| Source | Meaning | Lane color |
| --- | --- | --- |
| `self` | console self-recording | amber |
| `web` | Web SDK | teal |
| `tauri` | Tauri Plugin | lane-5 |

## Bundle contract

A session serializes to a `prism-session` bundle — a single JSON file that is the **sole contract for moving a session across processes or machines**:

```jsonc
{ "format": "prism-session", "version": 1,
  "session": {}, "windows": [], "segments": {}, "annotations": [] }
```

Three topologies share it: local file sharing, local server streaming, and offline-record-then-upload. Offline capture (`recordOffline` in the [Web SDK](./web)) buffers to IndexedDB, then `export`s a bundle to download or upload.
