import { useLocalStorage } from "@vueuse/core";
import zhCN from "../i18n/zh-CN.json";
import en from "../i18n/en.json";

// P12 i18n · 轻量双语 composable（不引 vue-i18n，方案 §6.9 / P11 stage 9 决策）
// zh-CN 先行，en 分阶段；localStorage 持久化 + <html lang> 同步 + 运行时 patch 静态头

export type Lang = "zh-CN" | "en";

const DICTS: Record<Lang, Record<string, unknown>> = {
  "zh-CN": zhCN as Record<string, unknown>,
  en: en as Record<string, unknown>,
};

// 模块级单例 - 跨组件共享同一份 lang 状态（无 pinia，全局 ref）
const currentLang = useLocalStorage<Lang>("prism:lang", "zh-CN");

// 按 key 取嵌套值，缺省回退原 key（保护代码片段 / 命令 / 时间码这类不翻的占位）
function lookup(dict: Record<string, unknown>, path: string): string | undefined {
  const parts = path.split(".");
  let cur: unknown = dict;
  for (const p of parts) {
    if (cur && typeof cur === "object" && p in (cur as Record<string, unknown>)) {
      cur = (cur as Record<string, unknown>)[p];
    } else {
      return undefined;
    }
  }
  return typeof cur === "string" ? cur : undefined;
}

export function t(path: string): string {
  const val = lookup(DICTS[currentLang.value], path);
  if (val !== undefined) return val;
  // en 缺 key 时回退到 zh-CN（en 分阶段补全的过渡期）
  const fallback = lookup(DICTS["zh-CN"], path);
  return fallback ?? path;
}

export function useLang() {
  function setLang(lang: Lang) {
    currentLang.value = lang;
    document.documentElement.lang = lang;
    patchStaticHead(lang);
  }

  function toggle() {
    setLang(currentLang.value === "zh-CN" ? "en" : "zh-CN");
  }

  // 运行时 patch 静态头（仅 title / description，og 卡保持中文 - 方案决策：仅运行时双语）
  function patchStaticHead(lang: Lang) {
    const title = lookup(DICTS[lang], "meta.title");
    const desc = lookup(DICTS[lang], "meta.description");
    if (title) document.title = title;
    if (desc) {
      const el = document.querySelector('meta[name="description"]');
      if (el) el.setAttribute("content", desc);
    }
  }

  // 初始化时同步 <html lang>（首次加载 index.html 默认 zh-CN）
  if (typeof document !== "undefined") {
    document.documentElement.lang = currentLang.value;
  }

  return { currentLang, setLang, toggle, t };
}
