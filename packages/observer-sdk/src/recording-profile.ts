import type { RecordProfile, RecordingOptions } from "./types";

/**
 * rrweb record() 量控子集：本 SDK 实际会设置的键。
 * 刻意用结构化局部类型而非 re-export rrweb 类型（与 RREvent 同一策略），避免类型面耦合；
 * 完整 record options 透传（细调每个通道）留 P.x。
 */
export interface RecordTuning {
  sampling?: {
    /** false=关；number=聚合阈值 ms（rrweb 默认 50）。 */
    mousemove?: boolean | number;
    /**
     * 对象形式是「禁用表」：值为 false 的交互不注册（rrweb 只注册 disableMap[key] !== false
     * 的事件，见 rrweb observer 源码），未列出/true = 记录。
     */
    mouseInteraction?: boolean | Record<string, boolean | undefined>;
    /** 节流间隔 ms（rrweb 类型只收 number，无法全关）。 */
    scroll?: number;
    input?: "all" | "last";
  };
  /** rrweb 简写形式：true / 'all' = 剪枝次要节点（script/comment/stylesheet/headMeta 等）。 */
  slimDOMOptions?: true | "all";
  /** 逗号分隔 CSS selector：命中元素子树的 DOM 变更不进事件流，回放显示占位。 */
  blockSelector?: string;
}

/**
 * 预设映射（P14 D1）。基准是 rrweb 默认行为，full 档返回 {} 与旧版零 diff（D3）。
 * 注意边界（决策文档已锁定）：sampling 管不到 mutation——高频 DOM 渲染场景的
 * 数据量问题主要靠 domBlocks（免录区）解决，档位只静默高频交互通道。
 */
const PRESETS: Record<Exclude<RecordProfile, "full">, RecordTuning> = {
  balanced: {
    sampling: { mousemove: 100, scroll: 200, input: "last" },
    slimDOMOptions: true,
  },
  minimal: {
    sampling: {
      mousemove: false,
      scroll: 500,
      // 禁用表语义（false=关）：保留 Click / TouchStart / TouchEnd，其余静默。
      // 注意 rrweb 2.1.1 枚举无 TouchMove（触摸移动走 mousemove 通道），TouchCancel 显式关
      mouseInteraction: {
        MouseUp: false,
        MouseDown: false,
        ContextMenu: false,
        DblClick: false,
        Focus: false,
        Blur: false,
        TouchCancel: false,
      },
      input: "last",
    },
    slimDOMOptions: "all",
  },
};

/** 组装 rrweb record 量控参数：未配置或 full 档返回 {}（零 diff）；domBlocks → blockSelector（D4）。 */
export function resolveRecordOptions(recording?: RecordingOptions): RecordTuning {
  const profile = recording?.profile ?? "full";
  const tuning: RecordTuning = profile === "full" ? {} : { ...PRESETS[profile] };
  if (recording?.domBlocks?.length) {
    tuning.blockSelector = recording.domBlocks.join(",");
  }
  return tuning;
}
