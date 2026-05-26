import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { useGameInternalStore, useGameUIStore } from '../store/gameStore';
import {
  ScreenShell,
  StoryFrame,
} from '../components/AkashicUI';
import GameplayHeader from '../components/GameplayHeader';
import ChoicePanel from '../components/ChoicePanel';
import GameplayToolbar from '../components/GameplayToolbar';
import NarrationPanel from '../components/NarrationPanel';
import type { NarrationRoundEntry } from '../components/gameplayTypes';
import type { Choice } from '../lib/api';

const EMPTY_BROADCAST_ITEMS: string[] = [];

const GameplayPage: React.FC = () => {
  const {
    obsessionPoints,
    intuitionPoints,
    turnIndex,
    currentScene,
    latestBroadcastItems,
    latestBroadcastSummary,
    isLoading,
    error,
  } = useGameUIStore(useShallow((state) => ({
    obsessionPoints: state.obsessionPoints,
    intuitionPoints: state.intuitionPoints,
    turnIndex: state.stateView?.turnIndex,
    currentScene: state.stateView?.currentScene ?? '',
    latestBroadcastItems: state.stateView?.latestBroadcastItems ?? EMPTY_BROADCAST_ITEMS,
    latestBroadcastSummary: state.stateView?.latestBroadcastSummary ?? '',
    isLoading: state.isLoading,
    error: state.error,
  })));
  const {
    createSave,
    submitChoice,
    previewChoice,
    setGameState,
  } = useGameUIStore(useShallow((state) => ({
    createSave: state.createSave,
    submitChoice: state.submitChoice,
    previewChoice: state.previewChoice,
    setGameState: state.setGameState,
  })));
  const displayRound = useGameInternalStore((state) => state.displayRound);
  const roundStates = useGameInternalStore((state) => state.roundStates);
  const [completedTypingKey, setCompletedTypingKey] = useState<string | null>(null);
  const [activeObsession, setActiveObsession] = useState(false);
  const [obsessionInput, setObsessionInput] = useState('');
  const [previews, setPreviews] = useState<Record<string, string>>({});
  const [feedback, setFeedback] = useState<string | null>(null);

  const currentRound = Math.max(displayRound || turnIndex || 1, 1);
  const narrationHistory = useMemo<NarrationRoundEntry[]>(() => (
    Object.values(roundStates)
      .filter((entry) => entry.narrationText || entry.selectedChoiceText || entry.isAwaitingNarration)
      .sort((left, right) => left.round - right.round)
  ), [roundStates]);
  const activeRoundState = roundStates[currentRound];
  const currentRoundChoices = activeRoundState?.choices ?? [];
  const hasChoices = currentRoundChoices.length > 0;
  const isNarrationStreaming = activeRoundState?.narrationStatus === 'pending'
    || activeRoundState?.narrationStatus === 'running';
  const shouldType = Boolean(activeRoundState?.isAwaitingNarration) || isNarrationStreaming;
  const typingKey = `${currentRound}:${activeRoundState?.isAwaitingNarration ? '1' : '0'}:${activeRoundState?.narrationText ?? ''}`;
  const isTyping = shouldType && completedTypingKey !== typingKey;
  const isChoiceInteractionDisabled = isTyping || isLoading;
  const isObsessionToggleDisabled = isChoiceInteractionDisabled || !hasChoices || obsessionPoints <= 0;
  const isObsessionSubmitDisabled = isChoiceInteractionDisabled || obsessionInput.trim().length === 0;
  const statusMessage = feedback ?? error;
  const broadcastItems = latestBroadcastItems
    .map((item) => item.trim())
    .filter(Boolean);
  const broadcastMessages = broadcastItems.length > 0
    ? broadcastItems
    : (latestBroadcastSummary.trim() ? [latestBroadcastSummary.trim()] : []);

  useEffect(() => {
    if (!feedback) return undefined;

    const timer = window.setTimeout(() => setFeedback(null), 2200);
    return () => window.clearTimeout(timer);
  }, [feedback]);

  useEffect(() => {
    setPreviews({});
    setObsessionInput('');
    setActiveObsession(false);
  }, [currentRound]);

  const handleTypewriterComplete = useCallback(() => {
    setCompletedTypingKey(typingKey);
  }, [typingKey]);

  const readErrorMessage = useCallback((cause: unknown, fallback: string) => {
    return cause instanceof Error ? cause.message : fallback;
  }, []);

  const handlePreview = async (choiceId: string, e: React.MouseEvent) => {
    e.stopPropagation();

    if (previews[choiceId]) return;

    try {
      const previewText = await previewChoice(choiceId);
      setPreviews((prev) => ({
        ...prev,
        [choiceId]: previewText,
      }));
      setFeedback('你窥见了一角尚未到来的命运。');
    } catch (previewError) {
      setFeedback(readErrorMessage(previewError, '直觉预览失败。'));
    }
  };

  const handleChoiceClick = async (choice: Choice) => {
    try {
      await submitChoice({
        input: {
          type: 'selected_option',
          action: choice.action,
        },
        displayText: choice.text,
      }, activeObsession);
      setActiveObsession(false);
      setObsessionInput('');
      setPreviews({});
      setFeedback(null);
    } catch (submitError) {
      setFeedback(readErrorMessage(submitError, '推进剧情失败。'));
    }
  };

  const handleObsessionSubmit = async (actionText: string) => {
    if (!actionText) {
      setFeedback('请先写下这次执念行动。');
      return;
    }

    try {
      await submitChoice({
        input: {
          type: 'free_text',
          action: actionText,
        },
        displayText: actionText,
      }, true);
      setActiveObsession(false);
      setObsessionInput('');
      setPreviews({});
      setFeedback(null);
    } catch (submitError) {
      setFeedback(readErrorMessage(submitError, '执念行动提交失败。'));
    }
  };

  const handleSave = async () => {
    try {
      await createSave();
      setFeedback('当前旅程已保存到本地演示存档。');
    } catch (saveError) {
      setFeedback(readErrorMessage(saveError, '存档失败。'));
    }
  };

  return (
    <ScreenShell className="items-stretch">
      <StoryFrame className="relative flex max-w-5xl flex-col overflow-hidden px-2.5 py-2.5 sm:px-3 sm:py-3 md:px-4 md:py-4">
        <div className="pointer-events-none absolute inset-0 bg-linear-to-b from-transparent via-[#08111d]/35 to-[#08111d]" />
        <div className="relative z-10 flex min-h-0 flex-1 flex-col gap-3">
          <GameplayHeader
            currentRound={currentRound}
            currentScene={currentScene}
            isLoading={isLoading}
            broadcastMessages={broadcastMessages}
          />

          <div className="flex min-h-0 flex-1 flex-col gap-3">
            <NarrationPanel
              narrationHistory={narrationHistory}
              currentRound={currentRound}
              isAwaitingNarration={Boolean(activeRoundState?.isAwaitingNarration)}
              onTypewriterComplete={handleTypewriterComplete}
            />

            <div className="flex flex-col absolute w-full bottom-0">
              <ChoicePanel
                hasChoices={hasChoices}
                choices={currentRoundChoices}
                previews={previews}
                activeObsession={activeObsession}
                obsessionInput={obsessionInput}
                isChoiceInteractionDisabled={isChoiceInteractionDisabled}
                isObsessionSubmitDisabled={isObsessionSubmitDisabled}
                onChoiceClick={handleChoiceClick}
                onPreview={handlePreview}
                onObsessionInputChange={setObsessionInput}
                onObsessionSubmit={handleObsessionSubmit}
              />

              <GameplayToolbar
                activeObsession={activeObsession}
                isObsessionToggleDisabled={isObsessionToggleDisabled}
                obsessionPoints={obsessionPoints}
                intuitionPoints={intuitionPoints}
                onToggleObsession={() => {
                  setActiveObsession((prev) => !prev);
                  setFeedback(null);
                }}
                onBackToLobby={() => setGameState('lobby')}
                onSave={handleSave}
                onShare={() => setFeedback('本地演示模式下可先存档，稍后可继续扩展分享入口。')}
              />
            </div>
            <div className="min-h-5">
              {statusMessage ? <p className="text-xs text-[#d9cbb1] sm:text-sm">{statusMessage}</p> : null}
            </div>

          </div>
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default GameplayPage;
