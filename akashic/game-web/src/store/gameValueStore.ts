import { create } from 'zustand';

const INITIAL_OBSESSION_POINTS = 3;
const INTUITION_POINTS_PER_ROUND = 2;

interface GameValueState {
  obsessionPoints: number;
  intuitionPoints: number;
  currentRound: number;
}

interface GameValueActions {
  resetValues: (round?: number) => void;
  syncRound: (round: number) => void;
  consumeObsession: () => void;
  consumeIntuition: () => void;
}

type GameValueStoreState = GameValueState & GameValueActions;

function normalizeRound(round: number): number {
  if (!Number.isFinite(round)) {
    return 1;
  }

  return Math.max(Math.trunc(round), 1);
}

function obsessionGainBetweenRounds(currentRound: number, nextRound: number): number {
  return Math.max(
    Math.floor((nextRound - 1) / 3) - Math.floor((currentRound - 1) / 3),
    0,
  );
}

export const useGameValueStore = create<GameValueStoreState>((set, get) => ({
  obsessionPoints: INITIAL_OBSESSION_POINTS,
  intuitionPoints: INTUITION_POINTS_PER_ROUND,
  currentRound: 1,
  resetValues: (round = 1) => {
    const nextRound = normalizeRound(round);
    set({
      obsessionPoints: INITIAL_OBSESSION_POINTS,
      intuitionPoints: INTUITION_POINTS_PER_ROUND,
      currentRound: nextRound,
    });
  },
  syncRound: (round) => {
    const nextRound = normalizeRound(round);
    const { currentRound, obsessionPoints } = get();

    if (nextRound === currentRound) {
      return;
    }

    if (nextRound < currentRound) {
      set({
        currentRound: nextRound,
        intuitionPoints: INTUITION_POINTS_PER_ROUND,
      });
      return;
    }

    set({
      currentRound: nextRound,
      intuitionPoints: INTUITION_POINTS_PER_ROUND,
      obsessionPoints: obsessionPoints + obsessionGainBetweenRounds(currentRound, nextRound),
    });
  },
  consumeObsession: () => {
    const { obsessionPoints } = get();
    if (obsessionPoints <= 0) {
      throw new Error('执念点不足。');
    }

    set({
      obsessionPoints: obsessionPoints - 1,
    });
  },
  consumeIntuition: () => {
    const { intuitionPoints } = get();
    if (intuitionPoints <= 0) {
      throw new Error('本轮直觉已耗尽。');
    }

    set({
      intuitionPoints: intuitionPoints - 1,
    });
  },
}));
