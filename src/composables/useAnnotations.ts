import { invoke } from "@tauri-apps/api/core";
import { ref } from "vue";

export type Annotation = {
  id: string;
  /** 相对会话起点 ms（与 signals 共享时间轴） */
  t: number;
  /** 关联窗口 label（可选） */
  label?: string;
  text: string;
  author: string;
  createdAt: number;
};

/**
 * 会话级标注：load 后前端持有完整列表，增删改后 debounce 整体覆写
 * annotations.jsonl。与 segment 事件流分离，回放时与 signals 共享相对
 * 会话起点的时间轴。导出/导入由后端 export_session / import_session 承载。
 */
export function useAnnotations(sessionId: string) {
  const annotations = ref<Annotation[]>([]);
  const loaded = ref(false);

  async function load() {
    try {
      const list = await invoke<Annotation[]>("list_annotations", { id: sessionId });
      annotations.value = list.sort((a, b) => a.t - b.t);
      loaded.value = true;
    } catch (e) {
      console.error("[annotations] load failed", e);
      annotations.value = [];
    }
  }

  // 立即保存：标注是低频手动操作（打点/删除），无需 debounce，避免窗口关闭丢未 flush 的变更
  async function persist() {
    try {
      await invoke("save_annotations", {
        id: sessionId,
        annotations: annotations.value,
      });
    } catch (e) {
      console.error("[annotations] save failed", e);
    }
  }

  function genId() {
    return `a${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
  }

  /** 在指定时间点添加标注，返回新标注。自动按 t 插入保持有序。 */
  function add(partial: Omit<Annotation, "id" | "createdAt">): Annotation {
    const a: Annotation = {
      ...partial,
      id: genId(),
      createdAt: Date.now(),
    };
    const insertAt = annotations.value.findIndex((x) => x.t > a.t);
    if (insertAt < 0) annotations.value.push(a);
    else annotations.value.splice(insertAt, 0, a);
    void persist();
    return a;
  }

  function update(id: string, text: string) {
    const a = annotations.value.find((x) => x.id === id);
    if (!a) return;
    a.text = text;
    void persist();
  }

  function remove(id: string) {
    annotations.value = annotations.value.filter((x) => x.id !== id);
    void persist();
  }

  return { annotations, loaded, load, add, update, remove };
}
