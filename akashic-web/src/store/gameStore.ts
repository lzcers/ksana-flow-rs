import { create } from 'zustand';
import type {
  ArchiveListItem,
  Character,
  CreateGameSessionData,
  EndingData,
  GameSessionSnapshot,
  RuntimeStateView,
  SaveListItem,
  StoryStreamEvent,
  StoryStreamEventName,
  StoryNode,
  World,
} from '../lib/api';
import {
  createGameSession,
  createIntuitionPreview,
  createSave as createSaveRequest,
  getGameSession,
  getGameSessionEnding,
  listArchives,
  listSaves,
  loadSave as loadSaveRequest,
  submitChoice as submitChoiceRequest,
} from '../lib/api';

export type GameState = 'lobby' | 'creation' | 'playing' | 'ending' | 'corridor';

interface GameStoreState {
  gameState: GameState;
  sessionId: string | null;
  character: Character;
  world: World;
  currentNode: StoryNode | null;
  endingData: EndingData | null;
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
  refreshSession: () => Promise<void>;
  submitChoice: (choiceId: string, useObsession?: boolean) => Promise<void>;
  previewChoice: (choiceId: string) => Promise<string>;
  fetchEnding: () => Promise<void>;
  createSave: (title?: string) => Promise<string>;
  fetchCorridorData: () => Promise<void>;
  loadSave: (saveId: string) => Promise<void>;
  resetGame: () => void;
}

const initialCharacter: Character = {
  name: '',
  gender: '保密',
  age: 18,
  appearance: '',
  traits: {
    courage: 50,
    rationality: 50,
    altruism: 50,
  },
  background: '',
};

const initialWorld: World = {
  era: '蒸汽朋克',
  coreConflict: '资源枯竭与永生诱惑',
  specialRules: [],
};

const initialState: Pick<
  GameStoreState,
  | 'gameState'
  | 'sessionId'
  | 'character'
  | 'world'
  | 'currentNode'
  | 'endingData'
  | 'stateView'
  | 'obsessionPoints'
  | 'intuitionPoints'
  | 'daysLeft'
  | 'worldNews'
  | 'turnIndex'
  | 'saves'
  | 'archives'
  | 'latestSaveId'
  | 'latestArchiveId'
  | 'isLoading'
  | 'error'
> = {
  gameState: 'lobby',
  sessionId: null,
  character: initialCharacter,
  world: initialWorld,
  currentNode: null,
  endingData: null,
  stateView: null,
  obsessionPoints: 3,
  intuitionPoints: 5,
  daysLeft: 30,
  worldNews: null,
  turnIndex: 0,
  saves: [],
  archives: [],
  latestSaveId: null,
  latestArchiveId: null,
  isLoading: false,
  error: null,
};

function applySnapshot(snapshot: GameSessionSnapshot) {
  return {
    sessionId: snapshot.sessionId,
    character: snapshot.character,
    world: snapshot.world,
    currentNode: snapshot.currentNode,
    stateView: snapshot.stateView,
    obsessionPoints: snapshot.resources.obsessionPoints,
    intuitionPoints: snapshot.resources.intuitionPoints,
    daysLeft: snapshot.resources.daysLeft,
    worldNews: snapshot.resources.worldNews,
    turnIndex: snapshot.stateView.turnIndex,
  };
}

function createSessionToSnapshot(session: CreateGameSessionData): GameSessionSnapshot {
  return {
    sessionId: session.sessionId,
    status: 'active',
    character: session.character,
    world: session.world,
    resources: session.resources,
    currentNode: session.currentNode,
    stateView: session.stateView,
    endingStatus: 'pending',
  };
}

function findStoryEvent<Name extends StoryStreamEventName>(
  events: StoryStreamEvent[],
  eventName: Name,
): StoryStreamEvent<Name> | undefined {
  return events.find((event): event is StoryStreamEvent<Name> => event.event === eventName);
}

function requireStoryEvent<Name extends StoryStreamEventName>(
  events: StoryStreamEvent[],
  eventName: Name,
  fallbackMessage: string,
): StoryStreamEvent<Name> {
  const matched = findStoryEvent(events, eventName);
  if (!matched) {
    throw new Error(fallbackMessage);
  }

  return matched;
}

export const useGameStore = create<GameStoreState>((set, get) => ({
  ...initialState,
  setGameState: (state) => set({ gameState: state }),
  updateCharacter: (updates) =>
    set((state) => ({
      character: { ...state.character, ...updates },
    })),
  updateWorld: (updates) =>
    set((state) => ({
      world: { ...state.world, ...updates },
    })),
  clearError: () => set({ error: null }),
  startGame: async () => {
    const { character, world } = get();
    set({ isLoading: true, error: null });

    try {
      const data = await createGameSession({ character, world });
      const snapshot = createSessionToSnapshot(data);

      set({
        ...applySnapshot(snapshot),
        endingData: null,
        latestArchiveId: null,
        gameState: 'playing',
      });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : '创建会话失败。' });
      throw error;
    } finally {
      set({ isLoading: false });
    }
  },
  refreshSession: async () => {
    const { sessionId } = get();
    if (!sessionId) return;

    set({ isLoading: true, error: null });
    try {
      const events = await getGameSession(sessionId);
      const snapshot = requireStoryEvent(
        events,
        'session.snapshot',
        '服务端没有返回会话快照。',
      ).data;

      set({
        ...applySnapshot(snapshot),
        endingData: null,
        gameState: snapshot.endingStatus === 'ready' ? 'ending' : 'playing',
      });

      if (snapshot.endingStatus === 'ready') {
        const endingEvents = await getGameSessionEnding(sessionId);
        const ending = requireStoryEvent(
          endingEvents,
          'ending.ready',
          '服务端没有返回结局内容。',
        ).data;
        set({ endingData: ending.ending });
      }
    } catch (error) {
      set({ error: error instanceof Error ? error.message : '刷新会话失败。' });
      throw error;
    } finally {
      set({ isLoading: false });
    }
  },
  submitChoice: async (choiceId, useObsession = false) => {
    const { sessionId } = get();
    if (!sessionId) {
      throw new Error('当前没有可推进的会话。');
    }

    set({ isLoading: true, error: null });
    try {
      const events = await submitChoiceRequest(sessionId, { choiceId, useObsession });
      const snapshot = requireStoryEvent(
        events,
        'session.snapshot',
        '服务端没有返回推进后的会话快照。',
      ).data;
      const endingEvent = findStoryEvent(events, 'ending.ready');
      set(applySnapshot(snapshot));

      if (snapshot.endingStatus === 'ready') {
        const ending =
          endingEvent?.data ??
          requireStoryEvent(
            await getGameSessionEnding(sessionId),
            'ending.ready',
            '服务端没有返回结局内容。',
          ).data;

        set({
          endingData: ending.ending,
          gameState: 'ending',
          latestArchiveId: `archive-${sessionId}`,
        });
      } else {
        set({ endingData: null, gameState: 'playing' });
      }
    } catch (error) {
      set({ error: error instanceof Error ? error.message : '推进剧情失败。' });
      throw error;
    } finally {
      set({ isLoading: false });
    }
  },
  previewChoice: async (choiceId) => {
    const { sessionId } = get();
    if (!sessionId) {
      throw new Error('当前没有可预览的会话。');
    }

    set({ isLoading: true, error: null });
    try {
      const events = await createIntuitionPreview(sessionId, choiceId);
      const data = requireStoryEvent(
        events,
        'intuition.preview',
        '服务端没有返回直觉预览结果。',
      ).data;

      set({
        intuitionPoints: data.resources.intuitionPoints,
        obsessionPoints: data.resources.obsessionPoints,
        daysLeft: data.resources.daysLeft,
        worldNews: data.resources.worldNews,
      });
      return data.previewText;
    } catch (error) {
      set({ error: error instanceof Error ? error.message : '生成直觉预览失败。' });
      throw error;
    } finally {
      set({ isLoading: false });
    }
  },
  fetchEnding: async () => {
    const { sessionId } = get();
    if (!sessionId) return;

    set({ isLoading: true, error: null });
    try {
      const events = await getGameSessionEnding(sessionId);
      const data = requireStoryEvent(
        events,
        'ending.ready',
        '服务端没有返回结局内容。',
      ).data;

      set({
        endingData: data.ending,
        gameState: 'ending',
        latestArchiveId: `archive-${sessionId}`,
      });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : '加载结局失败。' });
      throw error;
    } finally {
      set({ isLoading: false });
    }
  },
  createSave: async (title) => {
    const { sessionId, character, turnIndex } = get();
    if (!sessionId) {
      throw new Error('当前没有可保存的会话。');
    }

    set({ isLoading: true, error: null });
    try {
      const save = await createSaveRequest({
        sessionId,
        title: title?.trim() || `${character.name || '无名旅人'} · 第 ${turnIndex || 1} 幕`,
        autoGenerateShareCard: false,
      });
      const saves = await listSaves();
      set({
        saves: saves.items,
        latestSaveId: save.saveId,
      });
      return save.saveId;
    } catch (error) {
      set({ error: error instanceof Error ? error.message : '创建存档失败。' });
      throw error;
    } finally {
      set({ isLoading: false });
    }
  },
  fetchCorridorData: async () => {
    set({ isLoading: true, error: null });
    try {
      const [saves, archives] = await Promise.all([listSaves(), listArchives()]);
      set({
        saves: saves.items,
        archives: archives.items,
      });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : '加载回廊数据失败。' });
      throw error;
    } finally {
      set({ isLoading: false });
    }
  },
  loadSave: async (saveId) => {
    set({ isLoading: true, error: null });
    try {
      const loaded = await loadSaveRequest(saveId);
      const events = await getGameSession(loaded.sessionId);
      const snapshot = requireStoryEvent(
        events,
        'session.snapshot',
        '服务端没有返回读档后的会话快照。',
      ).data;

      set({
        ...applySnapshot(snapshot),
        latestSaveId: saveId,
        gameState: snapshot.endingStatus === 'ready' ? 'ending' : 'playing',
      });

      if (snapshot.endingStatus === 'ready') {
        const endingEvents = await getGameSessionEnding(loaded.sessionId);
        const ending = requireStoryEvent(
          endingEvents,
          'ending.ready',
          '服务端没有返回结局内容。',
        ).data;
        set({ endingData: ending.ending, latestArchiveId: `archive-${loaded.sessionId}` });
      } else {
        set({ endingData: null });
      }
    } catch (error) {
      set({ error: error instanceof Error ? error.message : '读档失败。' });
      throw error;
    } finally {
      set({ isLoading: false });
    }
  },
  resetGame: () =>
    set({
      ...initialState,
      character: initialCharacter,
      world: initialWorld,
    }),
}));
