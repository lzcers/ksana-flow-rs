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
  action: string;
  disabled: boolean;
  previewText?: string;
  costHints: {
    intuition: number;
    obsession: number;
  };
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
  latestBroadcastItems?: string[];
  latestProtagonistAction: string;
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

export interface CreateSaveSlotData {
  slotId: string;
  sessionId: string;
  title: string;
  createdAt: string;
  updatedAt: string;
}

export interface LoadGameSessionInput {
  slotId: string;
}

export interface GeneratedProfiles {
  world: string;
  protagonist: string;
}

export interface CreateGameSessionInput {
  worldProfile: string;
  protagonistProfile: string;
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
  phase: string;
  turnIndex: number;
  activeTurnId: number;
  worldState: SessionWorldState;
  currentTask: TaskView | null;
  tasks: TaskView[];
  latestNarration: string;
  currentProtagonistAction: string;
  choices: PendingProtagonistChoice[];
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
    throw new Error(message || `请求失败：${response.status}`);
  }

  const payload = (await response.json()) as ApiResponse<T>;
  if (!payload.success) {
    throw new Error('请求未成功完成。');
  }

  return payload.data;
}

function formatSpecialRules(specialRules: string[]): string {
  if (specialRules.length === 0) {
    return '无';
  }

  return specialRules.map((rule, index) => `${index + 1}. ${rule}`).join('\n');
}

export function buildGenerateProfilesPrompt(character: Character, world: World): string {
  return `请基于以下角色创建表单生成“世界设定”和“主角设定”。

这些表单内容都是已确定事实，禁止改写、替换或否定，只能围绕它们做扩写、补完和强化。

[角色表单]
- 姓名：${character.name}
- 性别：${character.gender}
- 年龄：${character.age}
- 外貌 / 标记：${character.appearance || '未填写'}
- 人生烙印：${character.background}
- 特质数值：
  - 勇气：${character.traits.courage}
  - 理性：${character.traits.rationality}
  - 利他：${character.traits.altruism}

[世界表单]
- 时代：${world.era}
- 核心矛盾：${world.coreConflict}
- 额外特殊规则：
${formatSpecialRules(world.specialRules)}

[生成目标]
- 这是长期互动叙事的设定底稿，不是一次性简介。
- 请让“核心矛盾”同时支配世界设定与主角设定。
- 世界设定重点写清规则、禁忌、秩序、势力和现实压力。
- 主角设定重点写清欲望、弱点、行动倾向，以及为何会被卷入主冲突。
- 请把三项特质转化为行为倾向、判断方式、优势与弱点，不要机械复述数字。
- 文风偏文学叙事，但内容必须具体、可演绎，能自然推动后续冲突和抉择。`;
}

export function generateProfiles(character: Character, world: World) {
  const prompt = buildGenerateProfilesPrompt(character, world);
  return requestJson<GeneratedProfiles>('/api/profiles/generate', {
    method: 'POST',
    body: JSON.stringify({ prompt }),
  });
}

export function createGameSession(input: CreateGameSessionInput) {
  return requestJson<CreateGameSessionData>('/api/game-sessions/create', {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

export function loadGameSession(input: LoadGameSessionInput) {
  return requestJson<GameSessionWorldStateData>('/api/game-sessions/load', {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

export function createGameSaveSlot(sessionId: string, input: CreateSaveSlotInput) {
  return requestJson<CreateSaveSlotData>(`/api/game-sessions/${sessionId}/save`, {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

export function submitGameSessionControl(
  sessionId: string,
  input: GameSessionControlInput,
) {
  return requestJson<{ action: string }>(`/api/game-sessions/${sessionId}/control`, {
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
  const eventSource = new EventSource(`/api/game-sessions/${sessionId}/stream${search}`);

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
