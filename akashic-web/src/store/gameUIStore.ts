import { create } from 'zustand';
import type { StoreApi, UseBoundStore } from 'zustand';
import type {
  Character,
  RuntimeStateView,
  StoryNode,
  World,
} from '../lib/api';

export type GameState = 'lobby' | 'creation' | 'playing';

export interface GameUIState {
  // 当前页面所处的整体阶段。
  gameState: GameState;
  // 角色设定表单与存档摘要会读取的人物信息。
  character: Character;
  // 世界设定表单与存档摘要会读取的世界信息。
  world: World;
  // 当前剧情节点，包含正文、配图与可选项。
  currentNode: StoryNode | null;
  // 运行时视图模型，驱动右侧状态面板等聚合信息。
  stateView: RuntimeStateView | null;
  // 执念点数。
  obsessionPoints: number;
  // 直觉点数。
  intuitionPoints: number;
  // 全局加载态，控制按钮禁用、骨架屏等。
  isLoading: boolean;
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
  // 操作：创建会话并开始游戏。
  startGame: () => Promise<void>;
  // 操作：提交当前选择，可选消耗执念点。
  submitChoice: (choiceId: string, useObsession?: boolean) => Promise<void>;
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
