import type {
  GameSessionWorldStateData,
  RuntimeStateView,
  SessionRoundHistoryData,
} from '../lib/api';
import {
  createRoundState,
  type GameInternalState,
  type RoundState,
} from './gameStore';
import {
  STREAM_PLACEHOLDER_TEXT,
  toChoiceFromSession,
} from './gameStoreHelpers';

function titleFromWorldState(
  worldState: SessionRoundHistoryData['worldState'] | GameSessionWorldStateData['worldState'] | null | undefined,
  fallback = '命运回响',
): string {
  return worldState?.sceneTitle?.trim() || fallback;
}

export function effectiveDisplayRound(session: GameSessionWorldStateData): number {
  if (session.phase === 'awaiting_player') {
    return Math.max(session.activeTurnId, 1);
  }

  return Math.max(session.turnIndex, session.activeTurnId, 1);
}

function latestHistoryFromSession(session: GameSessionWorldStateData): string {
  return session.latestNarration.trim()
    || session.worldState.description.trim()
    || STREAM_PLACEHOLDER_TEXT;
}

function latestBroadcastItemsFromSession(session: GameSessionWorldStateData): string[] {
  const nextItems = session.worldState.newInfo
    .map((item) => item.trim())
    .filter(Boolean);
  if (nextItems.length > 0) {
    return nextItems;
  }

  const fallback = session.worldState.currentEvent.trim() || session.worldState.description.trim();
  return fallback ? [fallback] : [];
}

export function stateViewFromSession(session: GameSessionWorldStateData): RuntimeStateView {
  const latestBroadcastItems = latestBroadcastItemsFromSession(session);
  return {
    gameState: 'playing',
    phase: session.phase,
    turnIndex: session.turnIndex,
    activeTurnId: session.activeTurnId,
    currentLocation: session.worldState.locationName || '命运现场',
    currentScene: session.worldState.sceneTitle || '命运回响',
    protagonistState: session.worldState.protagonistCondition || '命运仍在酝酿',
    npcsState: session.worldState.currentEvent || '诸多回响正在汇聚',
    latestHistory: latestHistoryFromSession(session),
    latestBroadcastSummary:
      session.worldState.currentEvent
      || session.worldState.description
      || '旅程已续上',
    latestBroadcastItems,
    latestProtagonistAction: session.currentProtagonistAction || '你还没有做出选择',
    isEnding: session.worldState.isEnding,
    endingType: session.worldState.endingType ?? null,
  };
}

export function internalStateFromSession(session: GameSessionWorldStateData): GameInternalState {
  const round = effectiveDisplayRound(session);
  if (session.history.length > 0) {
    const roundStates = session.history.reduce<Record<number, RoundState>>((acc, entry) => {
      acc[entry.round] = roundStateFromHistoryEntry(entry, session, round);
      return acc;
    }, {});

    if (!roundStates[round]) {
      roundStates[round] = currentRoundStateFromSession(session, round);
    }

    return {
      sessionId: session.sessionId,
      turnIndex: session.turnIndex,
      displayRound: round,
      roundStates,
    };
  }

  return {
    sessionId: session.sessionId,
    turnIndex: session.turnIndex,
    displayRound: round,
    roundStates: {
      [round]: currentRoundStateFromSession(session, round),
    },
  };
}

function roundStateFromHistoryEntry(
  entry: SessionRoundHistoryData,
  session: GameSessionWorldStateData,
  currentRound: number,
): RoundState {
  const isCurrentRound = entry.round === currentRound;
  const choices = (isCurrentRound ? session.choices : entry.choices).map(toChoiceFromSession);
  const selectedChoiceText = entry.selectedChoiceText?.trim()
    || deriveSelectedChoiceText(entry)
    || null;
  const narrationText = entry.narrationText.trim()
    || (isCurrentRound ? latestHistoryFromSession(session) : '');

  return createRoundState(entry.round, {
    title: titleFromWorldState(entry.worldState, titleFromWorldState(session.worldState)),
    narrationText,
    narrationStatus: isCurrentRound && session.currentTask?.kind === 'narration'
      ? session.currentTask.status
      : entry.narrationText.trim()
        ? 'done'
        : null,
    choices,
    choicesStatus: choices.length > 0 ? 'ready' : 'idle',
    selectedChoiceText,
    isAwaitingNarration: false,
  });
}

function deriveSelectedChoiceText(entry: SessionRoundHistoryData): string | null {
  const committedAction = entry.committedAction?.trim();
  if (!committedAction) {
    return null;
  }

  const matchedChoice = entry.choices.find((choice) => choice.option.action === committedAction);
  return matchedChoice?.option.title || committedAction;
}

function currentRoundStateFromSession(
  session: GameSessionWorldStateData,
  round: number,
): RoundState {
  return {
    ...createRoundState(round, {
      title: titleFromWorldState(session.worldState),
      narrationText: latestHistoryFromSession(session),
      narrationStatus: session.currentTask?.kind === 'narration' ? session.currentTask.status : null,
      choices: session.choices.map(toChoiceFromSession),
      choicesStatus: session.choices.length > 0 ? 'ready' : 'idle',
      selectedChoiceText: null,
      isAwaitingNarration: false,
    }),
  };
}
