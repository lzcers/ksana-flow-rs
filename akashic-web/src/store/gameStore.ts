import { create } from 'zustand';

export type GameState = 'lobby' | 'creation' | 'playing' | 'ending' | 'corridor';

export interface Character {
  name: string;
  gender: string;
  age: number;
  appearance: string;
  traits: {
    courage: number; // 0-100 (cautious <-> courageous)
    rationality: number; // 0-100 (emotional <-> rational)
    altruism: number; // 0-100 (selfish <-> altruistic)
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
  isObsessionUsed?: boolean;
}

export interface StoryNode {
  id: string;
  text: string;
  image?: string;
  choices: Choice[];
}

export interface EndingData {
  biography: string;
  turningPoints: { cause: string; effect: string }[];
  legacy: string;
  cgs: string[];
}

interface GameStoreState {
  gameState: GameState;
  character: Character;
  world: World;
  storyNodes: StoryNode[];
  currentNodeId: string | null;
  endingData: EndingData | null;
  
  // Enhanced Gameplay Mechanics
  obsessionPoints: number;
  intuitionPoints: number;
  worldNews: string | null;
  
  // Actions
  setGameState: (state: GameState) => void;
  updateCharacter: (updates: Partial<Character>) => void;
  updateWorld: (updates: Partial<World>) => void;
  addStoryNode: (node: StoryNode) => void;
  makeChoice: (choiceId: string, useObsession?: boolean) => void;
  setEndingData: (data: EndingData) => void;
  resetGame: () => void;
  
  // Enhanced Actions
  useObsession: () => boolean;
  useIntuition: () => boolean;
  triggerWorldNews: (news: string) => void;
  clearWorldNews: () => void;
}

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

export const useGameStore = create<GameStoreState>((set, get) => ({
  gameState: 'lobby',
  character: initialCharacter,
  world: initialWorld,
  storyNodes: [],
  currentNodeId: null,
  endingData: null,
  obsessionPoints: 3,
  intuitionPoints: 5,
  worldNews: null,

  setGameState: (state) => set({ gameState: state }),
  updateCharacter: (updates) => set((state) => ({
    character: { ...state.character, ...updates }
  })),
  updateWorld: (updates) => set((state) => ({
    world: { ...state.world, ...updates }
  })),
  addStoryNode: (node) => set((state) => ({
    storyNodes: [...state.storyNodes, node],
    currentNodeId: node.id
  })),
  useObsession: () => {
    const state = get();
    if (state.obsessionPoints > 0) {
      set({ obsessionPoints: state.obsessionPoints - 1 });
      return true;
    }
    return false;
  },
  useIntuition: () => {
    const state = get();
    if (state.intuitionPoints > 0) {
      set({ intuitionPoints: state.intuitionPoints - 1 });
      return true;
    }
    return false;
  },
  triggerWorldNews: (news) => {
    set({ worldNews: news });
    setTimeout(() => {
      set((state) => (state.worldNews === news ? { worldNews: null } : {}));
    }, 5000);
  },
  clearWorldNews: () => set({ worldNews: null }),
  makeChoice: (choiceId, useObsession = false) => {
    // Mock the next node generation based on the choice
    const state = get();
    const currentNode = state.storyNodes.find(n => n.id === state.currentNodeId);
    if (!currentNode) return;

    const chosenChoice = currentNode.choices.find(c => c.id === choiceId);
    const chosenText = chosenChoice?.text || '';
    
    // Simulate world news occasionally
    if (Math.random() < 0.3) {
      const newsEvents = [
        "远方城邦燃起烽火，旧秩序摇摇欲坠。",
        "神秘流星划破夜空，引发恐慌与狂热。",
        "市场物价剧烈波动，暗流涌动。",
        "古代遗迹发出共鸣，异象频生。"
      ];
      const randomNews = newsEvents[Math.floor(Math.random() * newsEvents.length)];
      state.triggerWorldNews(randomNews);
    }
    
    // Determine if we should end the game (e.g. after 3 nodes)
    if (state.storyNodes.length >= 3) {
      state.setEndingData({
        biography: `${state.character.name}的《此生回响录》：\n在${state.world.era}的时代，面对${state.world.coreConflict}的残酷现实，你以${state.character.background}的身份，走出了一条独特的道路。最终，你做出了关键的抉择：“${chosenText}”，为这段历史画上了休止符。`,
        turningPoints: [
          { cause: "面对未知的星辰", effect: "选择毫不犹豫地踏上旅程" },
          { cause: "遭遇遗迹守卫的考验", effect: "以智慧化解了危机" },
          { cause: "面临最终的抉择", effect: `选择了：${chosenText}${useObsession ? " (动用了执念)" : ""}` }
        ],
        legacy: "精神遗产评估：你的选择为世界留下了一丝希望的火种。",
        cgs: [
          "https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=A%20dramatic%20crossroads%20in%20a%20sci-fi%20or%20fantasy%20world%2C%20epic%20lighting%2C%20concept%20art&image_size=landscape_16_9",
          "https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=An%20ancient%20ruin%20with%20glowing%20runes%2C%20mysterious%20atmosphere%2C%20digital%20painting&image_size=landscape_16_9",
          "https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=A%20heroic%20figure%20looking%20at%20a%20beautiful%20sunrise%20over%20a%20futuristic%20city%2C%20hopeful%20ending%2C%20cinematic&image_size=landscape_16_9"
        ]
      });
      return;
    }

    // Otherwise, create a new node
    const newNodeId = `node-${state.storyNodes.length + 1}`;
    
    // Simulate NPC memory intervention occasionally
    const hasNpcIntervention = Math.random() < 0.25;
    const npcInterventionText = hasNpcIntervention ? 
      "\n\n[记忆介入] 一封没有署名的信件悄然送达，信中提到了你之前的行为：“我看到了你当时的决断，这让我对你刮目相看。但前方的路，你还能走多远？”\n" : "";
      
    const obsessionText = useObsession ? "\n\n[执念倾注] 你将强烈的执念倾注于此抉择中，某种不可名状的力量回应了你，但也带来了一丝反噬的阴影。" : "";

    const newNode: StoryNode = {
      id: newNodeId,
      text: `你选择了“${chosenText}”。\n\n随着你的决定，命运的齿轮再次转动。蝴蝶效应开始显现，周围的环境发生了微妙的变化。前方的道路更加扑朔迷离。${obsessionText}${npcInterventionText}`,
      image: 'https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=An%20ancient%20ruin%20with%20glowing%20runes%2C%20mysterious%20atmosphere%2C%20digital%20painting&image_size=landscape_16_9',
      choices: [
        { id: `${newNodeId}-c1`, text: '继续深入探索未知' },
        { id: `${newNodeId}-c2`, text: '寻找盟友的帮助' },
        { id: `${newNodeId}-c3`, text: '谨慎地观察四周' }
      ]
    };
    
    state.addStoryNode(newNode);
  },
  setEndingData: (data) => set({ endingData: data, gameState: 'ending' }),
  resetGame: () => set({
    gameState: 'lobby',
    character: initialCharacter,
    world: initialWorld,
    storyNodes: [],
    currentNodeId: null,
    endingData: null,
    obsessionPoints: 3,
    intuitionPoints: 5,
    worldNews: null,
  })
}));
