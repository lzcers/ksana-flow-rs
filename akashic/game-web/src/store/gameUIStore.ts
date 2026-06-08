import { create } from 'zustand';
import type { StoreApi } from 'zustand';
import {
  cloneGameSession,
  createGameSession,
  exportGameSaveArchive,
  generateProfiles,
  getGameSession,
  loadGameSessionFromArchive,
  openGameSessionStream,
  submitGameSessionControl,
} from '../lib/api';
import type {
  Character,
  GeneratedProfiles,
  PlayerActionInput,
  RuntimeStateView,
  StoryPreferences,
  TaskView,
  World,
} from '../lib/api';
import {
  readStoredSaveArchive,
  upsertStoredSaveSlot,
  writeStoredSaveArchive,
} from '../lib/saveSlots';
import { appRoutes, routeWithSession } from '../lib/appRoutes';
import { navigateTo } from '../lib/navigation';
import { useGameValueStore } from './gameValueStore';
import {
  createRoundState,
  initialInternalState,
  type GameInternalState,
  type RoundChoicesStatus,
  type RoundState,
  useGameInternalStore,
} from './gameStore';
import {
  effectiveDisplayRound,
  internalStateFromSession,
  stateViewFromSession,
} from './sessionMappers';
import {
  applyTaskUpdate,
  cloneCharacter,
  cloneStory,
  cloneWorld,
  initialCharacter,
  initialStory,
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

export type StartupStage =
  | 'idle'
  | 'generating_world'
  | 'generating_protagonist'
  | 'ready_to_enter'
  | 'creating_session';

export interface GameUIState {
  // 角色设定表单与存档摘要会读取的人物信息。
  character: Character;
  // 世界设定表单与存档摘要会读取的世界信息。
  world: World;
  // 故事设定表单会读取的叙事偏好与禁区。
  story: StoryPreferences;
  // 运行时视图模型，驱动右侧状态面板等聚合信息。
  stateView: RuntimeStateView | null;
  // 全局加载态，控制按钮禁用、骨架屏等。
  isLoading: boolean;
  // 开局前过渡页当前聚焦的阶段。
  startupStage: StartupStage;
  // 已生成但尚未正式注入会话的世界/主角设定。
  preparedProfiles: GeneratedProfiles | null;
  // 全局错误消息。
  error: string | null;
  // 从存档恢复后，当前已存在叙事应直接展示，不再重新打字。
  skipRestoredNarrationAnimation: boolean;
}

export interface GameUIActions {
  // 操作：更新角色设定。
  updateCharacter: (updates: Partial<Character>) => void;
  // 操作：更新世界设定。
  updateWorld: (updates: Partial<World>) => void;
  // 操作：更新故事设定。
  updateStory: (updates: Partial<StoryPreferences>) => void;
  // 操作：清除错误提示。
  clearError: () => void;
  // 操作：生成设定并进入过渡页。
  startGame: () => Promise<void>;
  // 操作：基于已生成设定正式创建会话并进入游戏。
  enterWorld: () => Promise<void>;
  // 操作：在进入游玩页后触发开场叙事。
  bootstrapSession: () => Promise<void>;
  // 操作：提交当前选择；执念模式下也可直接提交自定义行动文本。
  submitChoice: (
    submission: { input: PlayerActionInput; displayText: string },
    useObsession?: boolean,
  ) => Promise<void>;
  // 操作：创建当前进度的存档。
  createSave: (title?: string) => Promise<string>;
  // 操作：加载指定存档。
  loadSave: (saveId: string) => Promise<void>;
  // 操作：通过仍然存活的后端会话 id 恢复当前进度。
  restoreSession: (sessionId: string) => Promise<void>;
  // 操作：基于分享链接复制一份独立会话并切换到该分支。
  cloneSharedSession: (sourceSessionId: string) => Promise<{ sessionId: string; isEnding: boolean }>;
  // 操作：重置本地游戏状态并关闭流连接。
  resetGame: () => void;
}

export type GameUIStoreState = GameUIState & GameUIActions;

const initialUIState: GameUIState = {
  character: initialCharacter,
  world: initialWorld,
  story: initialStory,
  stateView: null,
  isLoading: false,
  startupStage: 'idle',
  preparedProfiles: null,
  error: null,
  skipRestoredNarrationAnimation: false,
};

let activeSessionStream: EventSource | null = null;
let activeStreamSessionId: string | null = null;
let lastStreamEventId: string | null = null;
let activeStreamTasks = new Map<string, TaskView>();
let activeStreamTaskRounds = new Map<string, number>();
let startupStageTimer: number | null = null;
let bootstrappingSessionId: string | null = null;
let restoringSessionId: string | null = null;
let activeCloneRequest: {
  sourceSessionId: string;
  promise: Promise<{ sessionId: string; isEnding: boolean }>;
} | null = null;
let startupFlowRunId = 0;

const MIN_GENERATING_PAGE_MS = 1200;
const MIN_CREATING_SESSION_STAGE_MS = 450;
const FIRST_ROUND_READY_TIMEOUT_MS = 45000;

function areChoicesEqual(left: RoundState['choices'], right: RoundState['choices']): boolean {
  if (left.length !== right.length) {
    return false;
  }

  return left.every((choice, index) => {
    const nextChoice = right[index];
    return choice.id === nextChoice.id
      && choice.text === nextChoice.text
      && choice.action === nextChoice.action
      && choice.motivationAndRisk === nextChoice.motivationAndRisk
      && choice.disabled === nextChoice.disabled;
  });
}

function closeActiveSessionStream() {
  activeSessionStream?.close();
  activeSessionStream = null;
  activeStreamSessionId = null;
  lastStreamEventId = null;
  activeStreamTasks = new Map();
  activeStreamTaskRounds = new Map();
  bootstrappingSessionId = null;
  restoringSessionId = null;
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
    if (state.startupStage === 'generating_world') {
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

function waitForRoundNarrationStarted(sessionId: string, round: number) {
  return new Promise<void>((resolve, reject) => {
    const hasStarted = () => {
      const internalState = useGameInternalStore.getState();
      if (internalState.sessionId !== sessionId) {
        return false;
      }

      const roundState = internalState.roundStates[round];
      return Boolean(roundState?.narrationText.trim());
    };

    if (hasStarted()) {
      resolve();
      return;
    }

    const timeoutId = window.setTimeout(() => {
      unsubscribe();
      reject(new Error('开场叙事比预想中更慢一些，请再试一次。'));
    }, FIRST_ROUND_READY_TIMEOUT_MS);

    const unsubscribe = useGameInternalStore.subscribe((state) => {
      const roundState = state.roundStates[round];
      if (
        state.sessionId === sessionId
        && roundState?.narrationText.trim()
      ) {
        window.clearTimeout(timeoutId);
        unsubscribe();
        resolve();
      }
    });
  });
}

function createSlotId() {
  const cryptoApi = globalThis.crypto;
  if (typeof cryptoApi?.randomUUID === 'function') {
    return `slot-${cryptoApi.randomUUID().replace(/-/g, '')}`;
  }

  if (typeof cryptoApi?.getRandomValues === 'function') {
    const randomBytes = new Uint8Array(16);
    cryptoApi.getRandomValues(randomBytes);
    const randomToken = Array.from(randomBytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
    return `slot-${randomToken}`;
  }

  return `slot-${Date.now().toString(16)}${Math.random().toString(16).slice(2)}`;
}

function resetUIState(): GameUIState {
  return {
    character: cloneCharacter(initialCharacter),
    world: cloneWorld(initialWorld),
    story: cloneStory(initialStory),
    stateView: null,
    isLoading: false,
    startupStage: 'idle',
    preparedProfiles: null,
    error: null,
    skipRestoredNarrationAnimation: false,
  };
}

function connectSessionStream(sessionId: string) {
  activeStreamSessionId = sessionId;
  activeSessionStream = openGameSessionStream(
    sessionId,
    {
      onTaskUpdated: (event, lastEventId) => {
        if (activeStreamSessionId !== sessionId) {
          return;
        }
        lastStreamEventId = lastEventId || lastStreamEventId;
        if (
          event.status === 'pending'
          && (event.kind === 'narration' || event.kind === 'protagonist_action')
        ) {
          const internalState = useGameInternalStore.getState();
          const stateView = useGameUIStore.getState().stateView;
          const boundRound = Math.max(
            internalState.displayRound || stateView?.activeTurnId || stateView?.turnIndex || 1,
            1,
          );
          activeStreamTaskRounds.set(event.entity, boundRound);
        }
        const nextTask = applyTaskUpdate(activeStreamTasks, event);
        applyStreamTaskToStores(nextTask, activeStreamTaskRounds.get(event.entity));
      },
      onError: () => {
        if (activeStreamSessionId !== sessionId) {
          return;
        }
        useGameUIStore.setState({
          error: '连接有些不稳定，正在为你续上这段旅程...',
        });
      },
    },
    lastStreamEventId,
  );
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
        title: previousRoundState?.title || stateView?.currentScene || '命运回响',
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

      if (uiState.skipRestoredNarrationAnimation) {
        nextUIState = nextUIState
          ? {
            ...nextUIState,
            skipRestoredNarrationAnimation: false,
          }
          : {
            skipRestoredNarrationAnimation: false,
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
      const previousRoundState = internalState.roundStates[nextRound];
      const nextTurnIndex = Math.max(internalState.turnIndex, nextRound);
      const nextDisplayRound = internalState.displayRound || nextRound;
      const nextTitle = summary?.sceneTitle ?? previousRoundState?.title ?? '命运回响';

      if (
        internalState.turnIndex !== nextTurnIndex
        || internalState.displayRound !== nextDisplayRound
        || !hadRoundState
        || previousRoundState?.title !== nextTitle
      ) {
        const nextRoundState = createRoundState(nextRound, {
          ...(previousRoundState ?? {}),
          title: nextTitle,
          isAwaitingNarration: previousRoundState?.isAwaitingNarration ?? true,
        });
        nextInternalState = {
          turnIndex: nextTurnIndex,
          displayRound: nextDisplayRound,
          roundStates: {
            ...internalState.roundStates,
            [nextRound]: nextRoundState,
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
            summary?.currentEvent
            ?? summary?.newInfo[0]
            ?? summary?.locationStatus
            ?? summary?.description
            ?? stateView.latestBroadcastSummary,
          latestBroadcastItems: nextBroadcastItems,
          isEnding: summary?.isEnding ?? stateView.isEnding,
          endingType: summary?.endingType ?? stateView.endingType,
        };

        if (
          stateView.turnIndex !== nextStateView.turnIndex
          || stateView.activeTurnId !== nextStateView.activeTurnId
          || stateView.currentScene !== nextStateView.currentScene
          || stateView.currentLocation !== nextStateView.currentLocation
          || stateView.protagonistState !== nextStateView.protagonistState
          || stateView.latestBroadcastSummary !== nextStateView.latestBroadcastSummary
          || stateView.latestBroadcastItems !== nextStateView.latestBroadcastItems
          || stateView.isEnding !== nextStateView.isEnding
          || stateView.endingType !== nextStateView.endingType
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

const createGameUIActions = (
  set: StoreApi<GameUIStoreState>['setState'],
  get: StoreApi<GameUIStoreState>['getState'],
): GameUIActions => ({
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
  updateStory: (updates) =>
    set((state) => ({
      story: {
        ...state.story,
        ...updates,
      },
    })),
  clearError: () => set({ error: null }),
  startGame: async () => {
    const runId = ++startupFlowRunId;
    const { character, world } = get();
    closeActiveSessionStream();
    clearStartupStageTimer();
    useGameInternalStore.setState({
      ...initialInternalState,
    });
    useGameValueStore.getState().resetValues();
    set({
      error: null,
      isLoading: true,
      startupStage: 'generating_world',
      preparedProfiles: null,
      stateView: null,
    });
    navigateTo(appRoutes.generating, { replace: true });
    await waitForNextPaint();
    scheduleStartupStageProgress();

    let generatedProfiles: GeneratedProfiles;
    try {
      const generatingStartedAt = Date.now();
      generatedProfiles = await generateProfiles(character, world);
      const generatingElapsed = Date.now() - generatingStartedAt;
      if (generatingElapsed < MIN_GENERATING_PAGE_MS) {
        await sleep(MIN_GENERATING_PAGE_MS - generatingElapsed);
      }
      if (runId !== startupFlowRunId) {
        return;
      }
    } catch (error) {
      if (runId !== startupFlowRunId) {
        return;
      }
      clearStartupStageTimer();
      closeActiveSessionStream();
      useGameInternalStore.setState({
        ...initialInternalState,
      });
      set({
        stateView: null,
        isLoading: false,
        startupStage: 'idle',
        error: error instanceof Error ? error.message : '开启旅程失败。',
      });
      navigateTo(appRoutes.creation, { replace: true });
      throw error;
    }

    clearStartupStageTimer();
    set({
      startupStage: 'creating_session',
      preparedProfiles: generatedProfiles,
    });
    await waitForNextPaint();
    if (runId !== startupFlowRunId) {
      return;
    }
    await get().enterWorld();
  },
  enterWorld: async () => {
    const runId = startupFlowRunId;
    const { character, world, preparedProfiles, startupStage, stateView } = get();
    const { sessionId } = useGameInternalStore.getState();

    if (startupStage === 'ready_to_enter' && sessionId && stateView) {
      if (runId !== startupFlowRunId) {
        return;
      }
      set({
        startupStage: 'idle',
        preparedProfiles: null,
      });
      navigateTo(routeWithSession(appRoutes.gameplay, sessionId), { replace: true });
      return;
    }

    if (!preparedProfiles) {
      throw new Error('设定还在准备中，请稍后再进入。');
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
          keyStoryBeats: preparedProfiles.keyStoryBeats,
        }),
        sleep(MIN_CREATING_SESSION_STAGE_MS),
      ]);
      if (runId !== startupFlowRunId) {
        return;
      }

      useGameInternalStore.setState({
        ...initialInternalState,
        sessionId: created.sessionId,
        displayRound: 1,
        roundStates: {
          1: createRoundState(1, {
            title: '第一轮',
            isAwaitingNarration: true,
          }),
        },
      });
      useGameValueStore.getState().resetValues(1);
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
          latestBroadcastSummary: '旅程已开始，正在展开开场内容...',
          latestBroadcastItems: ['旅程已开始，正在展开开场内容...'],
          latestProtagonistAction: '你还没有做出选择',
          isEnding: false,
          endingType: null,
        },
        error: null,
        isLoading: true,
      });
      connectSessionStream(created.sessionId);
      await submitGameSessionControl(created.sessionId, {
        control: { type: 'continue' },
      });
      await waitForRoundNarrationStarted(created.sessionId, 1);
      if (runId !== startupFlowRunId) {
        return;
      }
      set((state) => ({
        error: null,
        isLoading: false,
        skipRestoredNarrationAnimation: false,
        startupStage: 'ready_to_enter',
        stateView: state.stateView
          ? {
            ...state.stateView,
            phase: 'opening',
          }
          : state.stateView,
      }));
    } catch (error) {
      if (runId !== startupFlowRunId) {
        return;
      }
      closeActiveSessionStream();
      useGameInternalStore.setState({
        ...initialInternalState,
      });
      set({
        stateView: null,
        isLoading: false,
        startupStage: 'ready_to_enter',
        skipRestoredNarrationAnimation: false,
        error: error instanceof Error ? error.message : '进入回响失败。',
      });
      navigateTo(appRoutes.generating, { replace: true });
      throw error;
    }
  },
  bootstrapSession: async () => {
    const { stateView } = get();
    const { sessionId } = useGameInternalStore.getState();

    if (!sessionId || !stateView || stateView.phase !== 'booting') {
      return;
    }

    if (bootstrappingSessionId === sessionId) {
      return;
    }

    bootstrappingSessionId = sessionId;

    try {
      connectSessionStream(sessionId);
      await submitGameSessionControl(sessionId, {
        control: { type: 'continue' },
      });
    } catch (error) {
      if (bootstrappingSessionId === sessionId) {
        bootstrappingSessionId = null;
      }
      set({
        isLoading: false,
        error: error instanceof Error ? error.message : '进入回响失败。',
      });
      throw error;
    }
  },
  submitChoice: async (submission, useObsession = false) => {
    const { sessionId, displayRound, roundStates } = useGameInternalStore.getState();
    const {
      obsessionPoints,
      consumeObsession,
      syncRound,
    } = useGameValueStore.getState();

    if (!sessionId) {
      throw new Error('当前还没有进行中的旅程。');
    }

    if (activeStreamSessionId !== sessionId) {
      throw new Error('内容还在铺展中，请稍后再选择。');
    }

    const nextInput: PlayerActionInput = {
      type: submission.input.type,
      action: submission.input.action.trim(),
    };
    if (!nextInput.action) {
      throw new Error('写下你此刻想做的事。');
    }

    if (useObsession && obsessionPoints <= 0) {
      throw new Error('执念点不足。');
    }

    const activeRound = Math.max(displayRound || 1, 1);
    const nextRound = activeRound + 1;
    const currentRoundChoices = roundStates[activeRound]?.choices ?? [];
    if (
      nextInput.type === 'selected_option'
      && !currentRoundChoices.some((choice) => choice.action === nextInput.action)
    ) {
      throw new Error('这个选项已失效，请重新选择。');
    }
    const selectedChoiceText = useObsession
      ? `${submission.displayText} [执念]`
      : submission.displayText;

    const previousRoundState = roundStates[activeRound];
    const previousNextRoundState = roundStates[nextRound];

    set({
      isLoading: true,
      error: null,
      skipRestoredNarrationAnimation: false,
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
      if (useObsession) {
        consumeObsession();
      }
      syncRound(nextRound);
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
  createSave: async (title) => {
    const { sessionId } = useGameInternalStore.getState();
    if (!sessionId) {
      throw new Error('此刻还没有可保存的进度。');
    }

    const normalizedTitle = title?.trim();
    set({
      error: null,
      isLoading: true,
    });

    try {
      const saved = await exportGameSaveArchive(sessionId, {
        title: normalizedTitle || undefined,
      });
      const slotId = createSlotId();
      writeStoredSaveArchive(slotId, saved.compressedArchive);
      upsertStoredSaveSlot({
        slotId,
        sessionId: saved.sessionId,
        title: saved.title,
        createdAt: saved.createdAt,
        updatedAt: saved.updatedAt,
      });
      set({
        isLoading: false,
      });
      return slotId;
    } catch (error) {
      set({
        isLoading: false,
        error: error instanceof Error ? error.message : '存档失败。',
      });
      throw error;
    }
  },
  loadSave: async (saveId) => {
    const slotId = saveId.trim();
    if (!slotId) {
      throw new Error('未找到要读取的存档。');
    }

    closeActiveSessionStream();
    clearStartupStageTimer();
    useGameInternalStore.setState({
      ...initialInternalState,
    });
    set({
      error: null,
      isLoading: true,
    });

    try {
      const archive = readStoredSaveArchive(slotId);
      if (!archive) {
        throw new Error('没有找到这份存档，请确认它仍然存在。');
      }

      const loaded = await loadGameSessionFromArchive({
        compressedArchive: archive,
      });
      useGameInternalStore.setState(internalStateFromSession(loaded));
      useGameValueStore.getState().resetValues(effectiveDisplayRound(loaded));
      set({
        stateView: stateViewFromSession(loaded),
        isLoading: false,
        startupStage: 'idle',
        preparedProfiles: null,
        error: null,
        skipRestoredNarrationAnimation: true,
      });
      connectSessionStream(loaded.sessionId);
      navigateTo(routeWithSession(appRoutes.gameplay, loaded.sessionId), { replace: true });
    } catch (error) {
      closeActiveSessionStream();
      useGameInternalStore.setState({
        ...initialInternalState,
      });
      set({
        ...resetUIState(),
        error: error instanceof Error ? error.message : '读取存档失败。',
      });
      navigateTo(appRoutes.lobby, { replace: true });
      throw error;
    }
  },
  restoreSession: async (sessionId) => {
    const targetSessionId = sessionId.trim();
    if (!targetSessionId) {
      throw new Error('未找到要恢复的旅程编号。');
    }

    const currentSessionId = useGameInternalStore.getState().sessionId;
    if (currentSessionId === targetSessionId && get().stateView) {
      if (activeStreamSessionId !== targetSessionId) {
        closeActiveSessionStream();
        connectSessionStream(targetSessionId);
      }
      return;
    }

    if (restoringSessionId === targetSessionId) {
      return;
    }

    closeActiveSessionStream();
    clearStartupStageTimer();
    startupFlowRunId += 1;
    restoringSessionId = targetSessionId;
    useGameInternalStore.setState({
      ...initialInternalState,
    });
    set({
      stateView: null,
      isLoading: true,
      startupStage: 'idle',
      preparedProfiles: null,
      error: null,
      skipRestoredNarrationAnimation: true,
    });

    try {
      const loaded = await getGameSession(targetSessionId);
      if (restoringSessionId !== targetSessionId) {
        return;
      }

      useGameInternalStore.setState(internalStateFromSession(loaded));
      useGameValueStore.getState().resetValues(effectiveDisplayRound(loaded));
      set({
        stateView: stateViewFromSession(loaded),
        isLoading: false,
        startupStage: 'idle',
        preparedProfiles: null,
        error: null,
        skipRestoredNarrationAnimation: true,
      });
      connectSessionStream(loaded.sessionId);
      restoringSessionId = null;
    } catch (error) {
      if (restoringSessionId !== targetSessionId) {
        return;
      }

      closeActiveSessionStream();
      useGameInternalStore.setState({
        ...initialInternalState,
      });
      set({
        ...resetUIState(),
        error: error instanceof Error ? error.message : '这段旅程已经暂时无法续上。',
      });
      navigateTo(appRoutes.lobby, { replace: true });
      throw error;
    }
  },
  cloneSharedSession: async (sourceSessionId) => {
    const targetSessionId = sourceSessionId.trim();
    if (!targetSessionId) {
      throw new Error('未找到要复制的旅程编号。');
    }

    if (activeCloneRequest?.sourceSessionId === targetSessionId) {
      return activeCloneRequest.promise;
    }

    const clonePromise = (async () => {
      closeActiveSessionStream();
      clearStartupStageTimer();
      startupFlowRunId += 1;
      restoringSessionId = null;
      useGameInternalStore.setState({
        ...initialInternalState,
      });
      set({
        stateView: null,
        isLoading: true,
        startupStage: 'idle',
        preparedProfiles: null,
        error: null,
        skipRestoredNarrationAnimation: true,
      });

      try {
        const cloned = await cloneGameSession(targetSessionId);

        useGameInternalStore.setState(internalStateFromSession(cloned));
        useGameValueStore.getState().resetValues(effectiveDisplayRound(cloned));
        set({
          stateView: stateViewFromSession(cloned),
          isLoading: false,
          startupStage: 'idle',
          preparedProfiles: null,
          error: null,
          skipRestoredNarrationAnimation: true,
        });
        connectSessionStream(cloned.sessionId);
        return {
          sessionId: cloned.sessionId,
          isEnding: cloned.worldState.isEnding,
        };
      } catch (error) {
        closeActiveSessionStream();
        useGameInternalStore.setState({
          ...initialInternalState,
        });
        set({
          ...resetUIState(),
          error: error instanceof Error ? error.message : '这段旅程暂时无法复制。',
        });
        navigateTo(appRoutes.lobby, { replace: true });
        throw error;
      } finally {
        if (activeCloneRequest?.sourceSessionId === targetSessionId) {
          activeCloneRequest = null;
        }
      }
    })();

    activeCloneRequest = {
      sourceSessionId: targetSessionId,
      promise: clonePromise,
    };

    return clonePromise;
  },
  resetGame: () => {
    closeActiveSessionStream();
    clearStartupStageTimer();
    useGameInternalStore.setState({
      ...initialInternalState,
    });
    useGameValueStore.getState().resetValues();
    set((state) => ({
      ...state,
      ...resetUIState(),
    }));
  },
});

export const useGameUIStore = create<GameUIStoreState>((set, get) => ({
  ...initialUIState,
  ...createGameUIActions(set, get),
}));
