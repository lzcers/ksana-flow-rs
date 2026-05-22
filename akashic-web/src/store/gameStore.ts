import { create } from 'zustand';
import {
  createGameSession,
  openGameSessionStream,
  submitGameSessionControl,
} from '../lib/api';
import type { Choice, TaskView } from '../lib/api';
import {
  createGameUIStore,
  type GameState,
  type GameUIActions,
  type GameUIState,
  type GameUIStoreState,
} from './gameUIStore';
import {
  applyTaskUpdate,
  cloneCharacter,
  cloneWorld,
  initialCharacter,
  initialWorld,
  parseJsonValue,
  protagonistActionChoices,
  protagonistActionText,
  summarizeFatePlanning,
  STREAM_PLACEHOLDER_TEXT,
  taskLabel,
  taskRawContent,
  taskText,
  type JsonValue,
} from './gameStoreHelpers';

type RoundChoicesStatus = 'idle' | 'loading' | 'ready';

interface RoundState {
  round: number;
  narrationText: string;
  narrationStatus: TaskView['status'] | null;
  choices: Choice[];
  choicesStatus: RoundChoicesStatus;
  selectedChoiceText: string | null;
  isAwaitingNarration: boolean;
}

interface GameInternalState {
  // 内部状态：当前后端会话 id，用于继续推进会话。
  sessionId: string | null;
  // 内部状态：当前已推进到的回合序号。
  turnIndex: number;
  // 内部状态：当前页面应展示的回合序号，与服务端 turnIndex 解耦。
  displayRound: number;
  // 内部状态：按轮次隔离的叙事/选项时间线。
  roundStates: Record<number, RoundState>;
}

const initialUIState: GameUIState = {
  gameState: 'lobby' as GameState,
  character: initialCharacter,
  world: initialWorld,
  stateView: null,
  obsessionPoints: 3,
  intuitionPoints: 5,
  isLoading: false,
  error: null,
};

const initialInternalState: GameInternalState = {
  sessionId: null,
  turnIndex: 0,
  displayRound: 0,
  roundStates: {},
};

function createRoundState(round: number, overrides: Partial<RoundState> = {}): RoundState {
  return {
    round,
    narrationText: '',
    narrationStatus: null,
    choices: [],
    choicesStatus: 'idle',
    selectedChoiceText: null,
    isAwaitingNarration: false,
    ...overrides,
  };
}

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
    stateView: null,
    obsessionPoints: 3,
    intuitionPoints: 5,
    isLoading: false,
    error: null,
  };
}

function applyStreamTaskToStores(task: TaskView) {
  const internalState = useGameInternalStore.getState();
  const uiState = useGameUIStore.getState();
  const activeRound = Math.max(internalState.displayRound || uiState.stateView?.turnIndex || 1, 1);

  useGameUIStore.setState((state) => {
    const isLoading = task.status === 'pending' || task.status === 'running';

    switch (task.kind) {
      case 'narration': {
        const nextText = taskText(task);
        if (!nextText) {
          return { isLoading };
        }

        useGameInternalStore.setState((prev) => ({
          turnIndex: Math.max(prev.turnIndex, activeRound),
          displayRound: activeRound,
          roundStates: {
            ...prev.roundStates,
            [activeRound]: createRoundState(activeRound, {
              ...(prev.roundStates[activeRound] ?? {}),
              round: activeRound,
              narrationText: nextText,
              narrationStatus: task.status,
              isAwaitingNarration: false,
            }),
          },
        }));

        return {
          isLoading,
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
        const nextRound = Math.max(summary?.round ?? activeRound, 1);
        useGameInternalStore.setState((prev) => ({
          turnIndex: Math.max(prev.turnIndex, nextRound),
          displayRound: prev.displayRound || nextRound,
          roundStates: prev.roundStates[nextRound]
            ? prev.roundStates
            : {
              ...prev.roundStates,
              [nextRound]: createRoundState(nextRound, {
                isAwaitingNarration: true,
              }),
            },
        }));

        return {
          isLoading,
          stateView: state.stateView
            ? {
              ...state.stateView,
              turnIndex: nextRound,
              activeTurnId: nextRound,
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
        const nextChoices = protagonistActionChoices(task);
        const choicesStatus: RoundChoicesStatus = nextChoices ? 'ready' : 'loading';

        useGameInternalStore.setState((prev) => ({
          roundStates: {
            ...prev.roundStates,
            [activeRound]: createRoundState(activeRound, {
              ...(prev.roundStates[activeRound] ?? {}),
              round: activeRound,
              choices: nextChoices ?? [],
              choicesStatus,
              isAwaitingNarration: false,
            }),
          },
        }));

        return {
          isLoading,
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
      displayRound: 1,
      roundStates: {
        1: createRoundState(1, {
          isAwaitingNarration: true,
        }),
      },
    });
    set({
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
    const { sessionId, displayRound, roundStates } = useGameInternalStore.getState();
    const {
      obsessionPoints,
      intuitionPoints,
    } = get();

    if (!sessionId) {
      throw new Error('当前没有可推进的演示剧情。');
    }

    if (activeStreamSessionId !== sessionId) {
      throw new Error('当前会话流未就绪，无法提交选择。');
    }

    const nextObsession = useObsession ? Math.max(0, obsessionPoints - 1) : obsessionPoints;
    const activeRound = Math.max(displayRound || 1, 1);
    const nextRound = activeRound + 1;
    const selectedChoice = roundStates[activeRound]?.choices.find((choice) => choice.id === choiceId);
    const selectedChoiceText = selectedChoice
      ? (useObsession ? `${selectedChoice.text} [执念]` : selectedChoice.text)
      : null;

    if (!selectedChoiceText) {
      throw new Error('当前选择不存在。');
    }

    const previousRoundState = roundStates[activeRound];
    const previousNextRoundState = roundStates[nextRound];

    set({
      isLoading: true,
      obsessionPoints: nextObsession,
      intuitionPoints,
      error: null,
    });
    useGameInternalStore.setState((state) => ({
      displayRound: nextRound,
      roundStates: {
        ...state.roundStates,
        [activeRound]: createRoundState(activeRound, {
          ...(state.roundStates[activeRound] ?? {}),
          round: activeRound,
          selectedChoiceText,
          choices: [],
          choicesStatus: 'idle',
          isAwaitingNarration: false,
        }),
        [nextRound]: createRoundState(nextRound, {
          ...(state.roundStates[nextRound] ?? {}),
          round: nextRound,
          choices: [],
          choicesStatus: 'loading',
          isAwaitingNarration: true,
        }),
      },
    }));

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
      useGameInternalStore.setState((state) => {
        const nextRoundStates = { ...state.roundStates };
        if (previousRoundState) {
          nextRoundStates[activeRound] = previousRoundState;
        } else {
          delete nextRoundStates[activeRound];
        }

        if (previousNextRoundState) {
          nextRoundStates[nextRound] = previousNextRoundState;
        } else {
          delete nextRoundStates[nextRound];
        }

        return {
          displayRound: activeRound,
          roundStates: nextRoundStates,
        };
      });
      throw error;
    }
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
