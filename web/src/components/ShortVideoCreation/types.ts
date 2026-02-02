export interface ScriptData {
  content: string;
}

export interface CharacterData {
  id: string;
  name: string;
  avatar?: string;
  description: string;
  tags: string[];
}

export interface StoryboardShot {
  id: string;
  shotNo: number;
  image?: string; // URL or base64
  description: {
    background: string;
    relation: string;
    composition: string;
  };
  lines: {
    narration?: string;
    dialogue?: string;
  };
  mainCharacter: string;
  shotSize: string; // e.g., "特写", "近景", "中景"
  cameraAngle: string; // e.g., "视平", "俯平"
  lensType: string; // e.g., "单人镜头"
  duration: number; // seconds
}

export interface ProjectData {
  characters: CharacterData[];
  storyboard: StoryboardShot[];
}

export type ModuleType = 'script' | 'character' | 'storyboard';

export interface ShortVideoCreationProps {
  data: ProjectData;
  onBack?: () => void;
  onDataChange?: (data: ProjectData) => void;
  isNodeCompleted?: (value: unknown) => boolean;
}
