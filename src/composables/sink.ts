// 传输抽象与外部 Sink 实现下沉到 observer-sdk 包；P16 起 TauriSink 下沉到
// @prism-obs/observer-tauri 包（console self-obs 与外部应用 Local 模式共用同一实现），
// 此处仅 re-export。TauriSink 的行为说明见包内 src/sink.ts。
export type { LifecycleEvent, RREvent, SessionMeta, Sink } from "@prism-obs/observer-sdk";
export { HttpSink, IndexedDBSink } from "@prism-obs/observer-sdk";
export { TauriSink } from "@prism-obs/observer-tauri";
export type { HttpSinkOptions } from "@prism-obs/observer-sdk";
