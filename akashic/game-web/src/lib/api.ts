export interface Character {
  name: string;
  gender: string;
  age: number;
  appearance: string;
  traits: {
    intellect: number;
    physique: number;
    endurance: number;
    courage: number;
    rationality: number;
    altruism: number;
  };
  background: string;
}

export interface World {
  era: string;
  description: string;
  specialRules: string[];
}

export interface StoryPreferences {
  theme: string;
  atmosphere: string;
  narrativeStyle: string;
  taboos: string;
}

export type TurnPhase =
  | 'idle'
  | 'simulation'
  | 'application'
  | 'awaiting_player'
  | 'turn_completed'
  | 'ended'
  | 'failed';

export type RuntimePhase = TurnPhase | 'booting' | 'opening';

export interface Choice {
  id: string;
  text: string;
  action: string;
  disabled: boolean;
  motivationAndRisk?: string;
}

export interface RuntimeStateView {
  gameState: string;
  phase: RuntimePhase;
  turnIndex: number;
  activeTurnId: number;
  currentLocation: string;
  currentScene: string;
  protagonistState: string;
  npcsState: string;
  latestHistory: string;
  latestBroadcastSummary: string;
  latestBroadcastItems?: string[];
  latestProtagonistAction: string;
  isEnding: boolean;
  endingType?: string | null;
}

export interface SessionWorldState {
  round: number;
  sceneTitle: string;
  timeAbsolute: string;
  timeRelative?: string | null;
  locationName: string;
  locationExits: string[];
  locationStatus: string;
  description: string;
  currentEvent: string;
  newInfo: string[];
  innerConflict: string;
  hardAnchors: string[];
  pace: string;
  atmosphere: string;
  focalPoint: string;
  isEnding: boolean;
  endingType?: string | null;
  protagonistCondition: string;
  protagonistKnownSecrets: string[];
}

export interface ApiResponse<T> {
  success: boolean;
  data: T;
}

export interface CreateGameSessionData {
  sessionId: string;
  createdAt: string;
}

export interface CreateSaveSlotInput {
  title?: string;
}

export interface TurnStateArchive {
  phase: TurnPhase;
  turn_index: number;
  active_turn_id: number;
}

export interface ProtagonistDecisionArchive {
  committed_action: string;
  choices: PendingProtagonistChoice[];
}

export interface SessionArchivePayload {
  session_id: string;
  title: string;
  world_profile: string;
  protagonist_profile: string;
  key_story_beats?: string;
  turn_state: TurnStateArchive;
  fate_weaver: unknown;
  upper_narrator: unknown;
  protagonist: unknown;
  world_snapshot: unknown;
  protagonist_decision: ProtagonistDecisionArchive;
  history_log: unknown;
}

export interface SaveExportData {
  sessionId: string;
  title: string;
  createdAt: string;
  updatedAt: string;
  compressedArchive: string;
}

export interface LoadArchiveInput {
  compressedArchive: string;
}

export interface GeneratedProfiles {
  world: string;
  protagonist: string;
  keyStoryBeats: string;
}

export interface StorySummaryData {
  summary: string;
  narrationCount: number;
}

export interface CreateGameSessionInput {
  worldProfile: string;
  protagonistProfile: string;
  keyStoryBeats?: string;
}

export interface ProtagonistOption {
  title: string;
  action: string;
  motivationAndRisk: string;
}

export interface PendingProtagonistChoice {
  id: string;
  option: ProtagonistOption;
}

export type PlayerActionType = 'selected_option' | 'free_text';

export interface PlayerActionInput {
  type: PlayerActionType;
  action: string;
}

export interface TaskView {
  entity: string;
  kind: string;
  status: 'pending' | 'running' | 'done' | 'error';
  attempts: number;
  maxAttempts: number;
  lastError: string | null;
  chunks: string[];
  output: string | null;
  error: string | null;
}

export interface GameSessionWorldStateData {
  sessionId: string;
  status: string;
  phase: TurnPhase;
  turnIndex: number;
  activeTurnId: number;
  worldState: SessionWorldState;
  history: SessionRoundHistoryData[];
  currentTask: TaskView | null;
  tasks: TaskView[];
  latestNarration: string;
  currentProtagonistAction: string;
  choices: PendingProtagonistChoice[];
}

export interface SessionRoundHistoryData {
  round: number;
  worldState: SessionWorldState | null;
  narrationText: string;
  choices: PendingProtagonistChoice[];
  committedAction?: string | null;
  selectedChoiceText?: string | null;
}

export type GameSessionControlInput =
  | { control: { type: 'continue' }; action?: undefined }
  | { control?: undefined; action: PlayerActionInput };

export interface TaskUpdatedEvent {
  eventId?: number;
  entity: string;
  kind: string;
  status: 'pending' | 'running' | 'done' | 'error';
  chunk?: string | null;
  output?: string | null;
  error?: string | null;
}

const API_ORIGIN = import.meta.env.PROD ? 'https://game.akasa.fun' : '';

function withApiOrigin(path: string) {
  return `${API_ORIGIN}${path}`;
}

async function requestJson<T>(input: string, init?: RequestInit): Promise<T> {
  const response = await fetch(input, {
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers ?? {}),
    },
    ...init,
  });

  if (!response.ok) {
    const message = await response.text();
    throw new Error(message || '网络似乎起了雾，请稍后再试。');
  }

  const payload = (await response.json()) as ApiResponse<T>;
  if (!payload.success) {
    throw new Error('这次操作没能完成，请稍后再试。');
  }

  return payload.data;
}

function formatSpecialRules(specialRules: string[]): string {
  if (specialRules.length === 0) {
    return '无';
  }

  return specialRules.map((rule, index) => `${index + 1}. ${rule}`).join('\n');
}

export function buildGenerateProfilesPrompt(
  character: Character,
  world: World,
): string {
  return `请基于以下两组设定生成“世界设定”和“主角设定”。

这些表单内容都是已确定事实，禁止改写、替换或否定，只能围绕它们做扩写、补完和强化。

[人物设定]
- 姓名：${character.name}
- 性别：${character.gender}
- 年龄：${character.age}
- 人物设定：${character.background || '未填写'}
- 人物描述：${character.appearance || '未填写'}
- 属性分配：
  - 智力：${character.traits.intellect}
  - 体力：${character.traits.physique}
  - 耐力：${character.traits.endurance}
  - 勇气：${character.traits.courage}
  - 理性：${character.traits.rationality}
  - 利他：${character.traits.altruism}

[世界设定]
- 时代：${world.era}
- 世界描述：${world.description || '未填写'}
- 额外特殊规则：
${formatSpecialRules(world.specialRules)}

[生成目标]
- 这是长期互动叙事的设定底稿，不是一次性简介。
- 世界设定必须严格建立在“世界设定”事实上。
- 主角设定必须严格建立在“人物设定”事实上，并自然解释主角为何会被卷入这个故事。
- 世界设定重点写清世界如何运转、现实压力从何而来，以及什么样的秩序正在支配众人。
- 主角设定重点写清欲望、弱点、行动倾向，以及六项属性如何转化为行为习惯与判断方式。
`;
}

export function generateProfiles(character: Character, world: World) {
  const prompt = buildGenerateProfilesPrompt(character, world);
  return requestJson<GeneratedProfiles>(withApiOrigin('/api/profiles/generate'), {
    method: 'POST',
    body: JSON.stringify({ prompt }),
  });
}

export function createGameSession(input: CreateGameSessionInput) {
  return requestJson<CreateGameSessionData>(withApiOrigin('/api/game-sessions/create'), {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

export function getGameSession(sessionId: string) {
  return requestJson<GameSessionWorldStateData>(
    withApiOrigin(`/api/game-sessions/${encodeURIComponent(sessionId)}`),
  );
}

export function cloneGameSession(sessionId: string) {
  return requestJson<GameSessionWorldStateData>(
    withApiOrigin(`/api/game-sessions/${encodeURIComponent(sessionId)}/clone`),
    {
      method: 'POST',
    },
  );
}

export function exportGameSaveArchive(sessionId: string, input: CreateSaveSlotInput) {
  return requestJson<SaveExportData>(withApiOrigin(`/api/game-sessions/${sessionId}/save-export`), {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

export function generateGameSessionStorySummary(sessionId: string) {
  return requestJson<StorySummaryData>(withApiOrigin(`/api/game-sessions/${sessionId}/summary`), {
    method: 'POST',
  });
}

export function loadGameSessionFromArchive(input: LoadArchiveInput) {
  return requestJson<GameSessionWorldStateData>(withApiOrigin('/api/game-sessions/load-archive'), {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

export function submitGameSessionControl(
  sessionId: string,
  input: GameSessionControlInput,
) {
  return requestJson<{ action: string }>(withApiOrigin(`/api/game-sessions/${sessionId}/control`), {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

export function openGameSessionStream(
  sessionId: string,
  handlers: {
    onTaskUpdated: (event: TaskUpdatedEvent, lastEventId: string) => void;
    onError?: () => void;
  },
  since?: string | null,
) {
  const search = since ? `?since=${encodeURIComponent(since)}` : '';
  const eventSource = new EventSource(
    withApiOrigin(`/api/game-sessions/${sessionId}/stream${search}`),
  );

  eventSource.addEventListener('task.updated', (rawEvent) => {
    const event = rawEvent as MessageEvent<string>;
    handlers.onTaskUpdated(JSON.parse(event.data) as TaskUpdatedEvent, event.lastEventId);
  });

  if (handlers.onError) {
    eventSource.addEventListener('error', () => {
      handlers.onError?.();
    });
  }

  return eventSource;
}
