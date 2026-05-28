import { create } from 'zustand';
import type { Choice, TaskView } from '../lib/api';

export type RoundChoicesStatus = 'idle' | 'loading' | 'ready';

export interface RoundState {
  round: number;
  title: string;
  narrationText: string;
  narrationStatus: TaskView['status'] | null;
  choices: Choice[];
  choicesStatus: RoundChoicesStatus;
  selectedChoiceText: string | null;
  isAwaitingNarration: boolean;
}

export interface GameInternalState {
  // 内部状态：当前后端会话 id，用于继续推进会话。
  sessionId: string | null;
  // 内部状态：当前已推进到的回合序号。
  turnIndex: number;
  // 内部状态：当前页面应展示的回合序号，与服务端 turnIndex 解耦。
  displayRound: number;
  // 内部状态：按轮次隔离的叙事/选项时间线。
  roundStates: Record<number, RoundState>;
}

export const initialInternalState: GameInternalState = {
  sessionId: null,
  turnIndex: 0,
  displayRound: 0,
  roundStates: {},
};

export function createRoundState(round: number, overrides: Partial<RoundState> = {}): RoundState {
  return {
    round,
    title: '',
    narrationText: '',
    narrationStatus: null,
    choices: [],
    choicesStatus: 'idle',
    selectedChoiceText: null,
    isAwaitingNarration: false,
    ...overrides,
  };
}

export const useGameInternalStore = create<GameInternalState>(() => ({
  ...initialInternalState,
}));
