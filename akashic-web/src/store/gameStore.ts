import { create } from 'zustand';
import {
  createGameSession,
  openGameSessionStream,
  submitGameSessionControl,
} from '../lib/api';
import type {
  ArchiveListItem,
  Character,
  RuntimeStateView,
  SaveListItem,
  StoryNode,
  TaskView,
  World,
} from '../lib/api';
import {
  applyTaskUpdate,
  buildArchiveItem,
  buildEndingNode,
  buildSaveItem,
  buildStateView,
  buildStoryNode,
  cloneCharacter,
  cloneWorld,
  DEMO_ENDINGS,
  DEMO_NODES,
  initialCharacter,
  initialWorld,
  isDemoNodeId,
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
} from '../utils/gameStoreHelpers';

export type GameState = 'lobby' | 'creation' | 'playing';

interface GameStoreState {
  gameState: GameState;
  sessionId: string | null;
  character: Character;
  world: World;
  currentNode: StoryNode | null;
  streamedNarrationText: string;
  streamedNarrationStatus: TaskView['status'] | null;
  streamedFatePlanningRaw: string;
  streamedFatePlanningJson: JsonValue | null;
  streamedProtagonistActionRaw: string;
  streamedProtagonistActionJson: JsonValue | null;
  stateView: RuntimeStateView | null;
  obsessionPoints: number;
  intuitionPoints: number;
  daysLeft: number;
  worldNews: string | null;
  turnIndex: number;
  saves: SaveListItem[];
  archives: ArchiveListItem[];
  latestSaveId: string | null;
  latestArchiveId: string | null;
  isLoading: boolean;
  error: string | null;
  setGameState: (state: GameState) => void;
  updateCharacter: (updates: Partial<Character>) => void;
  updateWorld: (updates: Partial<World>) => void;
  clearError: () => void;
  startGame: () => Promise<void>;
  submitChoice: (choiceId: string, useObsession?: boolean) => Promise<void>;
  previewChoice: (choiceId: string) => Promise<string>;
  createSave: (title?: string) => Promise<string>;
  loadSave: (saveId: string) => Promise<void>;
  resetGame: () => void;
}

interface SaveSnapshot {
  sessionId: string;
  character: Character;
  world: World;
  currentNodeId: DemoNodeId;
  turnIndex: number;
  obsessionPoints: number;
  intuitionPoints: number;
  daysLeft: number;
  worldNews: string | null;
}

const seededArchives: ArchiveListItem[] = [
  {
    archiveId: 'archive-seeded-1',
    title: '旧馆藏 · 雨夜抄本',
    tag: '示例档案',
    era: '东方玄幻',
    summary: '一份预置的演示馆藏，用来展示本地归档在无后端时的视觉布局。',
    coverImage: STORY_IMAGES.corridor,
    createdAt: '2026-05-18T00:00:00.000Z',
  },
];

const seededSaves: SaveListItem[] = [
  {
    saveId: 'save-seeded-1',
    sessionId: 'demo-seeded',
    title: '示例存档 · 港区雨幕',
    characterName: '演示旅人',
    background: '寻梦的学者',
    era: '蒸汽朋克',
    turnIndex: 1,
    summary: '用于展示“进行中存档”样式，不依赖任何后端返回。',
    coverImage: STORY_IMAGES.opening,
    savedAt: '2026-05-18T00:00:00.000Z',
  },
];

const saveSnapshots = new Map<string, SaveSnapshot>([
  [
    'save-seeded-1',
    {
      sessionId: 'demo-seeded',
      character: {
        name: '演示旅人',
        gender: '保密',
        age: 22,
        appearance: '披着仍带雨意的长风衣，袖口藏着一支记事银笔',
        traits: { courage: 56, rationality: 68, altruism: 61 },
        background: '寻梦的学者',
      },
      world: {
        era: '蒸汽朋克',
        coreConflict: '旧档案馆深处正在泄露不属于这个时代的预言',
        specialRules: [],
      },
      currentNodeId: 'opening',
      turnIndex: 1,
      obsessionPoints: 3,
      intuitionPoints: 4,
      daysLeft: 6,
      worldNews: DEMO_NODES.opening.news,
    },
  ],
]);

const initialState = {
  gameState: 'lobby' as GameState,
  sessionId: null,
  character: initialCharacter,
  world: initialWorld,
  currentNode: null,
  streamedNarrationText: '',
  streamedNarrationStatus: null,
  streamedFatePlanningRaw: '',
  streamedFatePlanningJson: null,
  streamedProtagonistActionRaw: '',
  streamedProtagonistActionJson: null,
  stateView: null,
  obsessionPoints: 3,
  intuitionPoints: 5,
  daysLeft: 7,
  worldNews: null,
  turnIndex: 0,
  saves: seededSaves,
  archives: seededArchives,
  latestSaveId: null,
  latestArchiveId: seededArchives[0]?.archiveId ?? null,
  isLoading: false,
  error: null,
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

function resetPlayState(state: GameStoreState) {
  return {
    gameState: 'lobby' as GameState,
    sessionId: null,
    character: cloneCharacter(initialCharacter),
    world: cloneWorld(initialWorld),
    currentNode: null,
    streamedNarrationText: '',
    streamedNarrationStatus: null,
    streamedFatePlanningRaw: '',
    streamedFatePlanningJson: null,
    streamedProtagonistActionRaw: '',
    streamedProtagonistActionJson: null,
    stateView: null,
    obsessionPoints: 3,
    intuitionPoints: 5,
    daysLeft: 7,
    worldNews: null,
    turnIndex: 0,
    latestSaveId: null,
    latestArchiveId: state.latestArchiveId,
    isLoading: false,
    error: null,
  };
}

function applyStreamTaskToState(
  task: TaskView,
  set: (partial: Partial<GameStoreState> | ((state: GameStoreState) => Partial<GameStoreState>)) => void,
) {
  set((state) => {
    const isLoading = task.status === 'pending' || task.status === 'running';

    switch (task.kind) {
      case 'narration': {
        const nextText = taskText(task);
        if (!nextText) {
          return { isLoading };
        }

        return {
          isLoading,
          streamedNarrationText: nextText,
          streamedNarrationStatus: task.status,
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
              currentScene: taskLabel(task.kind),
              latestHistory: nextText,
              latestBroadcastSummary: nextText,
            }
            : null,
        };
      }
      case 'fate_planning': {
        const raw = taskRawContent(task);
        const parsed = parseJsonValue(raw);
        const summary = summarizeFatePlanning(parsed);

        return {
          isLoading,
          streamedFatePlanningRaw: raw,
          streamedFatePlanningJson: parsed,
          worldNews:
            summary?.currentEvent ??
            summary?.newInfo[0] ??
            summary?.locationStatus ??
            state.worldNews,
          stateView: state.stateView
            ? {
              ...state.stateView,
              currentScene: summary?.sceneTitle ?? taskLabel(task.kind),
              currentLocation: summary?.locationName ?? state.stateView.currentLocation,
              protagonistState: summary?.protagonistCondition ?? state.stateView.protagonistState,
              latestBroadcastSummary: summary?.description ?? state.stateView.latestBroadcastSummary,
            }
            : null,
        };
      }
      case 'protagonist_action': {
        const raw = taskRawContent(task);
        const parsed = parseJsonValue(raw);
        const nextChoices = protagonistActionChoices(task);

        return {
          isLoading,
          streamedProtagonistActionRaw: raw,
          streamedProtagonistActionJson: parsed,
          currentNode: state.currentNode
            ? {
              ...state.currentNode,
              choices: nextChoices ?? state.currentNode.choices,
            }
            : null,
          stateView: state.stateView
            ? {
              ...state.stateView,
              currentScene: taskLabel(task.kind),
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

export const useGameStore = create<GameStoreState>((set, get) => ({
  ...initialState,
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
    set({
      sessionId: null,
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
        latestBroadcastSummary: world.coreConflict,
        latestProtagonistAction: '尚未做出选择',
      },
      streamedNarrationText: '',
      streamedNarrationStatus: null,
      streamedFatePlanningRaw: '',
      streamedFatePlanningJson: null,
      streamedProtagonistActionRaw: '',
      streamedProtagonistActionJson: null,
      obsessionPoints: 3,
      intuitionPoints: 5,
      daysLeft: 7,
      worldNews: '正在创建会话并唤起第一轮命运...',
      turnIndex: 0,
      latestSaveId: null,
      error: null,
      gameState: 'playing',
      isLoading: true,
    });

    try {
      // 创建会话
      const created = await createGameSession(character, world);
      activeStreamSessionId = created.sessionId;

      set({
        sessionId: created.sessionId,
        worldNews: '会话已建立，正在推进第一轮...',
      });

      activeSessionStream = openGameSessionStream(
        created.sessionId,
        {
          onTaskUpdated: (event, lastEventId) => {
            if (activeStreamSessionId !== created.sessionId) {
              return;
            }
            lastStreamEventId = lastEventId || lastStreamEventId;
            const nextTask = applyTaskUpdate(activeStreamTasks, event);
            applyStreamTaskToState(nextTask, set);
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
    const {
      sessionId,
      character,
      world,
      currentNode,
      obsessionPoints,
      intuitionPoints,
      daysLeft,
      turnIndex,
      archives,
    } = get();

    if (!sessionId || !currentNode) {
      throw new Error('当前没有可推进的演示剧情。');
    }

    if (activeStreamSessionId === sessionId) {
      const nextObsession = useObsession ? Math.max(0, obsessionPoints - 1) : obsessionPoints;
      const nextDaysLeft = Math.max(1, daysLeft - 1);

      set((state) => ({
        isLoading: true,
        obsessionPoints: nextObsession,
        intuitionPoints,
        daysLeft: nextDaysLeft,
        streamedNarrationText: '',
        streamedNarrationStatus: null,
        streamedFatePlanningRaw: '',
        streamedFatePlanningJson: null,
        streamedProtagonistActionRaw: '',
        streamedProtagonistActionJson: null,
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
    const nextDaysLeft = Math.max(1, daysLeft - 1);

    if (choice.nextNodeId) {
      const nextNode = buildStoryNode(choice.nextNodeId, character, world);
      set({
        currentNode: nextNode,
        stateView: buildStateView(choice.nextNodeId, turnIndex + 1, nextNode.text, choice.text),
        worldNews: DEMO_NODES[choice.nextNodeId].news,
        obsessionPoints: nextObsession,
        intuitionPoints,
        daysLeft: nextDaysLeft,
        turnIndex: turnIndex + 1,
        error: null,
        gameState: 'playing',
      });
      return;
    }

    if (!choice.endingId) {
      throw new Error('演示剧情缺少结局配置。');
    }

    const endingNode = buildEndingNode(choice.endingId, character, world);
    const ending = DEMO_ENDINGS[choice.endingId];
    const archiveItem = buildArchiveItem(choice.endingId, character, world, sessionId);

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
        latestBroadcastSummary: ending.summary,
        latestProtagonistAction: choice.text,
      },
      archives: [archiveItem, ...archives.filter((item) => item.archiveId !== archiveItem.archiveId)],
      latestArchiveId: archiveItem.archiveId,
      obsessionPoints: nextObsession,
      intuitionPoints,
      daysLeft: nextDaysLeft,
      turnIndex: turnIndex + 1,
      worldNews: '命运收束：本次旅程已被写入本地归档演示数据。',
      error: null,
      gameState: 'playing',
    });
  },
  previewChoice: async (choiceId) => {
    throw new Error('演示直觉点已耗尽。');
  },
  createSave: async (title) => {
    throw new Error('当前没有可保存的演示旅程。');
  },
  loadSave: async (saveId) => {

  },
  resetGame: () => {
    closeActiveSessionStream();
    set((state) => ({
      ...state,
      ...resetPlayState(state),
    }));
  },
}));
