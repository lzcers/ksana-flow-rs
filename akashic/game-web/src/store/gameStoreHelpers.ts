import type {
  Character,
  Choice,
  PendingProtagonistChoice,
  StoryPreferences,
  TaskUpdatedEvent,
  TaskView,
  World,
} from '../lib/api';

interface StreamedProtagonistOption {
  title?: string;
  action?: string;
  motivation_and_risk?: string;
  motivationAndRisk?: string;
}

export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

type JsonObject = { [key: string]: JsonValue };

interface FatePlanningSummary {
  round: number | null;
  sceneTitle: string | null;
  locationName: string | null;
  locationStatus: string | null;
  description: string | null;
  currentEvent: string | null;
  newInfo: string[];
  isEnding: boolean | null;
  endingType: string | null;
  protagonistCondition: string | null;
}

export const STREAM_PLACEHOLDER_TEXT = '命运正在编织中...';

export const initialCharacter: Character = {
  name: '',
  gender: '',
  age: 18,
  appearance: '',
  traits: {
    intellect: 5,
    physique: 5,
    endurance: 5,
    courage: 5,
    rationality: 5,
    altruism: 5,
  },
  background: '',
};

export const initialWorld: World = {
  era: '蒸汽朋克',
  description: '',
  specialRules: [],
};

export const initialStory: StoryPreferences = {
  theme: '',
  atmosphere: '',
  narrativeStyle: '',
  taboos: '',
};

export function cloneCharacter(character: Character): Character {
  return {
    ...character,
    traits: { ...character.traits },
  };
}

export function cloneWorld(world: World): World {
  return {
    ...world,
    specialRules: [...world.specialRules],
  };
}

export function cloneStory(story: StoryPreferences): StoryPreferences {
  return {
    ...story,
  };
}

export function toChoiceFromSession(choice: PendingProtagonistChoice): Choice {
  return {
    id: choice.id,
    text: choice.option.title || choice.option.action,
    action: choice.option.action,
    motivationAndRisk: choice.option.motivationAndRisk,
    disabled: false,
  };
}

export function taskLabel(kind: string): string {
  switch (kind) {
    case 'fate_planning':
      return '命运编织中...';
    case 'narration':
      return '叙事展开中...';
    case 'protagonist_action':
      return '主角抉择';
    default:
      return '命运推进';
  }
}

export function taskContent(task: TaskView): string | null {
  if (task.status === 'done' && task.output != null) {
    return task.output;
  }

  if (task.chunks.length > 0) {
    return task.chunks.join('');
  }

  return task.output;
}

export function taskText(task: TaskView): string | null {
  const text = taskContent(task);
  if (!text?.trim()) {
    return null;
  }
  if (task.kind === 'narration') {
    return text;
  }
  return null;
}

export function taskRawContent(task: TaskView | null | undefined): string {
  return task ? taskContent(task) ?? '' : '';
}

export function parseJsonValue(raw: string): JsonValue | null {
  if (!raw.trim()) {
    return null;
  }

  try {
    return JSON.parse(raw) as JsonValue;
  } catch {
    return null;
  }
}

function isJsonObject(value: JsonValue | null): value is JsonObject {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function readJsonString(value: JsonValue | undefined): string | null {
  return typeof value === 'string' ? value : null;
}

function readJsonNumber(value: JsonValue | undefined): number | null {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value;
  }

  if (typeof value === 'string') {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : null;
  }

  return null;
}

function readJsonBoolean(value: JsonValue | undefined): boolean | null {
  return typeof value === 'boolean' ? value : null;
}

function readJsonStringArray(value: JsonValue | undefined): string[] {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string') : [];
}

export function summarizeFatePlanning(value: JsonValue | null): FatePlanningSummary | null {
  if (!isJsonObject(value)) {
    return null;
  }

  return {
    round: readJsonNumber(value.round),
    sceneTitle: readJsonString(value.scene_title),
    locationName: readJsonString(value.location_name),
    locationStatus: readJsonString(value.location_status),
    description: readJsonString(value.description),
    currentEvent: readJsonString(value.current_event),
    newInfo: readJsonStringArray(value.new_info),
    isEnding: readJsonBoolean(value.is_ending),
    endingType: readJsonString(value.ending_type),
    protagonistCondition: readJsonString(value.protagonist_condition),
  };
}

function toChoiceFromStreamOption(option: StreamedProtagonistOption, index: number): Choice {
  const action = option.action?.trim() || '';
  return {
    id: `choice-${index + 1}`,
    text: option.title?.trim() || action || `行动 ${index + 1}`,
    action,
    motivationAndRisk: option.motivationAndRisk?.trim() || option.motivation_and_risk?.trim(),
    disabled: false,
  };
}

function parseProtagonistChoicesPayload(raw: string): Choice[] | null {
  try {
    const parsed = JSON.parse(raw) as { options?: StreamedProtagonistOption[] };
    return (parsed.options ?? []).map(toChoiceFromStreamOption);
  } catch {
    return null;
  }
}

function extractCompletedJsonObjects(raw: string): string[] {
  const objects: string[] = [];
  let depth = 0;
  let startIndex = -1;
  let isInString = false;
  let isEscaping = false;

  for (let index = 0; index < raw.length; index += 1) {
    const char = raw[index];

    if (isEscaping) {
      isEscaping = false;
      continue;
    }

    if (char === '\\' && isInString) {
      isEscaping = true;
      continue;
    }

    if (char === '"') {
      isInString = !isInString;
      continue;
    }

    if (isInString) {
      continue;
    }

    if (char === '{') {
      if (depth === 0) {
        startIndex = index;
      }
      depth += 1;
      continue;
    }

    if (char === '}') {
      depth -= 1;
      if (depth === 0 && startIndex >= 0) {
        objects.push(raw.slice(startIndex, index + 1));
        startIndex = -1;
      }
    }
  }

  return objects;
}

function parseStreamingProtagonistChoices(raw: string): Choice[] | null {
  const parsed = parseProtagonistChoicesPayload(raw);
  if (parsed) {
    return parsed;
  }

  const optionsMatch = raw.match(/"options"\s*:\s*\[/);
  if (!optionsMatch) {
    return null;
  }

  const optionSection = raw.slice((optionsMatch.index ?? 0) + optionsMatch[0].length);
  const optionObjects = extractCompletedJsonObjects(optionSection);
  if (optionObjects.length === 0) {
    return null;
  }

  const options = optionObjects.flatMap((item) => {
    try {
      return [JSON.parse(item) as StreamedProtagonistOption];
    } catch {
      return [];
    }
  });

  return options.map(toChoiceFromStreamOption);
}

export function protagonistActionChoices(task: TaskView): Choice[] | null {
  if (task.kind !== 'protagonist_action') {
    return null;
  }

  const raw = taskContent(task);
  if (!raw?.trim()) {
    return null;
  }

  return parseStreamingProtagonistChoices(raw);
}

export function protagonistActionText(task: TaskView): string | null {
  const raw = taskContent(task);
  if (task.kind !== 'protagonist_action' || !raw?.trim()) {
    return null;
  }

  try {
    const parsedChoices = parseStreamingProtagonistChoices(raw);
    if (parsedChoices) {
      if (parsedChoices.length === 0) {
        return '前路暂时未显，请稍等剧情展开。';
      }
      return parsedChoices.map((choice) => choice.text).join(' / ');
    }

    const parsed = JSON.parse(raw) as { options?: StreamedProtagonistOption[] };
    const options = parsed.options ?? [];
    if (options.length === 0) {
      return '前路暂时未显，请稍等剧情展开。';
    }
    return options
      .map((option, index) => option.title?.trim() || option.action?.trim() || `行动 ${index + 1}`)
      .join(' / ');
  } catch {
    return raw.trim();
  }
}

export function cloneTask(task: TaskView): TaskView {
  return {
    ...task,
    chunks: [...task.chunks],
  };
}

export function applyTaskUpdate(tasks: Map<string, TaskView>, update: TaskUpdatedEvent): TaskView {
  const existingTask = tasks.get(update.entity);
  const shouldResetTask =
    update.status === 'pending'
    || !existingTask
    || existingTask.kind !== update.kind;
  const currentTask = shouldResetTask ? {
    entity: update.entity,
    kind: update.kind,
    status: 'pending',
    attempts: 1,
    maxAttempts: 1,
    lastError: null,
    chunks: [],
    output: null,
    error: null,
  } : existingTask;

  const nextTask: TaskView = {
    ...currentTask,
    kind: update.kind,
    status: update.status,
    chunks: [...currentTask.chunks],
  };

  if (update.chunk != null) {
    nextTask.chunks.push(update.chunk);
  }

  if (update.output !== undefined) {
    nextTask.output = update.output;
  }

  if (update.error !== undefined) {
    nextTask.error = update.error;
    nextTask.lastError = update.error;
  }

  if (update.status === 'done') {
    nextTask.error = null;
    nextTask.lastError = null;
  }

  tasks.set(update.entity, nextTask);
  return nextTask;
}
