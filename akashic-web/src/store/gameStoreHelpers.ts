import type {
  ArchiveListItem,
  Character,
  Choice,
  GameSessionWorldStateData,
  PendingProtagonistChoice,
  RuntimeStateView,
  SaveListItem,
  StoryNode,
  TaskUpdatedEvent,
  TaskView,
  World,
} from '../lib/api';

export type DemoNodeId = 'opening' | 'tower' | 'tunnel';
type DemoEndingId = 'bellkeeper' | 'ghostscribe' | 'torchbearer' | 'echobroker';

interface DemoChoiceMeta {
  id: string;
  text: string;
  preview: string;
  nextNodeId?: DemoNodeId;
  endingId?: DemoEndingId;
}

interface DemoNodeMeta {
  id: DemoNodeId;
  location: string;
  scene: string;
  protagonistState: string;
  summary: string;
  news: string;
  image: string;
  buildText: (character: Character, world: World) => string;
  choices: DemoChoiceMeta[];
}

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
  protagonistCondition: string | null;
}

export interface ControlledSessionStateSlice {
  currentNode: StoryNode;
  stateView: RuntimeStateView;
  turnIndex: number;
  gameState: 'playing';
  streamedNarrationText: string;
  streamedNarrationStatus: TaskView['status'] | null;
  streamedFatePlanningRaw: string;
  streamedFatePlanningJson: JsonValue | null;
  streamedProtagonistActionRaw: string;
  streamedProtagonistActionJson: JsonValue | null;
  isLoading: boolean;
}

const image = (prompt: string) =>
  `https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=${encodeURIComponent(prompt)}&image_size=landscape_16_9`;

export const STORY_IMAGES = {
  opening: image('mystical dark fantasy city street emerging from mist, cinematic story game background'),
  tower: image('clock tower interior with glowing gears and rain outside, dark fantasy cinematic concept art'),
  tunnel: image('hidden underground tunnel with lantern reflections and ancient glyphs, dark fantasy cinematic concept art'),
  endingHope: image('hero standing before dawn over a dark city, cinematic dark fantasy illustration'),
  endingShadow: image('solitary figure beside shattered mirror and moonlit corridor, cinematic dark fantasy illustration'),
  corridor: image('vast mystical hall of memory, dark blue archive walls, golden lines, cinematic concept art'),
};

export const STREAM_PLACEHOLDER_TEXT = '命运正在展开，请稍候...';

export const initialCharacter: Character = {
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

export const initialWorld: World = {
  era: '蒸汽朋克',
  coreConflict: '资源枯竭与永生诱惑',
  specialRules: [],
};

export const DEMO_ENDINGS: Record<
  DemoEndingId,
  {
    title: string;
    tag: string;
    summary: string;
    legacy: string;
    biography: (character: Character, world: World) => string;
    turningPoints: Array<{ cause: string; effect: string }>;
    cgs: string[];
  }
> = {
  bellkeeper: {
    title: '钟楼守火人',
    tag: '微光余烬',
    summary: '你用自己的名字换来钟楼重鸣，城中第一束晨光因此抵达。',
    legacy: '有些人不是为了活在故事里，而是为了替后来者点亮入口。',
    biography: (character, world) =>
      `${character.name || '无名旅人'}在${world.era}的长夜里登上钟楼，将关于${world.coreConflict}的真相刻进铜钟。` +
      ' 当齿轮重新咬合，整座城市终于意识到，命运并非必须沿着旧轨道坠落。',
    turningPoints: [
      { cause: '你选择登上钟楼，而不是躲回人群。', effect: '失控的城市第一次听见了属于自己的回声。' },
      { cause: '你敲响最后一次钟声。', effect: '晨雾被撕开，旧秩序开始松动。' },
    ],
    cgs: [STORY_IMAGES.tower, STORY_IMAGES.endingHope, STORY_IMAGES.corridor],
  },
  ghostscribe: {
    title: '镜廊代笔人',
    tag: '静默真相',
    summary: '你保存了真相，却把自己写成了无人知晓的注脚。',
    legacy: '隐匿不是失败，它只是另一种更漫长的抵达。',
    biography: (character, world) =>
      `${character.name || '无名旅人'}没有敲响钟楼，而是带着关于${world.coreConflict}的手稿遁入镜廊。` +
      ' 那些未被公开的名字与罪证，被你一页页藏进会在未来醒来的档案之中。',
    turningPoints: [
      { cause: '你收起了钟锤。', effect: '城市继续沉睡，但真相被完整保留下来。' },
      { cause: '你选择成为记录者。', effect: '后来人终于拥有了改写历史的钥匙。' },
    ],
    cgs: [STORY_IMAGES.opening, STORY_IMAGES.endingShadow, STORY_IMAGES.corridor],
  },
  torchbearer: {
    title: '隧底提灯者',
    tag: '温热回响',
    summary: '你把灯留给了他人，自己却成为所有人心里最亮的一段传闻。',
    legacy: '真正的出口，往往从愿意带上别人的那一步开始。',
    biography: (character, world) =>
      `${character.name || '无名旅人'}在地下甬道中救下迷路的孩子，并把最后一盏灯留给了同伴。` +
      ` 关于${world.coreConflict}的答案没有立刻降临，但城市开始学会互相照亮。`,
    turningPoints: [
      { cause: '你停下脚步，先去救人。', effect: '冷硬的地下秩序第一次为温情让路。' },
      { cause: '你把最后一盏灯交了出去。', effect: '无数人因此找到了回家的方向。' },
    ],
    cgs: [STORY_IMAGES.tunnel, STORY_IMAGES.endingHope, STORY_IMAGES.corridor],
  },
  echobroker: {
    title: '回声交易人',
    tag: '暗潮未歇',
    summary: '你换来了答案，也让自己永远留在答案的阴影里。',
    legacy: '当人试图驾驭深渊时，深渊总会悄悄留下回礼。',
    biography: (character, world) =>
      `${character.name || '无名旅人'}与甬道尽头的低语达成交换，提前得知了${world.coreConflict}背后的操盘者。` +
      ' 代价是你的声音被留在了黑暗里，从此只能通过别人的梦境重返现实。',
    turningPoints: [
      { cause: '你接受了低语的交易。', effect: '真相瞬间浮现，但你也失去了归途的完整轮廓。' },
      { cause: '你带着答案离开隧道。', effect: '整座城市开始在梦里重复你的警告。' },
    ],
    cgs: [STORY_IMAGES.tunnel, STORY_IMAGES.endingShadow, STORY_IMAGES.corridor],
  },
};

export const DEMO_NODES: Record<DemoNodeId, DemoNodeMeta> = {
  opening: {
    id: 'opening',
    location: '灰雾城区 · 旧港尽头',
    scene: '开场序章',
    protagonistState: '初入命运现场',
    summary: '命运在钟声与潮声之间同时向你招手。',
    news: '港区传闻：今夜有人会在雾中改写第一条规则。',
    image: STORY_IMAGES.opening,
    buildText: (character, world) =>
      `${character.name || '无名旅人'}踏进${world.era}的灰雾城区时，雨正顺着铜制招牌一滴滴滑落。` +
      ` 关于“${world.coreConflict}”的低语从港口、钟楼与人群肩头一同传来。` +
      ' 你意识到，今晚必须先选定自己靠近真相的方式。',
    choices: [
      {
        id: 'to-tower',
        text: '登上钟楼，试着找到第一声异常钟鸣的来源',
        preview: '你会看见一枚被改造过的钟摆，以及被刻意抹去的守则。',
        nextNodeId: 'tower',
      },
      {
        id: 'to-tunnel',
        text: '跟随提灯小贩，从排水甬道潜入地下回路',
        preview: '潮湿的石壁后面藏着孩子的脚印，也藏着会说话的回声。',
        nextNodeId: 'tunnel',
      },
    ],
  },
  tower: {
    id: 'tower',
    location: '云顶钟楼 · 齿轮夹层',
    scene: '钟楼对峙',
    protagonistState: '直面秩序真相',
    summary: '所有指针都停住了，只有你的决定仍在前进。',
    news: '钟楼密报：守钟人已失踪，只留下半页烧焦的值夜记录。',
    image: STORY_IMAGES.tower,
    buildText: (character, world) =>
      `你在钟楼顶层看见了失控的齿轮列阵，也看见了那本记录着${world.coreConflict}源头的夜班手稿。` +
      ` ${character.name || '无名旅人'}知道，接下来的动作会决定这座城是立刻苏醒，还是把真相封存到未来。`,
    choices: [
      {
        id: 'ending-bellkeeper',
        text: '敲响铜钟，让整座城市同时听见真相',
        preview: '钟声会震碎旧秩序，但你也必须承担第一个站出来的人所要付出的代价。',
        endingId: 'bellkeeper',
      },
      {
        id: 'ending-ghostscribe',
        text: '带走手稿，先把所有名字与罪证保存下来',
        preview: '你会失去当场改变一切的机会，却能留下更完整的答案。',
        endingId: 'ghostscribe',
      },
    ],
  },
  tunnel: {
    id: 'tunnel',
    location: '潮汐甬道 · 灯火下层',
    scene: '甬道分岔',
    protagonistState: '在善意与代价间抉择',
    summary: '出口不止一个，但不是每个出口都还能带着原本的你。',
    news: '地下耳语：有人在甬道深处出售“提前知道结局”的机会。',
    image: STORY_IMAGES.tunnel,
    buildText: (character, world) =>
      `甬道尽头传来断续哭声，也传来关于${world.coreConflict}的完整答案。` +
      ` ${character.name || '无名旅人'}站在潮水与火光之间，必须决定先守住别人，还是先换回属于自己的答案。`,
    choices: [
      {
        id: 'ending-torchbearer',
        text: '先去救人，把灯与退路都交给同伴',
        preview: '你会错过最快得到答案的机会，但会让很多人第一次愿意并肩前行。',
        endingId: 'torchbearer',
      },
      {
        id: 'ending-echobroker',
        text: '接受低语交易，立刻换取幕后真相',
        preview: '答案会来到你手中，但它不会白白停留。',
        endingId: 'echobroker',
      },
    ],
  },
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

function toChoice(choice: DemoChoiceMeta): Choice {
  return {
    id: choice.id,
    text: choice.text,
    disabled: false,
    costHints: {
      intuition: 1,
      obsession: 1,
    },
  };
}

export function buildStoryNode(nodeId: DemoNodeId, character: Character, world: World): StoryNode {
  const node = DEMO_NODES[nodeId];
  return {
    id: node.id,
    text: node.buildText(character, world),
    image: node.image,
    choices: node.choices.map(toChoice),
  };
}

export function buildStateView(
  nodeId: DemoNodeId,
  turnIndex: number,
  history: string,
  latestAction = '观察并做出抉择',
): RuntimeStateView {
  const node = DEMO_NODES[nodeId];
  return {
    gameState: 'playing',
    phase: 'demo_choice',
    turnIndex,
    activeTurnId: turnIndex,
    currentLocation: node.location,
    currentScene: node.scene,
    protagonistState: node.protagonistState,
    npcsState: '守钟人、提灯小贩、港区耳语者',
    latestHistory: history,
    latestBroadcastSummary: node.news,
    latestProtagonistAction: latestAction,
  };
}

export function buildEndingNode(endingId: DemoEndingId, character: Character, world: World): StoryNode {
  const ending = DEMO_ENDINGS[endingId];
  const biography = ending.biography(character, world);
  return {
    id: `ending-${endingId}`,
    text: `${ending.summary}\n\n${biography}\n\n“${ending.legacy}”`,
    image: ending.cgs[1],
    choices: [],
  };
}

export function buildArchiveItem(
  endingId: DemoEndingId,
  character: Character,
  world: World,
  sessionId: string,
): ArchiveListItem {
  const ending = DEMO_ENDINGS[endingId];
  return {
    archiveId: `archive-${sessionId}-${endingId}`,
    title: `${character.name || '无名旅人'} · ${ending.title}`,
    tag: ending.tag,
    era: world.era,
    summary: ending.summary,
    coverImage: ending.cgs[0],
    createdAt: new Date().toISOString(),
  };
}

export function buildSaveItem(
  saveId: string,
  sessionId: string,
  character: Character,
  world: World,
  turnIndex: number,
  currentNode: StoryNode,
): SaveListItem {
  return {
    saveId,
    sessionId,
    title: `${character.name || '无名旅人'} · 第 ${turnIndex || 1} 幕`,
    characterName: character.name || '无名旅人',
    background: character.background || '未写入烙印',
    era: world.era,
    turnIndex,
    summary: currentNode.text.slice(0, 58) + (currentNode.text.length > 58 ? '...' : ''),
    coverImage: currentNode.image,
    savedAt: new Date().toISOString(),
  };
}

export function isDemoNodeId(nodeId: string): nodeId is DemoNodeId {
  return nodeId === 'opening' || nodeId === 'tower' || nodeId === 'tunnel';
}

export function toChoiceFromSession(choice: PendingProtagonistChoice): Choice {
  return {
    id: choice.id,
    text: choice.option.title || choice.option.action,
    previewText: choice.option.motivationAndRisk,
    disabled: false,
    costHints: {
      intuition: 1,
      obsession: 1,
    },
  };
}

export function taskLabel(kind: string): string {
  switch (kind) {
    case 'fate_planning':
      return '命运编织';
    case 'narration':
      return '叙事展开';
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
    protagonistCondition: readJsonString(value.protagonist_condition),
  };
}

function toChoiceFromStreamOption(option: StreamedProtagonistOption, index: number): Choice {
  return {
    id: `choice-${index + 1}`,
    text: option.title?.trim() || option.action?.trim() || `行动 ${index + 1}`,
    previewText: option.motivationAndRisk?.trim() || option.motivation_and_risk?.trim(),
    disabled: false,
    costHints: {
      intuition: 1,
      obsession: 1,
    },
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
        return '主角暂时没有可执行的行动选项。';
      }
      return parsedChoices.map((choice) => choice.text).join(' / ');
    }

    const parsed = JSON.parse(raw) as { options?: StreamedProtagonistOption[] };
    const options = parsed.options ?? [];
    if (options.length === 0) {
      return '主角暂时没有可执行的行动选项。';
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

export function mapGameSessionState(session: GameSessionWorldStateData): ControlledSessionStateSlice {
  const currentTask = session.currentTask;
  const narrationText =
    (currentTask?.kind === 'narration' ? taskText(currentTask)?.trim() : null) ||
    session.latestNarration.trim() ||
    (session.status === 'running' ? STREAM_PLACEHOLDER_TEXT : '命运正在编织，请稍候...');
  const fatePlanningRaw = currentTask?.kind === 'fate_planning' ? taskRawContent(currentTask) : '';
  const protagonistActionRaw =
    currentTask?.kind === 'protagonist_action' ? taskRawContent(currentTask) : '';

  return {
    currentNode: {
      id: `${session.sessionId}-${session.activeTurnId}-${session.phase}`,
      text: narrationText,
      image: STORY_IMAGES.opening,
      choices: session.choices.map(toChoiceFromSession),
    },
    stateView: {
      gameState: 'playing',
      phase: session.phase,
      turnIndex: session.turnIndex,
      activeTurnId: session.activeTurnId,
      currentLocation: session.worldState.locationName || '命运现场',
      currentScene: session.worldState.sceneTitle || '命运展开',
      protagonistState: session.worldState.protagonistCondition || '等待命运显影',
      npcsState: session.worldState.currentEvent || '众生仍在命运中回响',
      latestHistory: narrationText,
      latestBroadcastSummary:
        session.worldState.currentEvent ||
        session.worldState.newInfo[0] ||
        session.worldState.locationStatus ||
        session.worldState.description ||
        narrationText,
      latestProtagonistAction: session.currentProtagonistAction || '尚未做出选择',
    },
    turnIndex: session.turnIndex,
    gameState: 'playing',
    streamedNarrationText: narrationText,
    streamedNarrationStatus: currentTask?.kind === 'narration' ? currentTask.status : null,
    streamedFatePlanningRaw: fatePlanningRaw,
    streamedFatePlanningJson: parseJsonValue(fatePlanningRaw),
    streamedProtagonistActionRaw: protagonistActionRaw,
    streamedProtagonistActionJson: parseJsonValue(protagonistActionRaw),
    isLoading: session.status === 'running',
  };
}

export function applyTaskUpdate(tasks: Map<string, TaskView>, update: TaskUpdatedEvent): TaskView {
  const currentTask = tasks.get(update.entity) ?? {
    entity: update.entity,
    kind: update.kind,
    status: 'pending',
    attempts: 1,
    maxAttempts: 1,
    lastError: null,
    chunks: [],
    output: null,
    error: null,
  };

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
