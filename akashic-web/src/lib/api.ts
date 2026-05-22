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

export interface ApiResponse<T> {
  success: boolean;
  data: T;
}

export interface CreateGameSessionData {
  sessionId: string;
  createdAt: string;
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

export type GameSessionControlInput =
  | { control: { type: 'continue' }; choice?: undefined }
  | { control?: undefined; choice: { choiceId: string } };

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

export function createGameSession(character: Character, world: World) {
  return requestJson<CreateGameSessionData>('/api/game-sessions/create', {
    method: 'POST',
    body: JSON.stringify({ character, world }),
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
