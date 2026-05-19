import React, { useCallback, useEffect, useState } from 'react';
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
import { useGameStore } from '../store/gameStore';
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
    worldNews,
    stateView,
    daysLeft,
    latestSaveId,
    isLoading,
    error,
    createSave,
    submitChoice,
    previewChoice,
    setGameState,
  } = useGameStore();
  const [isTyping, setIsTyping] = useState(true);
  const [activeObsession, setActiveObsession] = useState(false);
  const [previews, setPreviews] = useState<Record<string, string>>({});
  const [feedback, setFeedback] = useState<string | null>(null);
  const [isUtilityMenuOpen, setIsUtilityMenuOpen] = useState(false);

  const currentScene = stateView?.currentScene ?? '演示片段';
  const hasChoices = currentNode?.choices.length > 0;
  const showChoicePendingState = !hasChoices && (isTyping || isLoading);

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

  const handleTypewriterComplete = useCallback(() => {
    setIsTyping(false);
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
      setFeedback(previewError instanceof Error ? previewError.message : '直觉预览失败。');
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
      setFeedback(submitError instanceof Error ? submitError.message : '推进剧情失败。');
    }
  };

  const handleSave = async () => {
    try {
      await createSave();
      setFeedback('当前旅程已保存到本地演示存档。');
    } catch (saveError) {
      setFeedback(saveError instanceof Error ? saveError.message : '存档失败。');
    }
  };

  return (
    <ScreenShell className="items-stretch">
      <StoryFrame className="relative flex max-w-5xl flex-col overflow-hidden px-2.5 py-2.5 sm:px-3 sm:py-3 md:px-4 md:py-4">
        <div className="pointer-events-none absolute inset-0 bg-gradient-to-b from-transparent via-[#08111d]/35 to-[#08111d]" />
        <div className="relative z-10 flex min-h-0 flex-1 flex-col gap-3">
          <div className="shrink-0 space-y-2">
            <div className="flex flex-wrap gap-1">
              <StatusPill icon={Flame} className="px-2.5 py-1 text-[0.7rem] sm:text-xs">执念 {obsessionPoints}/5</StatusPill>
              <StatusPill icon={Eye} className="px-2.5 py-1 text-[0.7rem] sm:text-xs">直觉 {intuitionPoints}</StatusPill>
              <StatusPill icon={Clock3} className="px-2.5 py-1 text-[0.7rem] sm:text-xs">{daysLeft}日</StatusPill>
              <StatusPill icon={Sparkles} className="px-2.5 py-1 text-[0.7rem] sm:text-xs">{currentScene}</StatusPill>
            </div>
            <div className="akashic-pill w-fit border-amber-300/50 bg-[#1d1820]/95 px-2.5 py-1 text-[0.72rem] text-amber-100 sm:text-xs">
              <Sparkles className="h-3.5 w-3.5 text-amber-200" />
              <span>{worldNews ?? ""}</span>
            </div>
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
              {hasChoices && <div className="rounded-[1.1rem] border border-[rgba(116,103,80,0.35)] bg-[rgba(5,11,22,0.55)] px-1.5 py-1.5 sm:min-h-[10.5rem]">
                <div className="akashic-scroll max-h-[28dvh] space-y-1 overflow-y-auto pr-0.5 sm:max-h-[32dvh]">
                  {hasChoices ? currentNode.choices.map((choice) => (
                    <div key={choice.id} className="space-y-1.5">
                      <div className="grid grid-cols-[minmax(0,1fr)_2.5rem] items-center gap-1.5">
                        <button
                          onClick={() => void handleChoiceClick(choice.id)}
                          disabled={isTyping || isLoading || choice.disabled}
                          className={`akashic-choice h-10 disabled:cursor-not-allowed disabled:opacity-50 ${activeObsession ? 'border-red-400/45 bg-red-950/20 text-red-100' : 'text-[#f3ead8]'
                            }`}
                        >
                          <div className="flex min-h-[1.75rem] items-center text-left">
                            <div className="w-full text-sm font-semibold leading-5 sm:text-[0.95rem]">
                              {choice.text}
                            </div>
                          </div>
                        </button>

                        <button
                          type="button"
                          onClick={(e) => void handlePreview(choice.id, e)}
                          disabled={isTyping || isLoading}
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
                  <SecondaryButton
                    onClick={() => setActiveObsession((prev) => !prev)}
                    className={`min-h-0 gap-1.5 px-2.5 py-1.5 text-[0.72rem] leading-4 sm:text-xs ${activeObsession ? 'border-red-300/50 bg-red-950/25 text-red-100' : ''}`}
                    disabled={isTyping || isLoading || !hasChoices}
                  >
                    <Flame className={`h-3.5 w-3.5 ${activeObsession ? 'animate-pulse' : ''}`} />
                    执念
                  </SecondaryButton>
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
                            setFeedback(latestSaveId ? `最近存档：${latestSaveId}` : '本地演示模式下可先存档，稍后可继续扩展分享入口。');
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

            <div className="shrink-0 min-h-[1.25rem]">
              {feedback ? <p className="text-xs text-[#d9cbb1] sm:text-sm">{feedback}</p> : null}
              {error && !feedback ? <p className="text-xs text-[#d9cbb1] sm:text-sm">{error}</p> : null}
            </div>
          </div>
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default GameplayPage;
