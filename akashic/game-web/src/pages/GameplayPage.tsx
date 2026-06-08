import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { useNavigate } from 'react-router-dom';
import { useGameInternalStore } from '../store/gameStore';
import { useGameUIStore } from '../store/gameUIStore';
import { useGameValueStore } from '../store/gameValueStore';
import {
  ScreenShell,
  StoryFrame,
} from '../components/AkashicUI';
import GameplayHeader from '../components/GameplayHeader';
import ChoicePanel from '../components/ChoicePanel';
import GameplayToolbar from '../components/GameplayToolbar';
import NarrationPanel from '../components/NarrationPanel';
import type { NarrationRoundEntry } from '../components/gameplayTypes';
import { appRoutes, routeWithClonedSession } from '../lib/appRoutes';
import type { Choice } from '../lib/api';

const EMPTY_BROADCAST_ITEMS: string[] = [];

const GameplayPage: React.FC = () => {
  const navigate = useNavigate();
  const {
    phase,
    turnIndex,
    currentScene,
    latestBroadcastItems,
    latestBroadcastSummary,
    isLoading,
    error,
    skipRestoredNarrationAnimation,
  } = useGameUIStore(useShallow((state) => ({
    phase: state.stateView?.phase ?? '',
    turnIndex: state.stateView?.turnIndex,
    currentScene: state.stateView?.currentScene ?? '',
    latestBroadcastItems: state.stateView?.latestBroadcastItems ?? EMPTY_BROADCAST_ITEMS,
    latestBroadcastSummary: state.stateView?.latestBroadcastSummary ?? '',
    isLoading: state.isLoading,
    error: state.error,
    skipRestoredNarrationAnimation: state.skipRestoredNarrationAnimation,
  })));
  const {
    bootstrapSession,
    createSave,
    submitChoice,
    resetGame,
  } = useGameUIStore(useShallow((state) => ({
    bootstrapSession: state.bootstrapSession,
    createSave: state.createSave,
    submitChoice: state.submitChoice,
    resetGame: state.resetGame,
  })));
  const {
    obsessionPoints,
    intuitionPoints,
    consumeIntuition,
  } = useGameValueStore(useShallow((state) => ({
    obsessionPoints: state.obsessionPoints,
    intuitionPoints: state.intuitionPoints,
    consumeIntuition: state.consumeIntuition,
  })));
  const sessionId = useGameInternalStore((state) => state.sessionId);
  const displayRound = useGameInternalStore((state) => state.displayRound);
  const roundStates = useGameInternalStore((state) => state.roundStates);
  const [completedTypingKey, setCompletedTypingKey] = useState<string | null>(null);
  const [roundControls, setRoundControls] = useState<{
    round: number;
    activeObsession: boolean;
    obsessionInput: string;
    previews: Record<string, string>;
  }>({
    round: 1,
    activeObsession: false,
    obsessionInput: '',
    previews: {},
  });
  const [feedback, setFeedback] = useState<string | null>(null);

  const currentRound = Math.max(displayRound || turnIndex || 1, 1);
  const narrationHistory = useMemo<NarrationRoundEntry[]>(() => (
    Object.values(roundStates)
      .filter((entry) => entry.narrationText || entry.selectedChoiceText || entry.isAwaitingNarration)
      .sort((left, right) => left.round - right.round)
  ), [roundStates]);
  const activeRoundState = roundStates[currentRound];
  const hasCurrentRoundControls = roundControls.round === currentRound;
  const activeObsession = hasCurrentRoundControls ? roundControls.activeObsession : false;
  const obsessionInput = hasCurrentRoundControls ? roundControls.obsessionInput : '';
  const previews = hasCurrentRoundControls ? roundControls.previews : {};
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
  const canArchiveCurrentRound = phase === 'awaiting_player'
    && hasChoices
    && !isNarrationStreaming
    && !isTyping
    && !isLoading;
  const archiveActionUnavailableReason = canArchiveCurrentRound
    ? null
    : '选项出现后可分享或存档。';
  const archiveActionKey = `${sessionId ?? 'no-session'}:${currentRound}:${currentRoundChoices.map((choice) => choice.id).join(',')}`;
  const statusMessage = feedback ?? error;
  const broadcastItems = latestBroadcastItems
    .map((item) => item.trim())
    .filter(Boolean);
  const broadcastMessages = broadcastItems.length > 0
    ? broadcastItems
    : (latestBroadcastSummary.trim() ? [latestBroadcastSummary.trim()] : []);
  const shareSummaryFallback = useMemo(() => {
    const latestNarration = [...narrationHistory]
      .reverse()
      .find((entry) => entry.narrationText.trim())
      ?.narrationText
      .trim();
    const broadcastSummary = latestBroadcastSummary.trim();

    if (latestNarration && broadcastSummary && !latestNarration.includes(broadcastSummary)) {
      return `${broadcastSummary} ${latestNarration}`;
    }

    if (latestNarration) {
      return latestNarration;
    }

    if (broadcastSummary) {
      return broadcastSummary;
    }

    return `${currentScene} 的命运仍在推进，下一轮选择正在逼近。`;
  }, [currentScene, latestBroadcastSummary, narrationHistory]);
  const shareGameUrl = useMemo(() => (
    new URL(
      sessionId ? routeWithClonedSession(appRoutes.gameplay, sessionId) : appRoutes.lobby,
      window.location.origin,
    ).toString()
  ), [sessionId]);

  useEffect(() => {
    if (!feedback) return undefined;

    const timer = window.setTimeout(() => setFeedback(null), 2200);
    return () => window.clearTimeout(timer);
  }, [feedback]);

  const handleTypewriterComplete = useCallback(() => {
    setCompletedTypingKey(typingKey);
  }, [typingKey]);

  const readErrorMessage = useCallback((cause: unknown, fallback: string) => {
    return cause instanceof Error ? cause.message : fallback;
  }, []);

  useEffect(() => {
    if (!sessionId || phase !== 'booting') {
      return;
    }

    void bootstrapSession().catch((bootstrapError) => {
      setFeedback(readErrorMessage(bootstrapError, '开场叙事启动失败。'));
    });
  }, [bootstrapSession, phase, readErrorMessage, sessionId]);

  const handlePreview = async (choice: Choice, e: React.MouseEvent) => {
    e.stopPropagation();

    if (previews[choice.id]) {
      setFeedback(previews[choice.id]);
      return;
    }

    try {
      const motivationAndRisk = choice.motivationAndRisk?.trim();
      if (!motivationAndRisk) {
        throw new Error('这个选项暂时没有可窥见的命运碎片。');
      }
      consumeIntuition();

      setRoundControls((prev) => ({
        round: currentRound,
        activeObsession: prev.round === currentRound ? prev.activeObsession : false,
        obsessionInput: prev.round === currentRound ? prev.obsessionInput : '',
        previews: {
          ...(prev.round === currentRound ? prev.previews : {}),
          [choice.id]: motivationAndRisk,
        },
      }));
      setFeedback(motivationAndRisk);
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
      setRoundControls({
        round: currentRound,
        activeObsession: false,
        obsessionInput: '',
        previews: {},
      });
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
      setRoundControls({
        round: currentRound,
        activeObsession: false,
        obsessionInput: '',
        previews: {},
      });
      setFeedback(null);
    } catch (submitError) {
      setFeedback(readErrorMessage(submitError, '执念行动提交失败。'));
    }
  };

  const handleSave = async () => {
    if (!canArchiveCurrentRound) {
      setFeedback(archiveActionUnavailableReason);
      return;
    }

    try {
      await createSave();
      setFeedback('存档保存成功');
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
              skipRestoredNarrationAnimation={skipRestoredNarrationAnimation}
              onTypewriterComplete={handleTypewriterComplete}
            />
            <div className="min-h-5">
              {statusMessage ? <p className="text-xs text-[#d9cbb1] sm:text-sm">{statusMessage}</p> : null}
            </div>
            <div className="mt-auto flex touch-pan-y flex-col gap-2">
              <ChoicePanel
                hasChoices={hasChoices}
                choices={currentRoundChoices}
                previews={previews}
                remainingIntuitionPoints={intuitionPoints}
                activeObsession={activeObsession}
                obsessionInput={obsessionInput}
                isChoiceInteractionDisabled={isChoiceInteractionDisabled}
                isObsessionSubmitDisabled={isObsessionSubmitDisabled}
                onChoiceClick={handleChoiceClick}
                onPreview={handlePreview}
                onObsessionInputChange={(nextValue) => {
                  setRoundControls((prev) => ({
                    round: currentRound,
                    activeObsession: prev.round === currentRound ? prev.activeObsession : false,
                    obsessionInput: nextValue,
                    previews: prev.round === currentRound ? prev.previews : {},
                  }));
                }}
                onObsessionSubmit={handleObsessionSubmit}
              />
              <GameplayToolbar
                activeObsession={activeObsession}
                isObsessionToggleDisabled={isObsessionToggleDisabled}
                obsessionPoints={obsessionPoints}
                intuitionPoints={intuitionPoints}
                sessionId={sessionId}
                shareSummaryFallback={shareSummaryFallback}
                shareGameUrl={shareGameUrl}
                archiveActionKey={archiveActionKey}
                isArchiveActionDisabled={!canArchiveCurrentRound}
                archiveActionUnavailableReason={archiveActionUnavailableReason}
                onToggleObsession={() => {
                  setRoundControls((prev) => ({
                    round: currentRound,
                    activeObsession: prev.round === currentRound ? !prev.activeObsession : true,
                    obsessionInput: prev.round === currentRound ? prev.obsessionInput : '',
                    previews: prev.round === currentRound ? prev.previews : {},
                  }));
                  setFeedback(null);
                }}
                onBackToLobby={() => {
                  resetGame();
                  navigate(appRoutes.lobby);
                }}
                onSave={handleSave}
              />
            </div>
          </div>
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default GameplayPage;
