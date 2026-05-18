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
