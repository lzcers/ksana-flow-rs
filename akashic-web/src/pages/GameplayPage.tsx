import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  Clock3,
  Eye,
  Flame,
  House,
  Hourglass,
  MoreHorizontal,
  Save,
  Share2,
  Sparkles,
} from 'lucide-react';
import { useGameInternalStore, useGameUIStore } from '../store/gameStore';
import Typewriter from '../components/Typewriter';
import {
  ScreenShell,
  SecondaryButton,
  StatusPill,
  StoryFrame,
} from '../components/AkashicUI';
import { STREAM_PLACEHOLDER_TEXT } from '../store/gameStoreHelpers';

interface NarrationRoundEntry {
  round: number;
  narrationText: string;
  narrationStatus: 'pending' | 'running' | 'done' | 'error' | null;
  selectedChoiceText: string | null;
  isAwaitingNarration: boolean;
}

interface NarrationHistoryItemProps {
  entry: NarrationRoundEntry;
  isCurrentRound: boolean;
  isFinished: boolean;
  onComplete?: () => void;
}

const EMPTY_BROADCAST_ITEMS: string[] = [];

const NarrationHistoryItem: React.FC<NarrationHistoryItemProps> = React.memo(({
  entry,
  isCurrentRound,
  isFinished,
  onComplete,
}) => {
  return (
    <div className="space-y-2">
      <span className="text-xs font-medium tracking-[0.18em] text-[#8f98ab] uppercase">
        第 {entry.round} 轮
      </span>
      {entry.isAwaitingNarration && !entry.narrationText ? (
        <p className="text-sm font-medium text-[#8f98ab]">
          {STREAM_PLACEHOLDER_TEXT}
        </p>
      ) : (
        <Typewriter
          text={entry.narrationText}
          animate={isCurrentRound}
          isFinished={isFinished}
          onComplete={isCurrentRound ? onComplete : undefined}
        />
      )}
      {entry.selectedChoiceText ? (
        <p className="text-[0.82rem] font-medium leading-6 text-amber-100/90 sm:text-[0.92rem]">
          你的选择：{entry.selectedChoiceText}
        </p>
      ) : null}
    </div>
  );
});

NarrationHistoryItem.displayName = 'NarrationHistoryItem';

const GameplayPage: React.FC = () => {
  const obsessionPoints = useGameUIStore((state) => state.obsessionPoints);
  const intuitionPoints = useGameUIStore((state) => state.intuitionPoints);
  const turnIndex = useGameUIStore((state) => state.stateView?.turnIndex);
  const currentScene = useGameUIStore((state) => state.stateView?.currentScene ?? '');
  const latestBroadcastItems = useGameUIStore((state) => state.stateView?.latestBroadcastItems ?? EMPTY_BROADCAST_ITEMS);
  const latestBroadcastSummary = useGameUIStore((state) => state.stateView?.latestBroadcastSummary ?? '');
  const isLoading = useGameUIStore((state) => state.isLoading);
  const error = useGameUIStore((state) => state.error);
  const createSave = useGameUIStore((state) => state.createSave);
  const submitChoice = useGameUIStore((state) => state.submitChoice);
  const previewChoice = useGameUIStore((state) => state.previewChoice);
  const setGameState = useGameUIStore((state) => state.setGameState);
  const displayRound = useGameInternalStore((state) => state.displayRound);
  const roundStates = useGameInternalStore((state) => state.roundStates);
  const [isTyping, setIsTyping] = useState(true);
  const [activeObsession, setActiveObsession] = useState(false);
  const [previews, setPreviews] = useState<Record<string, string>>({});
  const [feedback, setFeedback] = useState<string | null>(null);
  const [isUtilityMenuOpen, setIsUtilityMenuOpen] = useState(false);
  const [broadcastIndex, setBroadcastIndex] = useState(0);
  const [isBroadcastDragging, setIsBroadcastDragging] = useState(false);
  const broadcastSwipeStartRef = useRef<{ pointerId: number; clientX: number } | null>(null);

  const currentRound = Math.max(displayRound || turnIndex || 1, 1);
  const narrationHistory = useMemo<NarrationRoundEntry[]>(() => (
    Object.values(roundStates)
      .filter((entry) => entry.narrationText || entry.selectedChoiceText || entry.isAwaitingNarration)
      .sort((left, right) => left.round - right.round)
  ), [roundStates]);
  const activeRoundState = roundStates[currentRound];
  const currentRoundChoices = activeRoundState?.choices ?? [];
  const hasChoices = currentRoundChoices.length > 0;
  const isChoiceInteractionDisabled = isTyping || isLoading;
  const isObsessionToggleDisabled = isChoiceInteractionDisabled || !hasChoices;
  const statusMessage = feedback ?? error;
  const shouldType = Boolean(activeRoundState?.narrationText) || Boolean(activeRoundState?.isAwaitingNarration);
  const isFatePlanningScene = isLoading && currentScene.includes('命运编织');
  const broadcastItems = latestBroadcastItems
    .map((item) => item.trim())
    .filter(Boolean);
  const broadcastMessages = broadcastItems.length > 0
    ? broadcastItems
    : (latestBroadcastSummary.trim() ? [latestBroadcastSummary.trim()] : []);
  const broadcastKey = broadcastMessages.join('||');
  const activeBroadcastMessage = broadcastMessages[broadcastIndex] ?? broadcastMessages[0] ?? '';
  const broadcastCountLabel = broadcastMessages.length > 0
    ? `${Math.min(broadcastIndex + 1, broadcastMessages.length)}/${broadcastMessages.length}`
    : '0/0';

  useEffect(() => {
    if (!feedback) return undefined;

    const timer = window.setTimeout(() => setFeedback(null), 2200);
    return () => window.clearTimeout(timer);
  }, [feedback]);

  useEffect(() => {
    setIsTyping(true);
    setPreviews({});
    setIsUtilityMenuOpen(false);
  }, [currentRound]);

  useEffect(() => {
    setBroadcastIndex((prev) => (prev === 0 ? prev : 0));
  }, [broadcastKey]);

  useEffect(() => {
    setIsTyping((prev) => (prev === shouldType ? prev : shouldType));
  }, [shouldType]);

  const handleTypewriterComplete = useCallback(() => {
    setIsTyping(false);
  }, []);

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

  const handleChoiceClick = async (choiceId: string) => {
    try {
      await submitChoice(choiceId, activeObsession);
      setIsTyping(true);
      setActiveObsession(false);
      setPreviews({});
      setFeedback(null);
    } catch (submitError) {
      setFeedback(readErrorMessage(submitError, '推进剧情失败。'));
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

  const moveBroadcastIndex = useCallback((direction: 'prev' | 'next') => {
    if (broadcastMessages.length <= 1) {
      return;
    }

    setBroadcastIndex((prev) => {
      if (direction === 'next') {
        return (prev + 1) % broadcastMessages.length;
      }
      return (prev - 1 + broadcastMessages.length) % broadcastMessages.length;
    });
  }, [broadcastMessages.length]);

  const releaseBroadcastPointer = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    broadcastSwipeStartRef.current = null;
    setIsBroadcastDragging(false);

    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  }, []);

  const handleBroadcastPointerDown = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    if (event.pointerType === 'mouse' && event.button !== 0) {
      return;
    }

    broadcastSwipeStartRef.current = {
      pointerId: event.pointerId,
      clientX: event.clientX,
    };
    setIsBroadcastDragging(true);
    event.currentTarget.setPointerCapture(event.pointerId);
  }, []);

  const handleBroadcastPointerEnd = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    const start = broadcastSwipeStartRef.current;
    if (!start || start.pointerId !== event.pointerId) {
      return;
    }

    const deltaX = event.clientX - start.clientX;
    releaseBroadcastPointer(event);

    if (Math.abs(deltaX) < 36) {
      return;
    }

    moveBroadcastIndex(deltaX < 0 ? 'next' : 'prev');
  }, [moveBroadcastIndex, releaseBroadcastPointer]);

  const handleBroadcastPointerCancel = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    const start = broadcastSwipeStartRef.current;
    if (!start || start.pointerId !== event.pointerId) {
      return;
    }

    releaseBroadcastPointer(event);
  }, [releaseBroadcastPointer]);

  return (
    <ScreenShell className="items-stretch">
      <StoryFrame className="relative flex max-w-5xl flex-col overflow-hidden px-2.5 py-2.5 sm:px-3 sm:py-3 md:px-4 md:py-4">
        <div className="pointer-events-none absolute inset-0 bg-linear-to-b from-transparent via-[#08111d]/35 to-[#08111d]" />
        <div className="relative z-10 flex min-h-0 flex-1 flex-col gap-3">
          <div className="shrink-0 space-y-2">
            <div className="flex flex-wrap gap-1 justify-between">
              <StatusPill icon={Clock3} className="px-2.5 py-1 text-[0.7rem] sm:text-xs">第 {currentRound} 轮</StatusPill>
              <StatusPill
                icon={isFatePlanningScene ? Hourglass : null}
                iconClassName={isFatePlanningScene ? 'h-3 w-3 animate-spin' : undefined}
                className="px-2.5 py-1 text-[0.7rem] sm:text-xs"
              >
                {currentScene}
              </StatusPill>
            </div>
            <div className="relative h-10 sm:h-11">
              {activeBroadcastMessage ? (
                <div
                  className={`akashic-pill absolute inset-y-0 left-0 flex w-full max-w-full items-start border-amber-300/50 bg-[#1d1820]/95 px-2.5 py-1 text-[0.72rem] text-amber-100 select-none touch-pan-y sm:text-xs ${isBroadcastDragging ? 'cursor-grabbing' : 'cursor-grab'}`}
                  onPointerDown={handleBroadcastPointerDown}
                  onPointerUp={handleBroadcastPointerEnd}
                  onPointerCancel={handleBroadcastPointerCancel}
                >
                  <Sparkles className="mt-0.5 h-3.5 w-3.5 shrink-0 text-amber-200" />
                  <span className="line-clamp-2 min-w-0 flex-1 leading-4">{activeBroadcastMessage}</span>
                  <span className="shrink-0 rounded-full border border-amber-300/25 bg-black/15 px-1.5 py-0.5 text-[0.65rem] leading-none text-amber-100/80 sm:text-[0.7rem]">
                    {broadcastCountLabel}
                  </span>
                </div>
              ) : null}
            </div>
          </div>

          <div className="flex min-h-0 flex-1 flex-col gap-3">
            <section className="akashic-panel flex h-[55dvh] shrink-0 flex-col p-2">
              <div className="flex min-h-0 flex-1 flex-col rounded-2xl bg-[#040912]/90 sm:rounded-[1.2rem] sm:pl-4 md:rounded-[1.3rem] md:pl-5">
                <div className="akashic-scroll min-h-0 flex-1 overflow-y-auto">
                  <div className="h-full space-y-5 py-1 pr-2 text-[1rem] font-semibold leading-[1.82] text-[#f6eddc] sm:text-[1rem] md:text-[1.2rem]">
                    {narrationHistory.map((entry) => {
                      return (
                        <NarrationHistoryItem
                          key={entry.round}
                          entry={entry}
                          isCurrentRound={entry.round === currentRound}
                          isFinished={entry.round !== currentRound || entry.narrationStatus === 'done'}
                          onComplete={handleTypewriterComplete}
                        />
                      );
                    })}
                    {!narrationHistory.length && activeRoundState?.isAwaitingNarration ? (
                      <p className="text-sm font-medium text-[#8f98ab]">
                        {STREAM_PLACEHOLDER_TEXT}
                      </p>
                    ) : null}
                  </div>
                </div>
              </div>
            </section>

            <div className="flex flex-col absolute w-full bottom-0">
              <div className="flex w-full">
                {hasChoices &&
                  <div className="game-choices flex-1 rounded-[1.1rem] border border-[rgba(116,103,80,0.35)] bg-[rgba(5,11,22,0.55)] px-1.5 py-2">
                    <div className="akashic-scroll max-h-[28dvh] space-y-1 overflow-y-auto pr-0.5 py-0.5">
                      {currentRoundChoices.map((choice) => (
                        <div key={choice.id} className="space-y-1.5">
                          <div className="grid grid-cols-[minmax(0,1fr)_2.5rem] items-center gap-1.5">
                            <button
                              onClick={() => void handleChoiceClick(choice.id)}
                              disabled={isChoiceInteractionDisabled || choice.disabled}
                              className={`akashic-choice h-10 disabled:cursor-not-allowed disabled:opacity-50 ${activeObsession ? 'border-red-400/45 bg-red-950/20 text-red-100' : 'text-[#f3ead8]'
                                }`}
                            >
                              <div className="flex min-h-7 items-center text-left">
                                <div className="w-full text-sm font-semibold leading-5 sm:text-[0.95rem]">
                                  {choice.text}
                                </div>
                              </div>
                            </button>

                            <button
                              type="button"
                              onClick={(e) => void handlePreview(choice.id, e)}
                              disabled={isChoiceInteractionDisabled}
                              className="akashic-icon-btn h-10 min-h-10 w-10 self-auto disabled:cursor-not-allowed disabled:opacity-50"
                              title="消耗 1 点直觉，窥探命运碎片"
                            >
                              <Eye className="h-4 w-4" />
                            </button>
                          </div>

                          {previews[choice.id] ? (
                            <div className="rounded-[0.8rem] border border-cyan-400/20 bg-cyan-950/10 px-2 py-2 text-[0.7rem] leading-4.5 text-cyan-100/90 sm:rounded-[0.95rem] sm:px-2 sm:py-2 sm:text-xs">
                              {previews[choice.id]}
                            </div>
                          ) : null}
                        </div>
                      ))}
                    </div>
                  </div>
                }
              </div>

              <div className="game-opts inset-x-0 rounded-full border border-[rgba(116,103,80,0.4)] bg-[rgba(8,14,26,0.82)] px-2 py-2 backdrop-blur-md">
                <div className="relative flex items-center justify-between gap-2">
                  <div className="flex items-center gap-2">
                    <SecondaryButton
                      onClick={() => setActiveObsession((prev) => !prev)}
                      className={`min-h-0 gap-1.5 px-2.5 py-1.5 text-[0.72rem] leading-4 sm:text-xs ${activeObsession ? 'border-red-300/50 bg-red-950/25 text-red-100' : ''}`}
                      disabled={isObsessionToggleDisabled}
                    >
                      <Flame className={`h-3.5 w-3.5 ${activeObsession ? 'animate-pulse' : ''}`} />
                      执念
                    </SecondaryButton>
                  </div>
                  <div className='flex items-center gap-2'>
                    <span className="inline-flex items-center gap-1 text-[0.72rem] leading-4 text-[#d9cbb1] sm:text-xs">
                      <Flame className="h-3.5 w-3.5" />
                      <span>{`${obsessionPoints}/5`}</span>
                    </span>
                    <span className="text-[0.72rem] leading-4 text-[#8f98ab] sm:text-xs">|</span>
                    <span className="inline-flex items-center gap-1 text-[0.72rem] leading-4 text-[#d9cbb1] sm:text-xs">
                      <Eye className="h-3.5 w-3.5" />
                      <span>{intuitionPoints}</span>
                    </span>
                  </div>
                  <div className="relative">
                    <SecondaryButton
                      type="button"
                      onClick={() => setIsUtilityMenuOpen((prev) => !prev)}
                      className="min-h-0 gap-1.5 px-2.5 py-1.5 text-[0.72rem] leading-4 sm:text-xs"
                    >
                      <MoreHorizontal className="h-3.5 w-3.5" />
                      菜单
                    </SecondaryButton>
                    {isUtilityMenuOpen ? (
                      <div className="absolute bottom-[calc(100%+0.45rem)] right-0 z-20 min-w-[8.8rem] rounded-[0.95rem] border border-[rgba(116,103,80,0.5)] bg-[rgba(7,13,24,0.96)] p-1.5 shadow-[0_10px_24px_rgba(0,0,0,0.45)]">
                        <button
                          type="button"
                          onClick={() => {
                            setGameState('lobby');
                            setIsUtilityMenuOpen(false);
                          }}
                          className="flex w-full items-center gap-1.5 rounded-[0.7rem] px-2 py-1.5 text-left text-[0.72rem] leading-4 text-[#f3ead8] transition-colors hover:bg-[rgba(188,169,124,0.14)] sm:text-xs"
                        >
                          <House className="h-3.5 w-3.5" />
                          返回大厅
                        </button>
                        <button
                          type="button"
                          onClick={() => {
                            void handleSave();
                            setIsUtilityMenuOpen(false);
                          }}
                          className="flex w-full items-center gap-1.5 rounded-[0.7rem] px-2 py-1.5 text-left text-[0.72rem] leading-4 text-[#f3ead8] transition-colors hover:bg-[rgba(188,169,124,0.14)] sm:text-xs"
                        >
                          <Save className="h-3.5 w-3.5" />
                          存档
                        </button>
                        <button
                          type="button"
                          onClick={() => {
                            setFeedback('本地演示模式下可先存档，稍后可继续扩展分享入口。');
                            setIsUtilityMenuOpen(false);
                          }}
                          className="flex w-full items-center gap-1.5 rounded-[0.7rem] px-2 py-1.5 text-left text-[0.72rem] leading-4 text-[#f3ead8] transition-colors hover:bg-[rgba(188,169,124,0.14)] sm:text-xs"
                        >
                          <Share2 className="h-3.5 w-3.5" />
                          分享
                        </button>
                      </div>
                    ) : null}
                  </div>
                </div>
              </div>
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
