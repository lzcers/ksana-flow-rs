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
  previewText?: string;
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

export interface ApiResponse<T> {
  success: boolean;
  data: T;
}

export interface CreateGameSessionData {
  sessionId: string;
  createdAt: string;
  status: string;
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

export interface WorldStateView {
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

interface RawWorldStateView {
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
}

export interface GameSessionWorldStateData {
  sessionId: string;
  status: string;
  phase: string;
  turnIndex: number;
  activeTurnId: number;
  worldState: WorldStateView;
  currentTask: TaskView | null;
  tasks: TaskView[];
  latestNarration: string;
  currentProtagonistAction: string;
  choices: PendingProtagonistChoice[];
}

interface RawGameSessionWorldStateData {
  sessionId: string;
  status: string;
  phase: string;
  turnIndex: number;
  activeTurnId: number;
  worldState: RawWorldStateView;
  currentTask: TaskView | null;
  tasks: TaskView[];
  latestNarration: string;
  currentProtagonistAction: string;
  choices: PendingProtagonistChoice[];
}

export interface ControlGameSessionData {
  action: string;
  session: GameSessionWorldStateData;
}

interface RawControlGameSessionData {
  action: string;
  session: RawGameSessionWorldStateData;
}

export interface TaskSnapshotEvent {
  task: TaskView;
}

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

function normalizeWorldState(worldState: RawWorldStateView): WorldStateView {
  return {
    round: worldState.round,
    sceneTitle: worldState.scene_title,
    timeAbsolute: worldState.time_absolute,
    timeRelative: worldState.time_relative,
    locationName: worldState.location_name,
    locationExits: worldState.location_exits,
    locationStatus: worldState.location_status,
    description: worldState.description,
    currentEvent: worldState.current_event,
    newInfo: worldState.new_info,
    innerConflict: worldState.inner_conflict,
    hardAnchors: worldState.hard_anchors,
    pace: worldState.pace,
    atmosphere: worldState.atmosphere,
    focalPoint: worldState.focal_point,
    protagonistCondition: worldState.protagonist_condition,
    protagonistKnownSecrets: worldState.protagonist_known_secrets,
  };
}

function normalizeSession(session: RawGameSessionWorldStateData): GameSessionWorldStateData {
  return {
    ...session,
    worldState: normalizeWorldState(session.worldState),
  };
}

export function createGameSession(character: Character, world: World) {
  return requestJson<CreateGameSessionData>('/api/game-sessions', {
    method: 'POST',
    body: JSON.stringify({ character, world }),
  });
}

export function controlGameSession(
  sessionId: string,
  body:
    | { control: { type: 'continue' }; choice?: undefined }
    | { control?: undefined; choice: { choiceId: string } },
) {
  return requestJson<RawControlGameSessionData>(`/api/game-sessions/${sessionId}/control`, {
    method: 'POST',
    body: JSON.stringify(body),
  }).then((data) => ({
    ...data,
    session: normalizeSession(data.session),
  }));
}

export function getGameSessionWorld(sessionId: string) {
  return requestJson<RawGameSessionWorldStateData>(`/api/game-sessions/${sessionId}`).then(
    normalizeSession,
  );
}

export function openGameSessionStream(
  sessionId: string,
  handlers: {
    onTaskSnapshot: (event: TaskSnapshotEvent) => void;
    onTaskUpdated: (event: TaskUpdatedEvent, lastEventId: string) => void;
    onError?: () => void;
  },
  since?: string | null,
) {
  const search = since ? `?since=${encodeURIComponent(since)}` : '';
  const eventSource = new EventSource(`/api/game-sessions/${sessionId}/stream${search}`);

  eventSource.addEventListener('task.snapshot', (rawEvent) => {
    const event = rawEvent as MessageEvent<string>;
    handlers.onTaskSnapshot(JSON.parse(event.data) as TaskSnapshotEvent);
  });

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
