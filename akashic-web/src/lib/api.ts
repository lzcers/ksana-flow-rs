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

export interface WorldSnapshotNpcState {
  name: string;
  location: string;
  mood: string;
  attitude: string;
  goal: string;
  secrets: string[];
}

export interface WorldSnapshotItemState {
  name: string;
  location: string;
  status: string;
  awareness: string;
  relevance: string;
}

export interface WorldSnapshotOngoingEvent {
  name: string;
  status: string;
  escalation_trigger: string;
}

export interface WorldSnapshot {
  round: number;
  scene_title: string;
  time_absolute: string;
  time_relative?: string | null;
  location_name: string;
  location_exits: string[];
  location_status: string;
  description: string;
  current_event: string;
  new_info: string[];
  inner_conflict: string;
  hard_anchors: string[];
  pace: string;
  atmosphere: string;
  focal_point: string;
  protagonist_condition: string;
  protagonist_known_secrets: string[];
  npcs: WorldSnapshotNpcState[];
  items: WorldSnapshotItemState[];
  events_in_progress: WorldSnapshotOngoingEvent[];
  unsolved_threads: string[];
  pacing_note: string;
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

/** 单条剧情历史记录。 */
export interface HistoryItem {
  type: string;
  turnIndex: number;
  text: string;
  createdAt: string;
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

/** SSE 完成事件数据。 */
export interface StoryStreamDoneData {
  route: string;
  sessionId?: string | null;
}

/** 历史流开始时的元数据。 */
export interface StoryHistoryMetaData {
  sessionId: string;
  totalItems: number;
}

/** 会话流握手数据。 */
export interface StoryStreamHandshakeData {
  sessionId: string;
  protocol: string;
  note: string;
}

export interface StoryTurnView {
  phase:
  | 'idle'
  | 'fate_weaving'
  | 'narrator_writing'
  | 'narrator_story'
  | 'protagonist_action'
  | 'awaiting_protagonist'
  | 'awaiting_player_choice'
  | 'turn_finished'
  | 'failed';
  turnIndex: number;
  activeTurnId: number;
}

export interface StoryTaskView {
  entity: string;
  kind: 'fate_planning' | 'protagonist_action' | 'narration';
  status: 'pending' | 'running' | 'done' | 'error';
  attempts: number;
  maxAttempts: number;
  lastError: string | null;
  chunks: string[];
  output: string | null;
  error: string | null;
}

export interface StoryTaskUpdate {
  entity: string;
  kind: 'fate_planning' | 'protagonist_action' | 'narration';
  status: 'pending' | 'running' | 'done' | 'error';
  chunk?: string | null;
  output?: string | null;
  error?: string | null;
}

export interface StoryTaskUpdatedData {
  task: StoryTaskView;
  update: StoryTaskUpdate;
}

export interface StoryStreamWarningData {
  sessionId: string;
  reason: string;
  skipped: number;
}

export interface StoryStreamEventMap {
  'session.created': CreateGameSessionData;
  'session.snapshot': GameSessionSnapshot;
  'choice.submitted': SubmitChoiceData;
  'ending.ready': GameSessionEndingData;
  'intuition.preview': IntuitionPreviewData;
  'history.started': StoryHistoryMetaData;
  'history.item': HistoryItem;
  'stream.handshake': StoryStreamHandshakeData;
  'turn.changed': StoryTurnView;
  'world.updated': WorldSnapshot;
  'task.updated': StoryTaskUpdatedData;
  'stream.warning': StoryStreamWarningData;
  'create_game_session.done': StoryStreamDoneData;
  'get_game_session.done': StoryStreamDoneData;
  'submit_choice.done': StoryStreamDoneData;
  'get_game_session_ending.done': StoryStreamDoneData;
  'create_intuition_preview.done': StoryStreamDoneData;
  'get_game_session_history.done': StoryStreamDoneData;
  'stream_game_session.done': StoryStreamDoneData;
}

export type StoryStreamEventName = keyof StoryStreamEventMap;

export interface StoryStreamEvent<Name extends StoryStreamEventName = StoryStreamEventName> {
  event: Name;
  data: StoryStreamEventMap[Name];
}

const LIVE_STORY_STREAM_EVENTS: StoryStreamEventName[] = [
  'session.created',
  'session.snapshot',
  'choice.submitted',
  'ending.ready',
  'intuition.preview',
  'history.started',
  'history.item',
  'stream.handshake',
  'turn.changed',
  'world.updated',
  'task.updated',
  'stream.warning',
  'create_game_session.done',
  'get_game_session.done',
  'submit_choice.done',
  'get_game_session_ending.done',
  'create_intuition_preview.done',
  'get_game_session_history.done',
  'stream_game_session.done',
];

function parseApiErrorMessage(status: number, payload: unknown) {
  if (
    payload &&
    typeof payload === 'object' &&
    'error' in payload &&
    payload.error &&
    typeof payload.error === 'object' &&
    'message' in payload.error &&
    typeof payload.error.message === 'string'
  ) {
    return payload.error.message;
  }

  return `请求失败：${status}`;
}

function parseSseChunk(block: string): StoryStreamEvent | null {
  const lines = block
    .split('\n')
    .map((line) => line.trimEnd())
    .filter((line) => line.length > 0 && !line.startsWith(':'));

  if (!lines.length) {
    return null;
  }

  let eventName = 'message';
  const dataLines: string[] = [];

  for (const line of lines) {
    if (line.startsWith('event:')) {
      eventName = line.slice('event:'.length).trim();
      continue;
    }

    if (line.startsWith('data:')) {
      dataLines.push(line.slice('data:'.length).trimStart());
    }
  }

  if (!dataLines.length) {
    return null;
  }

  return {
    event: eventName as StoryStreamEventName,
    data: JSON.parse(dataLines.join('\n')) as StoryStreamEventMap[StoryStreamEventName],
  };
}

async function readStoryStream(response: Response) {
  if (!response.ok) {
    let payload: unknown = null;

    try {
      payload = (await response.json()) as ApiErrorBody;
    } catch {
      payload = null;
    }

    throw new Error(parseApiErrorMessage(response.status, payload));
  }

  if (!response.body) {
    throw new Error('服务端没有返回可读取的流。');
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  const events: StoryStreamEvent[] = [];
  let buffer = '';

  while (true) {
    const { value, done } = await reader.read();
    buffer += decoder.decode(value ?? new Uint8Array(), { stream: !done });
    buffer = buffer.replace(/\r\n/g, '\n');

    let boundaryIndex = buffer.indexOf('\n\n');
    while (boundaryIndex !== -1) {
      const chunk = buffer.slice(0, boundaryIndex);
      buffer = buffer.slice(boundaryIndex + 2);

      const parsed = parseSseChunk(chunk);
      if (parsed) {
        events.push(parsed);
      }

      boundaryIndex = buffer.indexOf('\n\n');
    }

    if (done) {
      break;
    }
  }

  const tail = parseSseChunk(buffer.trim());
  if (tail) {
    events.push(tail);
  }

  return events;
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

/** 故事接口请求封装，统一读取 SSE 中的 JSON 事件。 */
async function requestStoryStream(path: string, init?: RequestInit) {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    headers: {
      Accept: 'text/event-stream',
      'Content-Type': 'application/json',
      ...(init?.headers ?? {}),
    },
    ...init,
  });

  return readStoryStream(response);
}

/** 创建新的游戏会话，返回初始化后的会话数据。 */
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

/** 获取指定会话的当前快照流。 */
export function getGameSession(sessionId: string) {
  return requestStoryStream(`/api/game-sessions/${sessionId}`);
}

export function subscribeGameSessionStream(
  sessionId: string,
  options: {
    onEvent: (event: StoryStreamEvent) => void;
    onError?: (event: Event) => void;
  },
) {
  const source = new EventSource(`${API_BASE_URL}/api/game-sessions/${sessionId}/stream`);
  const listeners = LIVE_STORY_STREAM_EVENTS.map((eventName) => {
    const listener = (event: MessageEvent<string>) => {
      const data = JSON.parse(event.data) as StoryStreamEventMap[typeof eventName];
      options.onEvent({
        event: eventName,
        data,
      });
    };
    source.addEventListener(eventName, listener as EventListener);
    return { eventName, listener };
  });

  if (options.onError) {
    source.onerror = options.onError;
  }

  return () => {
    for (const { eventName, listener } of listeners) {
      source.removeEventListener(eventName, listener as EventListener);
    }
    source.close();
  };
}

/** 提交当前选项并推进回合，返回故事 SSE 事件。 */
export function submitChoice(
  sessionId: string,
  input: {
    choiceId: string;
    useObsession: boolean;
  },
) {
  return requestStoryStream(`/api/game-sessions/${sessionId}/choices`, {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

/** 生成指定选项的直觉预览流。 */
export function createIntuitionPreview(sessionId: string, choiceId: string) {
  return requestStoryStream(`/api/game-sessions/${sessionId}/intuition-preview`, {
    method: 'POST',
    body: JSON.stringify({ choiceId }),
  });
}

/** 查询指定会话的结局流。 */
export function getGameSessionEnding(sessionId: string) {
  return requestStoryStream(`/api/game-sessions/${sessionId}/ending`);
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
