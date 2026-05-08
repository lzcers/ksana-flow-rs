const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '';

interface ApiResponse<T> {
  success: boolean;
  data: T;
}

interface ApiErrorBody {
  success: boolean;
  error?: {
    code: string;
    message: string;
  };
}

export interface Character {
  name: string;
  gender: string;
  age: number;
  appearance: string;
  traits: {
    courage: number;
    rationality: number;
    altruism: number;
  };
  background: string;
}

export interface World {
  era: string;
  coreConflict: string;
  specialRules: string[];
}

export interface Choice {
  id: string;
  text: string;
  disabled: boolean;
  costHints: {
    intuition: number;
    obsession: number;
  };
}

export interface StoryNode {
  id: string;
  text: string;
  image: string;
  choices: Choice[];
}

export interface SessionResources {
  obsessionPoints: number;
  intuitionPoints: number;
  daysLeft: number;
  worldNews: string | null;
}

export interface RuntimeStateView {
  gameState: string;
  phase: string;
  turnIndex: number;
  activeTurnId: number;
  currentLocation: string;
  currentScene: string;
  protagonistState: string;
  npcsState: string;
  latestHistory: string;
  latestBroadcastSummary: string;
  latestProtagonistAction: string;
}

export interface EndingData {
  biography: string;
  turningPoints: Array<{ cause: string; effect: string }>;
  legacy: string;
  cgs: string[];
}

export interface CreateGameSessionData {
  sessionId: string;
  createdAt: string;
  character: Character;
  world: World;
  resources: SessionResources;
  currentNode: StoryNode;
  stateView: RuntimeStateView;
}

export interface GameSessionSnapshot {
  sessionId: string;
  status: string;
  character: Character;
  world: World;
  resources: SessionResources;
  currentNode: StoryNode;
  stateView: RuntimeStateView;
  endingStatus: string;
}

export interface SubmitChoiceData {
  accepted: boolean;
  sessionId: string;
  turnId: number;
  resourceDelta: {
    obsessionPoints: number;
    intuitionPoints: number;
  };
  resources: SessionResources;
  stateView: RuntimeStateView;
}

export interface IntuitionPreviewData {
  choiceId: string;
  previewText: string;
  resourceDelta: {
    obsessionPoints: number;
    intuitionPoints: number;
  };
  resources: SessionResources;
}

export interface GameSessionEndingData {
  sessionId: string;
  endingStatus: string;
  ending: EndingData;
}

export interface SaveSummary {
  saveId: string;
  sessionId: string;
  title: string;
  summary: string;
  coverImage: string;
  turnIndex: number;
  savedAt: string;
}

export interface SaveListItem {
  saveId: string;
  sessionId: string;
  title: string;
  characterName: string;
  background: string;
  era: string;
  turnIndex: number;
  summary: string;
  coverImage: string;
  savedAt: string;
}

export interface ArchiveListItem {
  archiveId: string;
  title: string;
  tag: string;
  era: string;
  summary: string;
  coverImage: string;
  createdAt: string;
}

export interface ArchiveDetailData {
  archiveId: string;
  title: string;
  era: string;
  ending: EndingData;
}

export interface ShareCardData {
  shareCardId: string;
  imageUrl: string;
  expiresAt: string;
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers ?? {}),
    },
    ...init,
  });

  const payload = (await response.json()) as ApiResponse<T> | ApiErrorBody;
  if (!response.ok || !('success' in payload) || payload.success === false) {
    const message =
      'error' in payload && payload.error?.message
        ? payload.error.message
        : `请求失败：${response.status}`;
    throw new Error(message);
  }

  return (payload as ApiResponse<T>).data;
}

export function createGameSession(input: {
  character: Character;
  world: World;
  seed?: string;
}) {
  return request<CreateGameSessionData>('/api/game-sessions', {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

export function getGameSession(sessionId: string) {
  return request<GameSessionSnapshot>(`/api/game-sessions/${sessionId}`);
}

export function submitChoice(
  sessionId: string,
  input: {
    choiceId: string;
    useObsession: boolean;
  },
) {
  return request<SubmitChoiceData>(`/api/game-sessions/${sessionId}/choices`, {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

export function createIntuitionPreview(sessionId: string, choiceId: string) {
  return request<IntuitionPreviewData>(`/api/game-sessions/${sessionId}/intuition-preview`, {
    method: 'POST',
    body: JSON.stringify({ choiceId }),
  });
}

export function getGameSessionEnding(sessionId: string) {
  return request<GameSessionEndingData>(`/api/game-sessions/${sessionId}/ending`);
}

export function createSave(input: {
  sessionId: string;
  title: string;
  autoGenerateShareCard: boolean;
}) {
  return request<SaveSummary>('/api/saves', {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

export function listSaves() {
  return request<{ items: SaveListItem[] }>('/api/saves');
}

export function loadSave(saveId: string) {
  return request<{
    sessionId: string;
    loadedFromSaveId: string;
    status: string;
    resources: SessionResources;
    currentNode: StoryNode;
    stateView: RuntimeStateView;
  }>(`/api/saves/${saveId}/load`, {
    method: 'POST',
  });
}

export function listArchives() {
  return request<{ items: ArchiveListItem[] }>('/api/archives');
}

export function getArchive(archiveId: string) {
  return request<ArchiveDetailData>(`/api/archives/${archiveId}`);
}

export function generateSaveShareCard(saveId: string, style = 'golden-night') {
  return request<ShareCardData>('/api/share/save-card', {
    method: 'POST',
    body: JSON.stringify({ saveId, style }),
  });
}

export function generateEndingShareCard(
  archiveId: string,
  input: { includeCgs?: boolean; style?: string } = {},
) {
  return request<ShareCardData>('/api/share/ending-card', {
    method: 'POST',
    body: JSON.stringify({
      archiveId,
      includeCgs: input.includeCgs ?? true,
      style: input.style ?? 'golden-night',
    }),
  });
}
