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

/** 角色设定数据，用于创建会话和展示主角信息。 */
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

/** 世界观设定数据，用于创建会话和展示剧情背景。 */
export interface World {
  era: string;
  coreConflict: string;
  specialRules: string[];
}

/** 当前剧情节点中的可选项数据。 */
export interface Choice {
  id: string;
  text: string;
  disabled: boolean;
  costHints: {
    intuition: number;
    obsession: number;
  };
}

/** 当前剧情节点内容，包含文本、配图和选项。 */
export interface StoryNode {
  id: string;
  text: string;
  image: string;
  choices: Choice[];
}

/** 会话资源状态，用于展示理智、执念和剩余天数。 */
export interface SessionResources {
  obsessionPoints: number;
  intuitionPoints: number;
  daysLeft: number;
  worldNews: string | null;
}

/** 运行时状态视图，用于展示当前回合与叙事进度。 */
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

/** 结局详情数据，用于结局页和归档详情页展示。 */
export interface EndingData {
  biography: string;
  turningPoints: Array<{ cause: string; effect: string }>;
  legacy: string;
  cgs: string[];
}

/** 创建游戏会话后的返回数据。 */
export interface CreateGameSessionData {
  sessionId: string;
  createdAt: string;
  character: Character;
  world: World;
  resources: SessionResources;
  currentNode: StoryNode;
  stateView: RuntimeStateView;
}

/** 游戏会话快照，用于恢复当前游玩状态。 */
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

/** 提交选项后的回合推进结果。 */
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

/** 直觉预览返回数据，用于查看选项的额外提示。 */
export interface IntuitionPreviewData {
  choiceId: string;
  previewText: string;
  resourceDelta: {
    obsessionPoints: number;
    intuitionPoints: number;
  };
  resources: SessionResources;
}

/** 游戏会话结局查询结果。 */
export interface GameSessionEndingData {
  sessionId: string;
  endingStatus: string;
  ending: EndingData;
}

/** 创建存档后的返回摘要。 */
export interface SaveSummary {
  saveId: string;
  sessionId: string;
  title: string;
  summary: string;
  coverImage: string;
  turnIndex: number;
  savedAt: string;
}

/** 存档列表项数据，用于存档大厅展示。 */
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

/** 归档列表项数据，用于结局归档列表展示。 */
export interface ArchiveListItem {
  archiveId: string;
  title: string;
  tag: string;
  era: string;
  summary: string;
  coverImage: string;
  createdAt: string;
}

/** 单个归档详情数据，包含完整结局内容。 */
export interface ArchiveDetailData {
  archiveId: string;
  title: string;
  era: string;
  ending: EndingData;
}

/** 分享卡片生成结果，返回图片地址和过期时间。 */
export interface ShareCardData {
  shareCardId: string;
  imageUrl: string;
  expiresAt: string;
}

/** 通用请求封装，统一处理 JSON 响应与错误抛出。 */
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

/** 创建新的游戏会话，返回初始角色、世界和剧情节点。 */
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

/** 获取指定会话的当前快照，用于进入或恢复游玩页面。 */
export function getGameSession(sessionId: string) {
  return request<GameSessionSnapshot>(`/api/game-sessions/${sessionId}`);
}

/** 提交当前选项并推进回合，返回资源变动和最新状态。 */
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

/** 生成指定选项的直觉预览内容。 */
export function createIntuitionPreview(sessionId: string, choiceId: string) {
  return request<IntuitionPreviewData>(`/api/game-sessions/${sessionId}/intuition-preview`, {
    method: 'POST',
    body: JSON.stringify({ choiceId }),
  });
}

/** 查询指定会话的结局内容。 */
export function getGameSessionEnding(sessionId: string) {
  return request<GameSessionEndingData>(`/api/game-sessions/${sessionId}/ending`);
}

/** 创建存档，可选择同时生成分享卡。 */
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

/** 获取全部存档列表。 */
export function listSaves() {
  return request<{ items: SaveListItem[] }>('/api/saves');
}

/** 从指定存档加载会话，返回恢复后的当前状态。 */
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

/** 获取结局归档列表。 */
export function listArchives() {
  return request<{ items: ArchiveListItem[] }>('/api/archives');
}

/** 获取单个归档的详细结局内容。 */
export function getArchive(archiveId: string) {
  return request<ArchiveDetailData>(`/api/archives/${archiveId}`);
}

/** 为存档生成分享卡图片。 */
export function generateSaveShareCard(saveId: string, style = 'golden-night') {
  return request<ShareCardData>('/api/share/save-card', {
    method: 'POST',
    body: JSON.stringify({ saveId, style }),
  });
}

/** 为结局归档生成分享卡图片，可控制是否包含 CG。 */
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
