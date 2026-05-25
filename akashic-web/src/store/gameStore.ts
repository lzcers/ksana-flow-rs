import { create } from 'zustand';
import {
  createGameSession,
  generateProfiles,
  openGameSessionStream,
  submitGameSessionControl,
} from '../lib/api';
import type { Choice, GeneratedProfiles, PlayerActionInput, TaskView } from '../lib/api';
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
  startupStage: 'idle',
  preparedProfiles: null,
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
let activeStreamTaskRounds = new Map<string, number>();
let startupStageTimer: number | null = null;

const MIN_GENERATING_PAGE_MS = 1200;
const MIN_CREATING_SESSION_STAGE_MS = 450;

function areChoicesEqual(left: Choice[], right: Choice[]): boolean {
  if (left.length !== right.length) {
    return false;
  }

  return left.every((choice, index) => {
    const nextChoice = right[index];
    return choice.id === nextChoice.id
      && choice.text === nextChoice.text
      && choice.action === nextChoice.action
      && choice.previewText === nextChoice.previewText
      && choice.disabled === nextChoice.disabled
      && choice.costHints.intuition === nextChoice.costHints.intuition
      && choice.costHints.obsession === nextChoice.costHints.obsession;
  });
}

function closeActiveSessionStream() {
  activeSessionStream?.close();
  activeSessionStream = null;
  activeStreamSessionId = null;
  lastStreamEventId = null;
  activeStreamTasks = new Map();
  activeStreamTaskRounds = new Map();
}

function clearStartupStageTimer() {
  if (startupStageTimer !== null) {
    window.clearTimeout(startupStageTimer);
    startupStageTimer = null;
  }
}

function scheduleStartupStageProgress() {
  clearStartupStageTimer();
  startupStageTimer = window.setTimeout(() => {
    const state = useGameUIStore.getState();
    if (state.gameState === 'generating' && state.startupStage === 'generating_world') {
      useGameUIStore.setState({
        startupStage: 'generating_protagonist',
      });
    }
  }, 1400);
}

function sleep(ms: number) {
  return new Promise<void>((resolve) => {
    window.setTimeout(resolve, ms);
  });
}

function waitForNextPaint() {
  return new Promise<void>((resolve) => {
    window.requestAnimationFrame(() => resolve());
  });
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
    startupStage: 'idle',
    preparedProfiles: null,
    error: null,
  };
}

function applyStreamTaskToStores(task: TaskView, boundRound?: number | null) {
  const internalState = useGameInternalStore.getState();
  const uiState = useGameUIStore.getState();
  const stateView = uiState.stateView;
  const activeRound = Math.max(
    boundRound ?? internalState.displayRound ?? stateView?.turnIndex ?? 1,
    1,
  );
  const isLoading = task.status === 'pending' || task.status === 'running';
  let nextInternalState: Partial<GameInternalState> | null = null;
  let nextUIState: Partial<GameUIState> | null = null;

  switch (task.kind) {
    case 'narration': {
      const nextText = taskText(task);
      if (!nextText) {
        break;
      }

      const previousRoundState = internalState.roundStates[activeRound];
      const nextRoundState = createRoundState(activeRound, {
        ...(previousRoundState ?? {}),
        round: activeRound,
        narrationText: nextText,
        narrationStatus: task.status,
        isAwaitingNarration: false,
      });
      const nextTurnIndex = Math.max(internalState.turnIndex, activeRound);

      if (
        internalState.turnIndex !== nextTurnIndex
        || internalState.displayRound !== activeRound
        || previousRoundState?.narrationText !== nextRoundState.narrationText
        || previousRoundState?.narrationStatus !== nextRoundState.narrationStatus
        || previousRoundState?.isAwaitingNarration !== nextRoundState.isAwaitingNarration
      ) {
        nextInternalState = {
          turnIndex: nextTurnIndex,
          displayRound: activeRound,
          roundStates: {
            ...internalState.roundStates,
            [activeRound]: nextRoundState,
          },
        };
      }

      if (stateView && stateView.latestHistory !== nextText) {
        nextUIState = {
          stateView: {
            ...stateView,
            latestHistory: nextText,
          },
        };
      }
      break;
    }
    case 'fate_planning': {
      const raw = taskRawContent(task);
      const parsed = parseJsonValue(raw);
      const summary = summarizeFatePlanning(parsed);
      const nextRound = Math.max(summary?.round ?? activeRound, 1);
      const hadRoundState = Boolean(internalState.roundStates[nextRound]);
      const nextTurnIndex = Math.max(internalState.turnIndex, nextRound);
      const nextDisplayRound = internalState.displayRound || nextRound;

      if (
        internalState.turnIndex !== nextTurnIndex
        || internalState.displayRound !== nextDisplayRound
        || !hadRoundState
      ) {
        nextInternalState = {
          turnIndex: nextTurnIndex,
          displayRound: nextDisplayRound,
          roundStates: hadRoundState
            ? internalState.roundStates
            : {
              ...internalState.roundStates,
              [nextRound]: createRoundState(nextRound, {
                isAwaitingNarration: true,
              }),
            },
        };
      }

      if (stateView) {
        const nextBroadcastItems = summary?.newInfo.length
          ? summary.newInfo
          : stateView.latestBroadcastItems;
        const nextStateView = {
          ...stateView,
          turnIndex: nextRound,
          activeTurnId: nextRound,
          currentScene: summary?.sceneTitle ?? taskLabel(task.kind),
          currentLocation: summary?.locationName ?? stateView.currentLocation,
          protagonistState: summary?.protagonistCondition ?? stateView.protagonistState,
          latestBroadcastSummary:
            summary?.currentEvent ??
            summary?.newInfo[0] ??
            summary?.locationStatus ??
            summary?.description ??
            stateView.latestBroadcastSummary,
          latestBroadcastItems: nextBroadcastItems,
        };

        if (
          stateView.turnIndex !== nextStateView.turnIndex
          || stateView.activeTurnId !== nextStateView.activeTurnId
          || stateView.currentScene !== nextStateView.currentScene
          || stateView.currentLocation !== nextStateView.currentLocation
          || stateView.protagonistState !== nextStateView.protagonistState
          || stateView.latestBroadcastSummary !== nextStateView.latestBroadcastSummary
          || stateView.latestBroadcastItems !== nextStateView.latestBroadcastItems
        ) {
          nextUIState = {
            stateView: nextStateView,
          };
        }
      }
      break;
    }
    case 'protagonist_action': {
      const nextChoices = protagonistActionChoices(task);
      const choicesStatus: RoundChoicesStatus = nextChoices ? 'ready' : 'loading';
      const previousRoundState = internalState.roundStates[activeRound];
      const normalizedChoices = nextChoices ?? [];
      const nextRoundState = createRoundState(activeRound, {
        ...(previousRoundState ?? {}),
        round: activeRound,
        choices: normalizedChoices,
        choicesStatus,
        isAwaitingNarration: false,
      });

      if (
        !previousRoundState
        || previousRoundState.choicesStatus !== nextRoundState.choicesStatus
        || previousRoundState.isAwaitingNarration !== nextRoundState.isAwaitingNarration
        || !areChoicesEqual(previousRoundState.choices, normalizedChoices)
      ) {
        nextInternalState = {
          roundStates: {
            ...internalState.roundStates,
            [activeRound]: nextRoundState,
          },
        };
      }

      const nextProtagonistAction = protagonistActionText(task);
      if (
        stateView
        && nextProtagonistAction
        && stateView.latestProtagonistAction !== nextProtagonistAction
      ) {
        nextUIState = {
          stateView: {
            ...stateView,
            latestProtagonistAction: nextProtagonistAction,
          },
        };
      }
      break;
    }
    default:
      break;
  }

  if (uiState.isLoading !== isLoading) {
    nextUIState = {
      ...(nextUIState ?? {}),
      isLoading,
    };
  }

  if (nextInternalState) {
    useGameInternalStore.setState(nextInternalState);
  }

  if (nextUIState) {
    useGameUIStore.setState(nextUIState);
  }
}

export const useGameInternalStore = create<GameInternalState>(() => ({
  ...initialInternalState,
}));

const createGameUIActions = (
  set: (partial: Partial<GameUIStoreState> | ((state: GameUIStoreState) => Partial<GameUIStoreState>)) => void,
  get: () => GameUIStoreState,
): GameUIActions => ({
  setGameState: (state) => {
    if (state !== 'generating') {
      clearStartupStageTimer();
    }
    if (state !== 'playing') {
      closeActiveSessionStream();
    }
    set({
      gameState: state,
      startupStage: state === 'generating' ? get().startupStage : 'idle',
      preparedProfiles: state === 'generating' ? get().preparedProfiles : null,
      error: null,
    });
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
    clearStartupStageTimer();
    useGameInternalStore.setState({
      ...initialInternalState,
    });
    set({
      gameState: 'generating',
      error: null,
      isLoading: true,
      startupStage: 'generating_world',
      preparedProfiles: null,
      stateView: null,
    });
    await waitForNextPaint();
    scheduleStartupStageProgress();

    try {
      const generatingStartedAt = Date.now();
      const generatedProfiles = await generateProfiles(character, world);
      const generatingElapsed = Date.now() - generatingStartedAt;
      if (generatingElapsed < MIN_GENERATING_PAGE_MS) {
        await sleep(MIN_GENERATING_PAGE_MS - generatingElapsed);
      }
      clearStartupStageTimer();
      set({
        startupStage: 'ready_to_enter',
        preparedProfiles: generatedProfiles,
        isLoading: false,
      });
    } catch (error) {
      clearStartupStageTimer();
      closeActiveSessionStream();
      useGameInternalStore.setState({
        ...initialInternalState,
      });
      set({
        gameState: 'creation',
        stateView: null,
        isLoading: false,
        startupStage: 'idle',
        error: error instanceof Error ? error.message : '开启旅程失败。',
      });
      throw error;
    }
  },
  enterWorld: async () => {
    const { character, world, preparedProfiles } = get();
    if (!preparedProfiles) {
      throw new Error('设定尚未生成完成，无法进入幻世。');
    }

    set({
      error: null,
      isLoading: true,
    });
    await waitForNextPaint();

    try {
      const [created] = await Promise.all([
        createGameSession({
          worldProfile: preparedProfiles.world,
          protagonistProfile: preparedProfiles.protagonist,
        }),
        sleep(MIN_CREATING_SESSION_STAGE_MS),
      ]);

      useGameInternalStore.setState({
        ...initialInternalState,
        sessionId: created.sessionId,
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
          latestBroadcastSummary: '会话已建立，正在推进第一轮...',
          latestBroadcastItems: ['会话已建立，正在推进第一轮...'],
          latestProtagonistAction: '尚未做出选择',
        },
        obsessionPoints: 3,
        intuitionPoints: 5,
        error: null,
        gameState: 'playing',
        isLoading: true,
        startupStage: 'idle',
        preparedProfiles: null,
      });

      activeStreamSessionId = created.sessionId;

      activeSessionStream = openGameSessionStream(
        created.sessionId,
        {
          onTaskUpdated: (event, lastEventId) => {
            if (activeStreamSessionId !== created.sessionId) {
              return;
            }
            lastStreamEventId = lastEventId || lastStreamEventId;
            if (
              event.status === 'pending'
              && (event.kind === 'narration' || event.kind === 'protagonist_action')
            ) {
              const internalState = useGameInternalStore.getState();
              const stateView = useGameUIStore.getState().stateView;
              const boundRound = Math.max(internalState.displayRound || stateView?.turnIndex || 1, 1);
              activeStreamTaskRounds.set(event.entity, boundRound);
            }
            const nextTask = applyTaskUpdate(activeStreamTasks, event);
            applyStreamTaskToStores(nextTask, activeStreamTaskRounds.get(event.entity));
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
      set({
        gameState: 'generating',
        isLoading: false,
        startupStage: 'ready_to_enter',
        error: error instanceof Error ? error.message : '进入幻世失败。',
      });
      throw error;
    }
  },
  submitChoice: async (submission, useObsession = false) => {
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

    const nextInput: PlayerActionInput = {
      type: submission.input.type,
      action: submission.input.action.trim(),
    };
    if (!nextInput.action) {
      throw new Error('当前行动不能为空。');
    }

    const nextObsession = useObsession ? Math.max(0, obsessionPoints - 1) : obsessionPoints;
    const activeRound = Math.max(displayRound || 1, 1);
    const nextRound = activeRound + 1;
    const currentRoundChoices = roundStates[activeRound]?.choices ?? [];
    if (
      nextInput.type === 'selected_option'
      && !currentRoundChoices.some((choice) => choice.action === nextInput.action)
    ) {
      throw new Error('当前选择不存在。');
    }
    const selectedChoiceText = useObsession
      ? `${submission.displayText} [执念]`
      : submission.displayText;

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
        action: nextInput,
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
