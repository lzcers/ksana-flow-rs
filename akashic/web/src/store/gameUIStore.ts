import { create } from 'zustand';
import type { StoreApi } from 'zustand';
import {
  createGameSession,
  exportGameSaveArchive,
  generateProfiles,
  loadGameSessionFromArchive,
  openGameSessionStream,
  submitGameSessionControl,
} from '../lib/api';
import type {
  Character,
  GameSessionWorldStateData,
  GeneratedProfiles,
  PlayerActionInput,
  RuntimeStateView,
  SessionRoundHistoryData,
  TaskView,
  World,
} from '../lib/api';
import {
  readStoredSaveArchive,
  upsertStoredSaveSlot,
  writeStoredSaveArchive,
} from '../lib/saveSlots';
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
  toChoiceFromSession,
} from './gameStoreHelpers';

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
  // 操作：创建当前进度的存档。
  createSave: (title?: string) => Promise<string>;
  // 操作：加载指定存档。
  loadSave: (saveId: string) => Promise<void>;
  // 操作：重置本地游戏状态并关闭流连接。
  resetGame: () => void;
}

export type GameUIStoreState = GameUIState & GameUIActions;

const initialUIState: GameUIState = {
  gameState: 'lobby',
  character: initialCharacter,
  world: initialWorld,
  stateView: null,
  isLoading: false,
  startupStage: 'idle',
  preparedProfiles: null,
  error: null,
};

let activeSessionStream: EventSource | null = null;
let activeStreamSessionId: string | null = null;
let lastStreamEventId: string | null = null;
let activeStreamTasks = new Map<string, TaskView>();
let activeStreamTaskRounds = new Map<string, number>();
let startupStageTimer: number | null = null;

const MIN_GENERATING_PAGE_MS = 1200;
const MIN_CREATING_SESSION_STAGE_MS = 450;

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
    gameState: 'lobby',
    character: cloneCharacter(initialCharacter),
    world: cloneWorld(initialWorld),
    stateView: null,
    isLoading: false,
    startupStage: 'idle',
    preparedProfiles: null,
    error: null,
  };
}

function effectiveDisplayRound(session: GameSessionWorldStateData): number {
  if (session.phase === 'awaiting_player_choice') {
    return Math.max(session.activeTurnId, 1);
  }

  return Math.max(session.turnIndex, session.activeTurnId, 1);
}

function latestHistoryFromSession(session: GameSessionWorldStateData): string {
  return session.latestNarration.trim()
    || session.worldState.description.trim()
    || STREAM_PLACEHOLDER_TEXT;
}

function latestBroadcastItemsFromSession(session: GameSessionWorldStateData): string[] {
  const nextItems = session.worldState.newInfo
    .map((item) => item.trim())
    .filter(Boolean);
  if (nextItems.length > 0) {
    return nextItems;
  }

  const fallback = session.worldState.currentEvent.trim() || session.worldState.description.trim();
  return fallback ? [fallback] : [];
}

function stateViewFromSession(session: GameSessionWorldStateData): GameUIState['stateView'] {
  const latestBroadcastItems = latestBroadcastItemsFromSession(session);
  return {
    gameState: 'playing',
    phase: session.phase,
    turnIndex: session.turnIndex,
    activeTurnId: session.activeTurnId,
    currentLocation: session.worldState.locationName || '命运现场',
    currentScene: session.worldState.sceneTitle || '命运回响',
    protagonistState: session.worldState.protagonistCondition || '命运仍在酝酿',
    npcsState: session.worldState.currentEvent || '诸多回响正在汇聚',
    latestHistory: latestHistoryFromSession(session),
    latestBroadcastSummary:
      session.worldState.currentEvent
      || session.worldState.description
      || '会话已恢复',
    latestBroadcastItems,
    latestProtagonistAction: session.currentProtagonistAction || '尚未做出选择',
  };
}

function internalStateFromSession(session: GameSessionWorldStateData): GameInternalState {
  const round = effectiveDisplayRound(session);
  if (session.history.length > 0) {
    const roundStates = session.history.reduce<Record<number, RoundState>>((acc, entry) => {
      acc[entry.round] = roundStateFromHistoryEntry(entry, session, round);
      return acc;
    }, {});

    if (!roundStates[round]) {
      roundStates[round] = currentRoundStateFromSession(session, round);
    }

    return {
      sessionId: session.sessionId,
      turnIndex: session.turnIndex,
      displayRound: round,
      roundStates,
    };
  }

  return {
    sessionId: session.sessionId,
    turnIndex: session.turnIndex,
    displayRound: round,
    roundStates: {
      [round]: currentRoundStateFromSession(session, round),
    },
  };
}

function roundStateFromHistoryEntry(
  entry: SessionRoundHistoryData,
  session: GameSessionWorldStateData,
  currentRound: number,
): RoundState {
  const isCurrentRound = entry.round === currentRound;
  const choices = (isCurrentRound ? session.choices : entry.choices).map(toChoiceFromSession);
  const selectedChoiceText = entry.selectedChoiceText?.trim()
    || deriveSelectedChoiceText(entry)
    || null;
  const narrationText = entry.narrationText.trim()
    || (isCurrentRound ? latestHistoryFromSession(session) : '');

  return createRoundState(entry.round, {
    narrationText,
    narrationStatus: isCurrentRound && session.currentTask?.kind === 'narration'
      ? session.currentTask.status
      : entry.narrationText.trim()
        ? 'done'
        : null,
    choices,
    choicesStatus: choices.length > 0 ? 'ready' : 'idle',
    selectedChoiceText,
    isAwaitingNarration: false,
  });
}

function deriveSelectedChoiceText(entry: SessionRoundHistoryData): string | null {
  const committedAction = entry.committedAction?.trim();
  if (!committedAction) {
    return null;
  }

  const matchedChoice = entry.choices.find((choice) => choice.option.action === committedAction);
  return matchedChoice?.option.title || committedAction;
}

function currentRoundStateFromSession(
  session: GameSessionWorldStateData,
  round: number,
): RoundState {
  return {
    ...createRoundState(round, {
      narrationText: latestHistoryFromSession(session),
      narrationStatus: session.currentTask?.kind === 'narration' ? session.currentTask.status : null,
      choices: session.choices.map(toChoiceFromSession),
      choicesStatus: session.choices.length > 0 ? 'ready' : 'idle',
      selectedChoiceText: null,
      isAwaitingNarration: false,
    }),
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
          error: '叙事流连接出现波动，正在尝试恢复...',
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
            summary?.currentEvent
            ?? summary?.newInfo[0]
            ?? summary?.locationStatus
            ?? summary?.description
            ?? stateView.latestBroadcastSummary,
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

const createGameUIActions = (
  set: StoreApi<GameUIStoreState>['setState'],
  get: StoreApi<GameUIStoreState>['getState'],
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
    useGameValueStore.getState().resetValues();
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
          latestBroadcastSummary: '会话已建立，正在推进第一轮...',
          latestBroadcastItems: ['会话已建立，正在推进第一轮...'],
          latestProtagonistAction: '尚未做出选择',
        },
        error: null,
        gameState: 'playing',
        isLoading: true,
        startupStage: 'idle',
        preparedProfiles: null,
      });

      connectSessionStream(created.sessionId);

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
      consumeObsession,
      syncRound,
    } = useGameValueStore.getState();

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
      throw new Error('当前选择不存在。');
    }
    const selectedChoiceText = useObsession
      ? `${submission.displayText} [执念]`
      : submission.displayText;

    const previousRoundState = roundStates[activeRound];
    const previousNextRoundState = roundStates[nextRound];

    set({
      isLoading: true,
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
      throw new Error('当前没有可保存的演示旅程。');
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
      const slotId = `slot-${crypto.randomUUID().split('-').join('')}`;
      writeStoredSaveArchive(slotId, saved.archive);
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
      throw new Error('存档槽 ID 不能为空。');
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
        throw new Error('未找到本地存档内容。');
      }

      const loaded = await loadGameSessionFromArchive(archive);
      useGameInternalStore.setState(internalStateFromSession(loaded));
      useGameValueStore.getState().resetValues(effectiveDisplayRound(loaded));
      set({
        gameState: 'playing',
        stateView: stateViewFromSession(loaded),
        isLoading: false,
        startupStage: 'idle',
        preparedProfiles: null,
        error: null,
      });
      connectSessionStream(loaded.sessionId);
    } catch (error) {
      closeActiveSessionStream();
      useGameInternalStore.setState({
        ...initialInternalState,
      });
      set({
        ...resetUIState(),
        gameState: 'lobby',
        error: error instanceof Error ? error.message : '读取存档失败。',
      });
      throw error;
    }
  },
  resetGame: () => {
    closeActiveSessionStream();
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
