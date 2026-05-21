import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  Clock3,
  Eye,
  Flame,
  House,
  MoreHorizontal,
  Save,
  Share2,
  Sparkles,
} from 'lucide-react';
import { useGameInternalStore, useGameUIStore } from '../store/gameStore';
import Typewriter from '../components/Typewriter';
import {
  PrimaryButton,
  ScreenShell,
  SecondaryButton,
  StatusPill,
  StoryFrame,
} from '../components/AkashicUI';

const GameplayPage: React.FC = () => {
  const {
    currentNode,
    obsessionPoints,
    intuitionPoints,
    stateView,
    isLoading,
    error,
    createSave,
    submitChoice,
    previewChoice,
    setGameState,
  } = useGameUIStore();
  const {
    streamedNarrationText,
    streamedNarrationStatus,
    streamedFatePlanningRaw,
    streamedFatePlanningJson,
    streamedProtagonistActionRaw,
    streamedProtagonistActionJson,
  } = useGameInternalStore();
  const [isTyping, setIsTyping] = useState(true);
  const [activeObsession, setActiveObsession] = useState(false);
  const [previews, setPreviews] = useState<Record<string, string>>({});
  const [feedback, setFeedback] = useState<string | null>(null);
  const [isUtilityMenuOpen, setIsUtilityMenuOpen] = useState(false);
  const lastNarrationLogRef = useRef('');
  const lastFatePlanningLogRef = useRef('');
  const lastProtagonistActionLogRef = useRef('');

  const currentRound = Math.max(stateView?.turnIndex ?? 1, 1);
  const hasChoices = currentNode?.choices.length > 0;
  const isChoiceInteractionDisabled = isTyping || isLoading;
  const isObsessionToggleDisabled = isChoiceInteractionDisabled || !hasChoices;
  const statusMessage = feedback ?? error;

  useEffect(() => {
    if (!feedback) return undefined;

    const timer = window.setTimeout(() => setFeedback(null), 2200);
    return () => window.clearTimeout(timer);
  }, [feedback]);

  useEffect(() => {
    setIsTyping(true);
    setPreviews({});
    setIsUtilityMenuOpen(false);
  }, [currentNode?.id]);

  useEffect(() => {
    setIsTyping(Boolean(currentNode?.text));
  }, [currentNode?.text]);

  useEffect(() => {
    const nextNarration = streamedNarrationText.trim();

    if (
      !nextNarration
      || streamedNarrationStatus !== 'done'
      || nextNarration === lastNarrationLogRef.current
    ) {
      return;
    }

    lastNarrationLogRef.current = nextNarration;
    console.groupCollapsed('[Akashic Stream Debug:narration]');
    console.log(nextNarration);
    console.groupEnd();
  }, [streamedNarrationStatus, streamedNarrationText]);

  useEffect(() => {
    if (!streamedFatePlanningJson) {
      return;
    }

    const serialized = JSON.stringify(streamedFatePlanningJson);
    if (serialized === lastFatePlanningLogRef.current) {
      return;
    }

    lastFatePlanningLogRef.current = serialized;
    console.groupCollapsed('[Akashic Stream Debug:fate_planning]');
    console.log('raw', streamedFatePlanningRaw || '(暂无 fate_planning 流)');
    console.log('parsed', streamedFatePlanningJson);
    console.groupEnd();
  }, [
    streamedFatePlanningRaw,
    streamedFatePlanningJson,
  ]);

  useEffect(() => {
    if (!streamedProtagonistActionJson) {
      return;
    }

    const serialized = JSON.stringify(streamedProtagonistActionJson);
    if (serialized === lastProtagonistActionLogRef.current) {
      return;
    }

    lastProtagonistActionLogRef.current = serialized;
    console.groupCollapsed('[Akashic Stream Debug:protagonist_action]');
    console.log('raw', streamedProtagonistActionRaw || '(暂无 protagonist_action 流)');
    console.log('parsed', streamedProtagonistActionJson);
    console.groupEnd();
  }, [
    streamedProtagonistActionRaw,
    streamedProtagonistActionJson,
  ]);

  const handleTypewriterComplete = useCallback(() => {
    setIsTyping(false);
  }, []);

  const readErrorMessage = useCallback((cause: unknown, fallback: string) => {
    return cause instanceof Error ? cause.message : fallback;
  }, []);

  if (!currentNode) {
    return (
      <ScreenShell className="items-stretch">
        <StoryFrame className="relative max-w-4xl overflow-hidden px-4 py-8 sm:px-5 sm:py-10 md:px-6 md:py-12">
          <div className="space-y-4 text-center">
            <p className="text-lg font-semibold text-[#f6eddc]">
              还没有可展示的演示剧情
            </p>
            <p className="text-sm leading-6 text-[#9ca7be]">
              {error ?? '先在创建页生成一段本地演示旅程，然后再回来体验页面跳转与选择交互。'}
            </p>
            <div className="flex justify-center">
              <PrimaryButton onClick={() => setGameState('creation')}>
                进入创建页
              </PrimaryButton>
            </div>
          </div>
        </StoryFrame>
      </ScreenShell>
    );
  }

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

  return (
    <ScreenShell className="items-stretch">
      <StoryFrame className="relative flex max-w-5xl flex-col overflow-hidden px-2.5 py-2.5 sm:px-3 sm:py-3 md:px-4 md:py-4">
        <div className="pointer-events-none absolute inset-0 bg-linear-to-b from-transparent via-[#08111d]/35 to-[#08111d]" />
        <div className="relative z-10 flex min-h-0 flex-1 flex-col gap-3">
          <div className="shrink-0 space-y-2">
            <div className="flex flex-wrap gap-1 justify-between">
              <StatusPill icon={Clock3} className="px-2.5 py-1 text-[0.7rem] sm:text-xs">第 {currentRound} 轮</StatusPill>
              <StatusPill icon={null} className="px-2.5 py-1 text-[0.7rem] sm:text-xs">
                {stateView?.currentScene ?? '命运推进'}
              </StatusPill>
            </div>
            {stateView?.latestBroadcastSummary && <div className="akashic-pill w-fit border-amber-300/50 bg-[#1d1820]/95 px-2.5 py-1 text-[0.72rem] text-amber-100 sm:text-xs">
              <Sparkles className="h-3.5 w-3.5 shrink-0 text-amber-200" />
              <span>{stateView.latestBroadcastSummary}</span>
            </div>}
          </div>

          <div className="flex min-h-0 flex-1 flex-col gap-3">
            <section className="akashic-panel flex min-h-0 flex-1 flex-col p-2">
              <div className="flex min-h-0 flex-1 flex-col rounded-2xl bg-[#040912]/90 sm:rounded-[1.2rem] sm:pl-4 md:rounded-[1.3rem] md:pl-5">
                <div className="akashic-scroll min-h-0 flex-1 overflow-y-auto">
                  <div className="text-[1rem] font-semibold leading-[1.82] text-[#f6eddc] sm:text-[1rem] md:text-[1.2rem]">
                    <Typewriter text={currentNode.text} speed={28} onComplete={handleTypewriterComplete} />
                  </div>
                </div>
              </div>
            </section>

            <div className="shrink-0 space-y-1.5">
              {hasChoices && <div className="rounded-[1.1rem] border border-[rgba(116,103,80,0.35)] bg-[rgba(5,11,22,0.55)] px-1.5 py-2">
                <div className="akashic-scroll max-h-[28dvh] space-y-1 overflow-y-auto pr-0.5 py-0.5">
                  {hasChoices ? currentNode.choices.map((choice) => (
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
                  )) : null}
                </div>
              </div>}
              <div className="shrink-0 rounded-full border border-[rgba(116,103,80,0.4)] bg-[rgba(8,14,26,0.82)] px-2 py-2 backdrop-blur-md">
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

            <div className="shrink-0 min-h-5">
              {statusMessage ? <p className="text-xs text-[#d9cbb1] sm:text-sm">{statusMessage}</p> : null}
            </div>
          </div>
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default GameplayPage;
