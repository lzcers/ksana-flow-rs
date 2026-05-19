import { create } from 'zustand';
import {
  controlGameSession,
  createGameSession,
  getGameSessionWorld,
  openGameSessionStream,
} from '../lib/api';
import type {
  ArchiveListItem,
  Character,
  Choice,
  GameSessionWorldStateData,
  PendingProtagonistChoice,
  RuntimeStateView,
  SaveListItem,
  StoryNode,
  TaskView,
  World,
} from '../lib/api';

export type GameState = 'lobby' | 'creation' | 'playing';

interface GameStoreState {
  gameState: GameState;
  sessionId: string | null;
  character: Character;
  world: World;
  currentNode: StoryNode | null;
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
  submitChoice: (choiceId: string, useObsession?: boolean) => Promise<void>;
  previewChoice: (choiceId: string) => Promise<string>;
  createSave: (title?: string) => Promise<string>;
  loadSave: (saveId: string) => Promise<void>;
  resetGame: () => void;
}

type DemoNodeId = 'opening' | 'tower' | 'tunnel';
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

interface SaveSnapshot {
  sessionId: string;
  character: Character;
  world: World;
  currentNodeId: DemoNodeId;
  turnIndex: number;
  obsessionPoints: number;
  intuitionPoints: number;
  daysLeft: number;
  worldNews: string | null;
}

interface StreamedProtagonistOption {
  title?: string;
  action?: string;
  motivation_and_risk?: string;
  motivationAndRisk?: string;
}

const image = (prompt: string) =>
  `https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=${encodeURIComponent(prompt)}&image_size=landscape_16_9`;

const STORY_IMAGES = {
  opening: image('mystical dark fantasy city street emerging from mist, cinematic story game background'),
  tower: image('clock tower interior with glowing gears and rain outside, dark fantasy cinematic concept art'),
  tunnel: image('hidden underground tunnel with lantern reflections and ancient glyphs, dark fantasy cinematic concept art'),
  endingHope: image('hero standing before dawn over a dark city, cinematic dark fantasy illustration'),
  endingShadow: image('solitary figure beside shattered mirror and moonlit corridor, cinematic dark fantasy illustration'),
  corridor: image('vast mystical hall of memory, dark blue archive walls, golden lines, cinematic concept art'),
};

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

const DEMO_ENDINGS: Record<
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

const DEMO_NODES: Record<DemoNodeId, DemoNodeMeta> = {
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

const seededArchives: ArchiveListItem[] = [
  {
    archiveId: 'archive-seeded-1',
    title: '旧馆藏 · 雨夜抄本',
    tag: '示例档案',
    era: '东方玄幻',
    summary: '一份预置的演示馆藏，用来展示本地归档在无后端时的视觉布局。',
    coverImage: STORY_IMAGES.corridor,
    createdAt: '2026-05-18T00:00:00.000Z',
  },
];

const seededSaves: SaveListItem[] = [
  {
    saveId: 'save-seeded-1',
    sessionId: 'demo-seeded',
    title: '示例存档 · 港区雨幕',
    characterName: '演示旅人',
    background: '寻梦的学者',
    era: '蒸汽朋克',
    turnIndex: 1,
    summary: '用于展示“进行中存档”样式，不依赖任何后端返回。',
    coverImage: STORY_IMAGES.opening,
    savedAt: '2026-05-18T00:00:00.000Z',
  },
];

const saveSnapshots = new Map<string, SaveSnapshot>([
  [
    'save-seeded-1',
    {
      sessionId: 'demo-seeded',
      character: {
        name: '演示旅人',
        gender: '保密',
        age: 22,
        appearance: '披着仍带雨意的长风衣，袖口藏着一支记事银笔',
        traits: { courage: 56, rationality: 68, altruism: 61 },
        background: '寻梦的学者',
      },
      world: {
        era: '蒸汽朋克',
        coreConflict: '旧档案馆深处正在泄露不属于这个时代的预言',
        specialRules: [],
      },
      currentNodeId: 'opening',
      turnIndex: 1,
      obsessionPoints: 3,
      intuitionPoints: 4,
      daysLeft: 6,
      worldNews: DEMO_NODES.opening.news,
    },
  ],
]);

const initialState = {
  gameState: 'lobby' as GameState,
  sessionId: null,
  character: initialCharacter,
  world: initialWorld,
  currentNode: null,
  stateView: null,
  obsessionPoints: 3,
  intuitionPoints: 5,
  daysLeft: 7,
  worldNews: null,
  turnIndex: 0,
  saves: seededSaves,
  archives: seededArchives,
  latestSaveId: null,
  latestArchiveId: seededArchives[0]?.archiveId ?? null,
  isLoading: false,
  error: null,
};

let activeSessionStream: EventSource | null = null;
let activeStreamSessionId: string | null = null;
let lastStreamEventId: string | null = null;
const STREAM_PLACEHOLDER_TEXT = '命运正在展开，请稍候...';

function closeActiveSessionStream() {
  activeSessionStream?.close();
  activeSessionStream = null;
  activeStreamSessionId = null;
  lastStreamEventId = null;
}

function cloneCharacter(character: Character): Character {
  return {
    ...character,
    traits: { ...character.traits },
  };
}

function cloneWorld(world: World): World {
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

function buildStoryNode(nodeId: DemoNodeId, character: Character, world: World): StoryNode {
  const node = DEMO_NODES[nodeId];
  return {
    id: node.id,
    text: node.buildText(character, world),
    image: node.image,
    choices: node.choices.map(toChoice),
  };
}

function buildStateView(
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
    latestBroadcastSummary: node.summary,
    latestProtagonistAction: latestAction,
  };
}

function buildEndingNode(endingId: DemoEndingId, character: Character, world: World): StoryNode {
  const ending = DEMO_ENDINGS[endingId];
  const biography = ending.biography(character, world);
  return {
    id: `ending-${endingId}`,
    text: `${ending.summary}\n\n${biography}\n\n“${ending.legacy}”`,
    image: ending.cgs[1],
    choices: [],
  };
}

function buildArchiveItem(
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

function buildSaveItem(
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

function isDemoNodeId(nodeId: string): nodeId is DemoNodeId {
  return nodeId === 'opening' || nodeId === 'tower' || nodeId === 'tunnel';
}

function resetPlayState(state: GameStoreState) {
  return {
    gameState: 'lobby' as GameState,
    sessionId: null,
    character: cloneCharacter(initialCharacter),
    world: cloneWorld(initialWorld),
    currentNode: null,
    stateView: null,
    obsessionPoints: 3,
    intuitionPoints: 5,
    daysLeft: 7,
    worldNews: null,
    turnIndex: 0,
    latestSaveId: null,
    latestArchiveId: state.latestArchiveId,
    isLoading: false,
    error: null,
  };
}

function toChoiceFromSession(choice: PendingProtagonistChoice): Choice {
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

function taskLabel(kind: string): string {
  switch (kind) {
    case 'fate_planning':
      return '命运织线';
    case 'narration':
      return '叙事展开';
    case 'protagonist_action':
      return '主角抉择';
    default:
      return '命运推进';
  }
}

function taskContent(task: TaskView): string | null {
  if (task.status === 'done' && task.output != null) {
    return task.output;
  }

  if (task.chunks.length > 0) {
    return task.chunks.join('');
  }

  return task.output;
}

function taskText(task: TaskView): string | null {
  const text = taskContent(task);
  if (!text?.trim()) {
    return null;
  }
  if (task.kind === 'narration') {
    return text;
  }
  return null;
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

function protagonistActionChoices(task: TaskView): Choice[] | null {
  if (task.kind !== 'protagonist_action') {
    return null;
  }

  const raw = taskContent(task);
  if (!raw?.trim()) {
    return null;
  }

  return parseStreamingProtagonistChoices(raw);
}

function protagonistActionText(task: TaskView): string | null {
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

function persistedNarrationText(session: GameSessionWorldStateData): string {
  const taskNarration = session.tasks.find(
    (task) => task.kind === 'narration' && task.status === 'done' && task.output?.trim(),
  );
  if (taskNarration?.output?.trim()) {
    return taskNarration.output.trim();
  }

  if (!isSessionLoading(session) && session.latestNarration.trim()) {
    return session.latestNarration.trim();
  }

  return '';
}

function sessionText(session: GameSessionWorldStateData, fallbackText?: string | null): string {
  return (
    fallbackText?.trim() ||
    persistedNarrationText(session) ||
    '命运正在编织，请稍候...'
  );
}

function sessionNews(session: GameSessionWorldStateData): string | null {
  return (
    session.worldState.currentEvent ||
    session.worldState.newInfo[0] ||
    session.worldState.locationStatus ||
    null
  );
}

function mapSessionToPlayState(
  session: GameSessionWorldStateData,
  fallbackText?: string | null,
): Pick<GameStoreState, 'currentNode' | 'stateView' | 'worldNews' | 'turnIndex' | 'gameState'> {
  const text = sessionText(session, fallbackText);

  return {
    currentNode: {
      id: `${session.sessionId}-${session.activeTurnId}-${session.phase}`,
      text,
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
      latestHistory: text,
      latestBroadcastSummary: session.worldState.description || text,
      latestProtagonistAction: session.currentProtagonistAction || '尚未做出选择',
    },
    worldNews: sessionNews(session),
    turnIndex: session.turnIndex,
    gameState: 'playing',
  };
}

function isSessionLoading(session: GameSessionWorldStateData): boolean {
  return session.status === 'running';
}

async function refreshSessionState(
  sessionId: string,
  set: (partial: Partial<GameStoreState> | ((state: GameStoreState) => Partial<GameStoreState>)) => void,
) {
  const session = await getGameSessionWorld(sessionId);
  if (activeStreamSessionId !== sessionId) {
    return;
  }

  set((state) => ({
    ...mapSessionToPlayState(session, state.currentNode?.text),
    isLoading: isSessionLoading(session),
    error: null,
  }));
}

export const useGameStore = create<GameStoreState>((set, get) => ({
  ...initialState,
  setGameState: (state) => {
    if (state !== 'playing') {
      closeActiveSessionStream();
    }
    set({ gameState: state, error: null });
  },
  updateCharacter: (updates) =>
    set((state) => ({
      character: {
        ...state.character,
        ...updates,
        traits: updates.traits ? { ...state.character.traits, ...updates.traits } : state.character.traits,
      },
    })),
  updateWorld: (updates) =>
    set((state) => ({
      world: {
        ...state.world,
        ...updates,
        specialRules: updates.specialRules ?? state.world.specialRules,
      },
    })),
  clearError: () => set({ error: null }),
  startGame: async () => {
    const { character, world } = get();
    closeActiveSessionStream();
    set({
      sessionId: null,
      currentNode: {
        id: 'loading',
        text: STREAM_PLACEHOLDER_TEXT,
        image: STORY_IMAGES.opening,
        choices: [],
      },
      stateView: {
        gameState: 'playing',
        phase: 'booting',
        turnIndex: 0,
        activeTurnId: 0,
        currentLocation: '命运现场',
        currentScene: '命运编织中',
        protagonistState: `${character.name || '无名旅人'} 正踏入 ${world.era}`,
        npcsState: '诸多回响正在汇聚',
        latestHistory: STREAM_PLACEHOLDER_TEXT,
        latestBroadcastSummary: world.coreConflict,
        latestProtagonistAction: '尚未做出选择',
      },
      obsessionPoints: 3,
      intuitionPoints: 5,
      daysLeft: 7,
      worldNews: '正在创建会话并唤起第一轮命运...',
      turnIndex: 0,
      latestSaveId: null,
      error: null,
      gameState: 'playing',
      isLoading: true,
    });

    try {
      const created = await createGameSession(character, world);
      activeStreamSessionId = created.sessionId;

      set({
        sessionId: created.sessionId,
        worldNews: '会话已建立，正在推进第一轮...',
      });

      const controlled = await controlGameSession(created.sessionId, {
        control: { type: 'continue' },
      });
      set({
        sessionId: created.sessionId,
        ...mapSessionToPlayState(controlled.session),
        isLoading: true,
        error: null,
      });

      activeSessionStream = openGameSessionStream(
        created.sessionId,
        {
          onTaskUpdated: (event, lastEventId) => {
            if (activeStreamSessionId !== created.sessionId) {
              return;
            }
            lastStreamEventId = lastEventId || lastStreamEventId;
            const nextText = taskText(event.task);
            const nextChoices = protagonistActionChoices(event.task);

            set((state) => ({
              worldNews: `命运编织中：${taskLabel(event.task.kind)}`,
              currentNode: state.currentNode
                ? {
                  ...state.currentNode,
                  text:
                    nextText ??
                    (state.currentNode.text === STREAM_PLACEHOLDER_TEXT ? '' : state.currentNode.text),
                  choices:
                    nextChoices != null
                      ? nextChoices
                      : event.task.kind === 'narration'
                        ? []
                        : state.currentNode.choices,
                }
                : null,
              stateView: state.stateView
                ? {
                  ...state.stateView,
                  currentScene: taskLabel(event.task.kind),
                  latestProtagonistAction:
                    protagonistActionText(event.task) ?? state.stateView.latestProtagonistAction,
                  latestHistory:
                    nextText ??
                    (state.stateView.latestHistory === STREAM_PLACEHOLDER_TEXT ? '' : state.stateView.latestHistory),
                  latestBroadcastSummary: nextText ?? state.stateView.latestBroadcastSummary,
                }
                : null,
            }));

            if (event.task.status !== 'running') {
              void refreshSessionState(created.sessionId, set).catch((error) => {
                set({
                  isLoading: false,
                  error: error instanceof Error ? error.message : '刷新会话状态失败。',
                });
              });
            }
          },
          onError: () => {
            if (activeStreamSessionId !== created.sessionId) {
              return;
            }
            set({
              error: '叙事流连接出现波动，正在尝试恢复...',
            });
          },
        },
        lastStreamEventId,
      );

      await refreshSessionState(created.sessionId, set);
    } catch (error) {
      closeActiveSessionStream();
      set({
        isLoading: false,
        error: error instanceof Error ? error.message : '开启旅程失败。',
      });
      throw error;
    }
  },
  submitChoice: async (choiceId, useObsession = false) => {
    const {
      sessionId,
      character,
      world,
      currentNode,
      obsessionPoints,
      intuitionPoints,
      daysLeft,
      turnIndex,
      archives,
    } = get();

    if (!sessionId || !currentNode) {
      throw new Error('当前没有可推进的演示剧情。');
    }

    if (activeStreamSessionId === sessionId) {
      const nextObsession = useObsession ? Math.max(0, obsessionPoints - 1) : obsessionPoints;
      const nextDaysLeft = Math.max(1, daysLeft - 1);

      set({
        isLoading: true,
        obsessionPoints: nextObsession,
        intuitionPoints,
        daysLeft: nextDaysLeft,
        currentNode: {
          ...currentNode,
          choices: [],
        },
        error: null,
      });

      try {
        const controlled = await controlGameSession(sessionId, {
          choice: { choiceId },
        });

        set({
          ...mapSessionToPlayState(controlled.session),
          isLoading: true,
          error: null,
        });

        await refreshSessionState(sessionId, set);
        return;
      } catch (error) {
        set({
          isLoading: false,
          error: error instanceof Error ? error.message : '提交选择失败。',
        });
        throw error;
      }
    }

    const currentMeta = DEMO_NODES[currentNode.id as DemoNodeId];
    const choice = currentMeta.choices.find((item) => item.id === choiceId);

    if (!choice) {
      throw new Error('当前选择不存在。');
    }

    const nextObsession = useObsession ? Math.max(0, obsessionPoints - 1) : obsessionPoints;
    const nextDaysLeft = Math.max(1, daysLeft - 1);

    if (choice.nextNodeId) {
      const nextNode = buildStoryNode(choice.nextNodeId, character, world);
      set({
        currentNode: nextNode,
        stateView: buildStateView(choice.nextNodeId, turnIndex + 1, nextNode.text, choice.text),
        worldNews: DEMO_NODES[choice.nextNodeId].news,
        obsessionPoints: nextObsession,
        intuitionPoints,
        daysLeft: nextDaysLeft,
        turnIndex: turnIndex + 1,
        error: null,
        gameState: 'playing',
      });
      return;
    }

    if (!choice.endingId) {
      throw new Error('演示剧情缺少结局配置。');
    }

    const endingNode = buildEndingNode(choice.endingId, character, world);
    const ending = DEMO_ENDINGS[choice.endingId];
    const archiveItem = buildArchiveItem(choice.endingId, character, world, sessionId);

    set({
      currentNode: endingNode,
      stateView: {
        gameState: 'playing',
        phase: 'demo_complete',
        turnIndex: turnIndex + 1,
        activeTurnId: turnIndex + 1,
        currentLocation: currentMeta.location,
        currentScene: '人生回响',
        protagonistState: '命运已收束',
        npcsState: '馆藏记录已生成',
        latestHistory: endingNode.text,
        latestBroadcastSummary: ending.summary,
        latestProtagonistAction: choice.text,
      },
      archives: [archiveItem, ...archives.filter((item) => item.archiveId !== archiveItem.archiveId)],
      latestArchiveId: archiveItem.archiveId,
      obsessionPoints: nextObsession,
      intuitionPoints,
      daysLeft: nextDaysLeft,
      turnIndex: turnIndex + 1,
      worldNews: '命运收束：本次旅程已被写入本地归档演示数据。',
      error: null,
      gameState: 'playing',
    });
  },
  previewChoice: async (choiceId) => {
    const { currentNode, intuitionPoints } = get();

    if (!currentNode) {
      throw new Error('当前没有可预览的剧情节点。');
    }

    if (intuitionPoints <= 0) {
      throw new Error('演示直觉点已耗尽。');
    }

    const sessionChoice = currentNode.choices.find((item) => item.id === choiceId);
    if (sessionChoice?.previewText) {
      set({
        intuitionPoints: Math.max(0, intuitionPoints - 1),
        error: null,
      });
      return sessionChoice.previewText;
    }

    const currentMeta = DEMO_NODES[currentNode.id as DemoNodeId];
    const choice = currentMeta.choices.find((item) => item.id === choiceId);

    if (!choice) {
      throw new Error('当前选择不存在。');
    }

    set({
      intuitionPoints: Math.max(0, intuitionPoints - 1),
      error: null,
    });

    return choice.preview;
  },
  createSave: async (title) => {
    const {
      sessionId,
      character,
      world,
      currentNode,
      turnIndex,
      obsessionPoints,
      intuitionPoints,
      daysLeft,
      worldNews,
      saves,
    } = get();

    if (!sessionId || !currentNode) {
      throw new Error('当前没有可保存的演示旅程。');
    }

    if (!isDemoNodeId(currentNode.id)) {
      throw new Error('当前收束片段不支持存档，请返回大厅开启新人生。');
    }

    const saveId = `save-${Date.now()}`;
    const item = buildSaveItem(saveId, sessionId, character, world, turnIndex, currentNode);
    const finalItem = title?.trim() ? { ...item, title: title.trim() } : item;

    saveSnapshots.set(saveId, {
      sessionId,
      character: cloneCharacter(character),
      world: cloneWorld(world),
      currentNodeId: currentNode.id as DemoNodeId,
      turnIndex,
      obsessionPoints,
      intuitionPoints,
      daysLeft,
      worldNews,
    });

    set({
      saves: [finalItem, ...saves.filter((save) => save.saveId !== saveId)],
      latestSaveId: saveId,
      error: null,
    });

    return saveId;
  },
  loadSave: async (saveId) => {
    const snapshot = saveSnapshots.get(saveId);

    if (!snapshot) {
      throw new Error('未找到对应的本地演示存档。');
    }

    const currentNode = buildStoryNode(snapshot.currentNodeId, snapshot.character, snapshot.world);

    set({
      sessionId: snapshot.sessionId,
      character: cloneCharacter(snapshot.character),
      world: cloneWorld(snapshot.world),
      currentNode,
      stateView: buildStateView(snapshot.currentNodeId, snapshot.turnIndex, currentNode.text, '从本地存档恢复旅程'),
      obsessionPoints: snapshot.obsessionPoints,
      intuitionPoints: snapshot.intuitionPoints,
      daysLeft: snapshot.daysLeft,
      worldNews: snapshot.worldNews,
      turnIndex: snapshot.turnIndex,
      latestSaveId: saveId,
      error: null,
      gameState: 'playing',
    });
  },
  resetGame: () => {
    closeActiveSessionStream();
    set((state) => ({
      ...state,
      ...resetPlayState(state),
    }));
  },
}));
