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
import { appRoutes } from '../lib/appRoutes';
import type { Choice } from '../lib/api';

const EMPTY_BROADCAST_ITEMS: string[] = [];

function formatEndingTypeLabel(endingType: string | null): string {
  const normalized = endingType?.trim();
  if (!normalized) {
    return '终局已定';
  }

  return normalized
    .split(/[_\s-]+/)
    .filter(Boolean)
    .map((segment) => segment[0]?.toUpperCase() + segment.slice(1))
    .join(' ');
}

function endingDescription(endingType: string | null, isLoading: boolean): string {
  if (isLoading) {
    return '回响已抵达落幕，这一段终章仍在你的眼前徐徐显现。';
  }

  switch (endingType) {
    case 'triumph':
      return '旅程在抵达所愿之处后落幕，余下的是一段仍会回响的胜利。';
    case 'tragedy':
      return '故事走到了无法挽回的尽头，余音里只剩代价与沉默。';
    case 'bittersweet':
      return '你得到了某些答案，也永远失去了一部分自己。';
    case 'open':
      return '这一段旅程已告一段落，世界仍在更远处轻轻翻涌。';
    default:
      return '这段旅程已经抵达自然的终点，你可以停留片刻，读完最后的余韵。';
  }
}

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
    isEnding,
    endingType,
  } = useGameUIStore(useShallow((state) => ({
    phase: state.stateView?.phase ?? '',
    turnIndex: state.stateView?.turnIndex,
    currentScene: state.stateView?.currentScene ?? '',
    latestBroadcastItems: state.stateView?.latestBroadcastItems ?? EMPTY_BROADCAST_ITEMS,
    latestBroadcastSummary: state.stateView?.latestBroadcastSummary ?? '',
    isLoading: state.isLoading,
    error: state.error,
    skipRestoredNarrationAnimation: state.skipRestoredNarrationAnimation,
    isEnding: state.stateView?.isEnding ?? false,
    endingType: state.stateView?.endingType ?? null,
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
  const hasChoices = currentRoundChoices.length > 0 && !isEnding;
  const isNarrationStreaming = activeRoundState?.narrationStatus === 'pending'
    || activeRoundState?.narrationStatus === 'running';
  const shouldType = Boolean(activeRoundState?.isAwaitingNarration) || isNarrationStreaming;
  const typingKey = `${currentRound}:${activeRoundState?.isAwaitingNarration ? '1' : '0'}:${activeRoundState?.narrationText ?? ''}`;
  const isTyping = shouldType && completedTypingKey !== typingKey;
  const isChoiceInteractionDisabled = isTyping || isLoading || isEnding;
  const isObsessionToggleDisabled = isChoiceInteractionDisabled || !hasChoices || obsessionPoints <= 0;
  const isObsessionSubmitDisabled = isChoiceInteractionDisabled || obsessionInput.trim().length === 0;
  const statusMessage = feedback ?? error;
  const broadcastItems = latestBroadcastItems
    .map((item) => item.trim())
    .filter(Boolean);
  const broadcastMessages = broadcastItems.length > 0
    ? broadcastItems
    : (latestBroadcastSummary.trim() ? [latestBroadcastSummary.trim()] : []);
  const endingTitle = formatEndingTypeLabel(endingType);
  const endingBody = endingDescription(endingType, isTyping || isNarrationStreaming || isLoading);

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

      setPreviews((prev) => ({
        ...prev,
        [choice.id]: motivationAndRisk,
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
              {isEnding ? (
                <section className="rounded-[1.1rem] border border-amber-300/35 bg-[rgba(40,25,16,0.6)] px-4 py-3 text-[#f6eddc] shadow-[0_10px_30px_rgba(0,0,0,0.18)]">
                  <div className="flex items-center justify-between gap-3">
                    <p className="text-sm font-semibold tracking-[0.18em] text-amber-200/90">
                      故事已落幕
                    </p>
                    <span className="rounded-full border border-amber-200/20 bg-black/15 px-2.5 py-1 text-[0.7rem] uppercase tracking-[0.18em] text-amber-100/85">
                      {endingTitle}
                    </span>
                  </div>
                  <p className="mt-2 text-sm leading-6 text-[#e7d9c2]">
                    {endingBody}
                  </p>
                </section>
              ) : null}
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
                onBackToLobby={() => {
                  resetGame();
                  navigate(appRoutes.lobby);
                }}
                onSave={handleSave}
                onShare={() => setFeedback('分享功能即将开放，当前可先导出存档保存这段旅程。')}
              />
            </div>
          </div>
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default GameplayPage;
