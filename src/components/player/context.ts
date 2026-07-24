import type { InjectionKey } from "vue";
import type { usePlayer } from "../../composables/usePlayer";
import type { useAnnotations } from "../../composables/useAnnotations";

/** Player 子组件共享的上下文：usePlayer + useAnnotations 实例 + sessionId。
 *  PlayerShell 创建并 provide，Timeline / DiagnosisPanel / ReplayGrid inject 消费。 */
export interface PlayerCtx {
  sessionId: string;
  player: ReturnType<typeof usePlayer>;
  annos: ReturnType<typeof useAnnotations>;
}

export const PLAYER_CTX: InjectionKey<PlayerCtx> = Symbol("player-ctx");
