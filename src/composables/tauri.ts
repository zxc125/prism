/**
 * Tauri 抽象层（P10）：把对 Tauri API 的直接依赖收敛到一处，按 `isTauri()` 在
 * 桌面 / 浏览器两环境下正确分派。`@tauri-apps/api` 改动态 import，浏览器构建
 * 不打包死代码。
 *
 * 见 docs/架构/P10-console2.0重设计（方案）.md §7.1。
 */

/** 运行时检测是否在 Tauri 桌面 webview 内。 */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * 打开一个路由对应的窗口/标签。
 * - Tauri：invoke `open_window`（单实例聚焦 / 多窗口协调）。
 * - 浏览器：同标签内 router push（默认）或新标签 window.open。
 */
export async function openRoute(route: string, opts: { newTab?: boolean } = {}): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("open_window", { route });
    return;
  }
  // 浏览器：新标签 or 同标签路由跳转
  if (opts.newTab) {
    window.open(`#${route}`, "_blank");
  } else {
    // 同标签跳转：改 hash 触发 vue-router
    window.location.hash = route;
  }
}

/**
 * 选一个 bundle 文件并返回内容。
 * - Tauri：plugin-dialog 原生选择器 + Rust 读文件（避免大 JSON 过 IPC 时直接拿 path）。
 *   返回 { path } 给 Backend.importBundlePath（Rust 侧再读）。
 * - 浏览器：`<input type="file">` + FileReader，直接返回 { content }。
 */
export async function pickBundleFile(): Promise<
  { path: string; name: string } | { content: string; name: string } | null
> {
  if (isTauri()) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const path = await open({
      multiple: false,
      filters: [{ name: "会话 bundle", extensions: ["json"] }],
    });
    if (typeof path !== "string") return null;
    return { path, name: path.split(/[\\/]/).pop() ?? path };
  }
  // 浏览器：<input type=file>
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json,application/json";
    input.onchange = () => {
      const file = input.files?.[0];
      if (!file) return resolve(null);
      const reader = new FileReader();
      reader.onload = () =>
        resolve({ content: String(reader.result ?? ""), name: file.name });
      reader.onerror = () => resolve(null);
      reader.readAsText(file);
    };
    input.click();
  });
}

/**
 * 取当前 webview 窗口 label（仅 Tauri 有意义）。浏览器返回 "browser"。
 * useRecorder 等需要按 label 区分窗口的逻辑用此守卫。
 */
export async function currentWindowLabel(): Promise<string> {
  if (!isTauri()) return "browser";
  const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
  return getCurrentWebviewWindow().label;
}

/**
 * 监听窗口聚焦变化（仅 Tauri）。浏览器无对应事件，返回 no-op。
 */
export async function onWindowFocus(cb: (focused: boolean) => void): Promise<() => void> {
  if (!isTauri()) return () => {};
  const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
  return await getCurrentWebviewWindow().onFocusChanged(({ payload: focused }) => cb(focused));
}
