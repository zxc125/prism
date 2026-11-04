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

/** 一个已选取的 bundle 文件：Tauri 给 { path }（Rust 侧读），浏览器给 { content }。 */
export type PickedBundle =
  | { path: string; name: string }
  | { content: string; name: string };

function nameOfPath(p: string): string {
  return p.split(/[\\/]/).pop() ?? p;
}

function readTextFile(file: File): Promise<PickedBundle | null> {
  return new Promise((resolve) => {
    const reader = new FileReader();
    reader.onload = () =>
      resolve({ content: String(reader.result ?? ""), name: file.name });
    reader.onerror = () => resolve(null);
    reader.readAsText(file);
  });
}

/**
 * 选 bundle 文件并返回内容（P24：多选扩展；缺省单选，行为与历史一致）。
 * - Tauri：plugin-dialog 原生选择器，返回 { path } 给 Backend.importBundlePath（Rust 侧读）。
 * - 浏览器：`<input type="file">` + FileReader，返回 { content }。
 * 全部取消 / 读取失败时返回 null（多选下任意一个成功即返回成功集）。
 */
export async function pickBundleFiles(multiple = false): Promise<PickedBundle[] | null> {
  if (isTauri()) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const picked = await open({
      multiple,
      filters: [{ name: "会话 bundle", extensions: ["json"] }],
    });
    if (picked === null) return null;
    const paths = Array.isArray(picked) ? picked : [picked];
    if (!paths.length) return null;
    return paths.map((path) => ({ path, name: nameOfPath(path) }));
  }
  // 浏览器：<input type=file>
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.multiple = multiple;
    input.accept = ".json,application/json";
    input.onchange = async () => {
      const files = Array.from(input.files ?? []);
      if (!files.length) return resolve(null);
      const items = (await Promise.all(files.map(readTextFile))).filter(
        (x): x is PickedBundle => x !== null,
      );
      resolve(items.length ? items : null);
    };
    input.click();
  });
}

/**
 * 选一个 bundle 文件并返回内容（单选兼容入口，见 pickBundleFiles）。
 */
export async function pickBundleFile(): Promise<
  { path: string; name: string } | { content: string; name: string } | null
> {
  const items = await pickBundleFiles(false);
  return items?.[0] ?? null;
}

/**
 * 监听 bundle 文件拖放（P24 导入交互），跨环境分派：
 * - Tauri：webview `dragDropEnabled`（默认开）会拦截 HTML5 drop 事件，必须走
 *   `onDragDropEvent`（返回真实文件系统 path）；
 * - 浏览器：HTML5 drag 事件 + FileReader 读 content。
 * JSON 以外的文件被忽略。返回取消监听函数。
 */
export async function onBundleFileDrop(handlers: {
  onHoverChange: (hovering: boolean) => void;
  onDrop: (items: PickedBundle[]) => void;
}): Promise<() => void> {
  const isJson = (name: string) => name.toLowerCase().endsWith(".json");
  if (isTauri()) {
    const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
    const un = await getCurrentWebviewWindow().onDragDropEvent((ev) => {
      const p = ev.payload;
      if (p.type === "enter" || p.type === "over") {
        handlers.onHoverChange(true);
      } else if (p.type === "leave") {
        handlers.onHoverChange(false);
      } else if (p.type === "drop") {
        handlers.onHoverChange(false);
        const items = (p.paths ?? [])
          .filter((path) => isJson(path))
          .map((path) => ({ path, name: nameOfPath(path) }));
        if (items.length) handlers.onDrop(items);
      }
    });
    return un;
  }
  // 浏览器：HTML5 drag 事件（depth 计数防子元素抖动）
  let depth = 0;
  const hasFiles = (e: DragEvent) =>
    Array.from(e.dataTransfer?.types ?? []).includes("Files");
  const onDragEnter = (e: DragEvent) => {
    if (!hasFiles(e)) return;
    e.preventDefault();
    depth += 1;
    handlers.onHoverChange(true);
  };
  const onDragOver = (e: DragEvent) => {
    if (hasFiles(e)) e.preventDefault();
  };
  const onDragLeave = () => {
    depth = Math.max(0, depth - 1);
    if (!depth) handlers.onHoverChange(false);
  };
  const onDrop = async (e: DragEvent) => {
    if (!hasFiles(e)) return;
    e.preventDefault();
    depth = 0;
    handlers.onHoverChange(false);
    const files = Array.from(e.dataTransfer?.files ?? []).filter((f) =>
      isJson(f.name),
    );
    if (!files.length) return;
    const items = (await Promise.all(files.map(readTextFile))).filter(
      (x): x is PickedBundle => x !== null,
    );
    if (items.length) handlers.onDrop(items);
  };
  window.addEventListener("dragenter", onDragEnter);
  window.addEventListener("dragover", onDragOver);
  window.addEventListener("dragleave", onDragLeave);
  window.addEventListener("drop", onDrop);
  return () => {
    window.removeEventListener("dragenter", onDragEnter);
    window.removeEventListener("dragover", onDragOver);
    window.removeEventListener("dragleave", onDragLeave);
    window.removeEventListener("drop", onDrop);
  };
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
