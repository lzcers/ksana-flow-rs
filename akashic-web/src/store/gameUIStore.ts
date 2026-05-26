import { create } from 'zustand';
import type { StoreApi, UseBoundStore } from 'zustand';
import type {
  Character,
  GeneratedProfiles,
  PlayerActionInput,
  RuntimeStateView,
  World,
} from '../lib/api';

export type GameState = 'lobby' | 'archive_list' | 'creation' | 'generating' | 'playing';
export type StartupStage =
  | 'idle'
  | 'generating_world'
  | 'generating_protagonist'
  | 'ready_to_enter'
  | 'creating_session';

export interface GameUIState {
  // 当前页面所处的整体阶段。
  gameState: GameState;
  // 角色设定表单与存档摘要会读取的人物信息。
  character: Character;
  // 世界设定表单与存档摘要会读取的世界信息。
  world: World;
  // 运行时视图模型，驱动右侧状态面板等聚合信息。
  stateView: RuntimeStateView | null;
  // 执念点数。
  obsessionPoints: number;
  // 直觉点数。
  intuitionPoints: number;
  // 全局加载态，控制按钮禁用、骨架屏等。
  isLoading: boolean;
  // 开局前过渡页当前聚焦的阶段。
  startupStage: StartupStage;
  // 已生成但尚未正式注入会话的世界/主角设定。
  preparedProfiles: GeneratedProfiles | null;
  // 全局错误消息。
  error: string | null;
}

export interface GameUIActions {
  // 操作：切换整体页面阶段。
  setGameState: (state: GameState) => void;
  // 操作：更新角色设定。
  updateCharacter: (updates: Partial<Character>) => void;
  // 操作：更新世界设定。
  updateWorld: (updates: Partial<World>) => void;
  // 操作：清除错误提示。
  clearError: () => void;
  // 操作：生成设定并进入过渡页。
  startGame: () => Promise<void>;
  // 操作：基于已生成设定正式创建会话并进入游戏。
  enterWorld: () => Promise<void>;
  // 操作：提交当前选择；执念模式下也可直接提交自定义行动文本。
  submitChoice: (
    submission: { input: PlayerActionInput; displayText: string },
    useObsession?: boolean,
  ) => Promise<void>;
  // 操作：预览选择结果，通常对应直觉点玩法。
  previewChoice: (choiceId: string) => Promise<string>;
  // 操作：创建当前进度的存档。
  createSave: (title?: string) => Promise<string>;
  // 操作：加载指定存档。
  loadSave: (saveId: string) => Promise<void>;
  // 操作：重置本地游戏状态并关闭流连接。
  resetGame: () => void;
}

export type GameUIStoreState = GameUIState & GameUIActions;

export function createGameUIStore(
  initialState: GameUIState,
  createActions: (
    set: StoreApi<GameUIStoreState>['setState'],
    get: StoreApi<GameUIStoreState>['getState'],
  ) => GameUIActions,
): UseBoundStore<StoreApi<GameUIStoreState>> {
  return create<GameUIStoreState>((set, get) => ({
    ...initialState,
    ...createActions(set, get),
  }));
}
