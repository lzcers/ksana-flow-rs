import { create } from 'zustand';
import {
  createGameSession,
  openGameSessionStream,
  submitGameSessionControl,
} from '../lib/api';
import type { TaskView } from '../lib/api';
import {
  createGameUIStore,
  type GameState,
  type GameUIActions,
  type GameUIState,
  type GameUIStoreState,
} from './gameUIStore';
import {
  applyTaskUpdate,
  buildEndingNode,
  buildStateView,
  buildStoryNode,
  cloneCharacter,
  cloneWorld,
  DEMO_NODES,
  initialCharacter,
  initialWorld,
  parseJsonValue,
  protagonistActionChoices,
  protagonistActionText,
  summarizeFatePlanning,
  STREAM_PLACEHOLDER_TEXT,
  STORY_IMAGES,
  taskLabel,
  taskRawContent,
  taskText,
  type DemoNodeId,
  type JsonValue,
} from './gameStoreHelpers';

interface GameInternalState {
  // 内部状态：当前后端会话 id，用于继续推进会话。
  sessionId: string | null;
  // 内部状态：当前已推进到的回合序号。
  turnIndex: number;
  // 内部状态：叙事任务流式输出中的正文文本。
  streamedNarrationText: string;
  // 内部状态：叙事任务当前状态，可用于判断流是否完成。
  streamedNarrationStatus: TaskView['status'] | null;
  // 内部状态：命运规划原始文本，通常用于调试面板或扩展信息区。
  streamedFatePlanningRaw: string;
  // 内部状态：命运规划解析后的 JSON，便于派生摘要字段。
  streamedFatePlanningJson: JsonValue | null;
  // 内部状态：主角行动原始文本，通常用于调试面板或扩展信息区。
  streamedProtagonistActionRaw: string;
  // 内部状态：主角行动解析后的 JSON，便于生成选项与摘要。
  streamedProtagonistActionJson: JsonValue | null;
}

const initialUIState: GameUIState = {
  gameState: 'lobby' as GameState,
  character: initialCharacter,
  world: initialWorld,
  currentNode: null,
  stateView: null,
  obsessionPoints: 3,
  intuitionPoints: 5,
  isLoading: false,
  error: null,
};

const initialInternalState: GameInternalState = {
  sessionId: null,
  turnIndex: 0,
  streamedNarrationText: '',
  streamedNarrationStatus: null,
  streamedFatePlanningRaw: '',
  streamedFatePlanningJson: null,
  streamedProtagonistActionRaw: '',
  streamedProtagonistActionJson: null,
};

let activeSessionStream: EventSource | null = null;
let activeStreamSessionId: string | null = null;
let lastStreamEventId: string | null = null;
let activeStreamTasks = new Map<string, TaskView>();

function closeActiveSessionStream() {
  activeSessionStream?.close();
  activeSessionStream = null;
  activeStreamSessionId = null;
  lastStreamEventId = null;
  activeStreamTasks = new Map();
}

function resetUIState(): GameUIState {
  return {
    gameState: 'lobby' as GameState,
    character: cloneCharacter(initialCharacter),
    world: cloneWorld(initialWorld),
    currentNode: null,
    stateView: null,
    obsessionPoints: 3,
    intuitionPoints: 5,
    isLoading: false,
    error: null,
  };
}

function applyStreamTaskToStores(task: TaskView) {
  useGameUIStore.setState((state) => {
    const isLoading = task.status === 'pending' || task.status === 'running';

    switch (task.kind) {
      case 'narration': {
        const nextText = taskText(task);
        if (!nextText) {
          return { isLoading };
        }

        useGameInternalStore.setState({
          streamedNarrationText: nextText,
          streamedNarrationStatus: task.status,
        });

        return {
          isLoading,
          currentNode: state.currentNode
            ? {
              ...state.currentNode,
              text: nextText,
              choices: [],
            }
            : null,
          stateView: state.stateView
            ? {
              ...state.stateView,
              latestHistory: nextText,
            }
            : null,
        };
      }
      case 'fate_planning': {
        const raw = taskRawContent(task);
        const parsed = parseJsonValue(raw);
        const summary = summarizeFatePlanning(parsed);
        useGameInternalStore.setState({
          streamedFatePlanningRaw: raw,
          streamedFatePlanningJson: parsed,
        });

        return {
          isLoading,
          stateView: state.stateView
            ? {
              ...state.stateView,
              turnIndex: summary?.round ?? state.stateView.turnIndex,
              activeTurnId: summary?.round ?? state.stateView.activeTurnId,
              currentScene: summary?.sceneTitle ?? taskLabel(task.kind),
              currentLocation: summary?.locationName ?? state.stateView.currentLocation,
              protagonistState: summary?.protagonistCondition ?? state.stateView.protagonistState,
              latestBroadcastSummary:
                summary?.currentEvent ??
                summary?.newInfo[0] ??
                summary?.locationStatus ??
                summary?.description ??
                state.stateView.latestBroadcastSummary,
              latestBroadcastItems:
                summary?.newInfo.length
                  ? summary.newInfo
                  : state.stateView.latestBroadcastItems,
            }
            : null,
        };
      }
      case 'protagonist_action': {
        const raw = taskRawContent(task);
        const parsed = parseJsonValue(raw);
        const nextChoices = protagonistActionChoices(task);

        useGameInternalStore.setState({
          streamedProtagonistActionRaw: raw,
          streamedProtagonistActionJson: parsed,
        });

        return {
          isLoading,
          currentNode: state.currentNode
            ? {
              ...state.currentNode,
              choices: nextChoices ?? state.currentNode.choices,
            }
            : null,
          stateView: state.stateView
            ? {
              ...state.stateView,
              latestProtagonistAction:
                protagonistActionText(task) ?? state.stateView.latestProtagonistAction,
            }
            : null,
        };
      }
      default:
        return { isLoading };
    }
  });
}

export const useGameInternalStore = create<GameInternalState>(() => ({
  ...initialInternalState,
}));

const createGameUIActions = (
  set: (partial: Partial<GameUIStoreState> | ((state: GameUIStoreState) => Partial<GameUIStoreState>)) => void,
  get: () => GameUIStoreState,
): GameUIActions => ({
  setGameState: (state) => {
    if (state !== 'playing') {
      closeActiveSessionStream();
    }
    set({ gameState: state, error: null });
  },
  updateCharacter: (updates) =>
    set((state) => ({
      character: {
        ...state.character,
        ...updates,
        traits: updates.traits ? { ...state.character.traits, ...updates.traits } : state.character.traits,
      },
    })),
  updateWorld: (updates) =>
    set((state) => ({
      world: {
        ...state.world,
        ...updates,
        specialRules: updates.specialRules ?? state.world.specialRules,
      },
    })),
  clearError: () => set({ error: null }),
  startGame: async () => {
    const { character, world } = get();
    closeActiveSessionStream();
    useGameInternalStore.setState({
      ...initialInternalState,
    });
    set({
      currentNode: {
        id: 'loading',
        text: STREAM_PLACEHOLDER_TEXT,
        image: STORY_IMAGES.opening,
        choices: [],
      },
      stateView: {
        gameState: 'playing',
        phase: 'booting',
        turnIndex: 0,
        activeTurnId: 0,
        currentLocation: '命运现场',
        currentScene: '命运编织中',
        protagonistState: `${character.name || '无名旅人'} 正踏入 ${world.era}`,
        npcsState: '诸多回响正在汇聚',
        latestHistory: STREAM_PLACEHOLDER_TEXT,
        latestBroadcastSummary: '正在创建会话并唤起第一轮命运...',
        latestBroadcastItems: ['正在创建会话并唤起第一轮命运...'],
        latestProtagonistAction: '尚未做出选择',
      },
      obsessionPoints: 3,
      intuitionPoints: 5,
      error: null,
      gameState: 'playing',
      isLoading: true,
    });

    try {
      // 创建会话
      const created = await createGameSession(character, world);
      activeStreamSessionId = created.sessionId;
      useGameInternalStore.setState({
        sessionId: created.sessionId,
      });

      set((state) => ({
        stateView: state.stateView
          ? {
            ...state.stateView,
            latestBroadcastSummary: '会话已建立，正在推进第一轮...',
            latestBroadcastItems: ['会话已建立，正在推进第一轮...'],
          }
          : null,
      }));

      activeSessionStream = openGameSessionStream(
        created.sessionId,
        {
          onTaskUpdated: (event, lastEventId) => {
            if (activeStreamSessionId !== created.sessionId) {
              return;
            }
            lastStreamEventId = lastEventId || lastStreamEventId;
            const nextTask = applyTaskUpdate(activeStreamTasks, event);
            applyStreamTaskToStores(nextTask);
          },
          onError: () => {
            if (activeStreamSessionId !== created.sessionId) {
              return;
            }
            set({
              error: '叙事流连接出现波动，正在尝试恢复...',
            });
          },
        },
        lastStreamEventId,
      );

      await submitGameSessionControl(created.sessionId, {
        control: { type: 'continue' },
      });
    } catch (error) {
      closeActiveSessionStream();
      set({
        isLoading: false,
        error: error instanceof Error ? error.message : '开启旅程失败。',
      });
      throw error;
    }
  },
  submitChoice: async (choiceId, useObsession = false) => {
    const { sessionId, turnIndex } = useGameInternalStore.getState();
    const {
      character,
      world,
      currentNode,
      obsessionPoints,
      intuitionPoints,
    } = get();

    if (!sessionId || !currentNode) {
      throw new Error('当前没有可推进的演示剧情。');
    }

    if (activeStreamSessionId === sessionId) {
      const nextObsession = useObsession ? Math.max(0, obsessionPoints - 1) : obsessionPoints;

      set((state) => ({
        isLoading: true,
        obsessionPoints: nextObsession,
        intuitionPoints,
        currentNode: state.currentNode
          ? {
            ...state.currentNode,
            text: STREAM_PLACEHOLDER_TEXT,
            choices: [],
          }
          : null,
        stateView: state.stateView
          ? {
            ...state.stateView,
            latestHistory: STREAM_PLACEHOLDER_TEXT,
          }
          : null,
        error: null,
      }));
      useGameInternalStore.setState({
        streamedNarrationText: '',
        streamedNarrationStatus: null,
        streamedFatePlanningRaw: '',
        streamedFatePlanningJson: null,
        streamedProtagonistActionRaw: '',
        streamedProtagonistActionJson: null,
      });

      try {
        await submitGameSessionControl(sessionId, {
          choice: { choiceId },
        });
        return;
      } catch (error) {
        set({
          isLoading: false,
          error: error instanceof Error ? error.message : '提交选择失败。',
        });
        throw error;
      }
    }

    const currentMeta = DEMO_NODES[currentNode.id as DemoNodeId];
    const choice = currentMeta.choices.find((item) => item.id === choiceId);

    if (!choice) {
      throw new Error('当前选择不存在。');
    }

    const nextObsession = useObsession ? Math.max(0, obsessionPoints - 1) : obsessionPoints;

    if (choice.nextNodeId) {
      const nextNode = buildStoryNode(choice.nextNodeId, character, world);
      set({
        currentNode: nextNode,
        stateView: buildStateView(choice.nextNodeId, turnIndex + 1, nextNode.text, choice.text),
        obsessionPoints: nextObsession,
        intuitionPoints,
        error: null,
        gameState: 'playing',
      });
      useGameInternalStore.setState({
        turnIndex: turnIndex + 1,
      });
      return;
    }

    if (!choice.endingId) {
      throw new Error('演示剧情缺少结局配置。');
    }

    const endingNode = buildEndingNode(choice.endingId, character, world);
    set({
      currentNode: endingNode,
      stateView: {
        gameState: 'playing',
        phase: 'demo_complete',
        turnIndex: turnIndex + 1,
        activeTurnId: turnIndex + 1,
        currentLocation: currentMeta.location,
        currentScene: '人生回响',
        protagonistState: '命运已收束',
        npcsState: '馆藏记录已生成',
        latestHistory: endingNode.text,
        latestBroadcastSummary: '命运收束：本次旅程已被写入本地归档演示数据。',
        latestBroadcastItems: ['命运收束：本次旅程已被写入本地归档演示数据。'],
        latestProtagonistAction: choice.text,
      },
      obsessionPoints: nextObsession,
      intuitionPoints,
      error: null,
      gameState: 'playing',
    });
    useGameInternalStore.setState({
      turnIndex: turnIndex + 1,
    });
  },
  previewChoice: async (_choiceId) => {
    throw new Error('演示直觉点已耗尽。');
  },
  createSave: async (_title) => {
    throw new Error('当前没有可保存的演示旅程。');
  },
  loadSave: async (_saveId) => {

  },
  resetGame: () => {
    closeActiveSessionStream();
    useGameInternalStore.setState({
      ...initialInternalState,
    });
    set((state) => ({
      ...state,
      ...resetUIState(),
    }));
  },
});

export const useGameUIStore = createGameUIStore(initialUIState, createGameUIActions);

export const useGameStore = useGameUIStore;
