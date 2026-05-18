import React, { useEffect, useState } from 'react';
import {
  Clock3,
  Eye,
  Flame,
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
    submitChoice,
    obsessionPoints,
    intuitionPoints,
    worldNews,
    previewChoice,
    stateView,
    daysLeft,
    createSave,
    latestSaveId,
    isLoading,
    error,
    setGameState,
  } = useGameStore();
  const [isTyping, setIsTyping] = useState(true);
  const [activeObsession, setActiveObsession] = useState(false);
  const [previews, setPreviews] = useState<Record<string, string>>({});
  const [feedback, setFeedback] = useState<string | null>(null);

  const currentScene = stateView?.currentScene ?? '演示片段';
  const isResolved = currentNode?.choices.length === 0;

  useEffect(() => {
    if (!feedback) return undefined;

    const timer = window.setTimeout(() => setFeedback(null), 2200);
    return () => window.clearTimeout(timer);
  }, [feedback]);

  useEffect(() => {
    setIsTyping(true);
    setPreviews({});
  }, [currentNode?.id]);

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
      <StoryFrame className="relative max-w-5xl overflow-hidden px-2.5 py-2.5 sm:px-3 sm:py-3 md:px-4 md:py-4">
        <div className="pointer-events-none absolute inset-0 bg-gradient-to-b from-transparent via-[#08111d]/35 to-[#08111d]" />
        <div className="relative z-10 space-y-3.5">
          <div className="flex flex-wrap gap-2">
            <StatusPill icon={Flame} className="px-2.5 py-1 text-[0.7rem] sm:text-xs">执念 {obsessionPoints}/5</StatusPill>
            <StatusPill icon={Eye} className="px-2.5 py-1 text-[0.7rem] sm:text-xs">直觉 {intuitionPoints}</StatusPill>
            <StatusPill icon={Clock3} className="px-2.5 py-1 text-[0.7rem] sm:text-xs">{daysLeft}日</StatusPill>
            <StatusPill icon={Sparkles} className="px-2.5 py-1 text-[0.7rem] sm:text-xs">{currentScene}</StatusPill>
          </div>
          {worldNews ? (
            <div className="akashic-pill w-fit border-amber-300/50 bg-[#1d1820]/95 px-2.5 py-1 text-[0.72rem] text-amber-100 sm:text-xs">
              <Sparkles className="h-3.5 w-3.5 text-amber-200" />
              <span>{worldNews}</span>
            </div>
          ) : null}
          <section className="akashic-panel px-3 py-3 sm:px-4 sm:py-4 md:px-5 md:py-5">
            <div className="rounded-[1rem] bg-[#040912]/90 pl-3 sm:rounded-[1.2rem] sm:pl-4 md:rounded-[1.3rem] md:pl-5">
              <div className="py-3 pr-3 sm:py-4 sm:pr-4 md:py-5 md:pr-5">
                <div className="min-h-[7rem] whitespace-pre-wrap text-[1rem] font-semibold leading-[1.82] text-[#f6eddc] sm:min-h-[8.5rem] sm:text-[1.2rem] md:min-h-[10rem] md:text-[1.7rem]">
                  <Typewriter text={currentNode.text} speed={28} onComplete={() => setIsTyping(false)} />
                </div>
              </div>
            </div>
          </section>

          <div className="space-y-1.5">
            {currentNode.choices.length ? currentNode.choices.map((choice) => (
              <div key={choice.id} className="space-y-1">
                <div className="flex gap-2">
                  <button
                    onClick={() => void handleChoiceClick(choice.id)}
                    disabled={isTyping || isLoading || choice.disabled}
                    className={`akashic-choice h-[3rem] flex-1 px-3 py-1.5 disabled:cursor-not-allowed disabled:opacity-50 ${activeObsession ? 'border-red-400/45 bg-red-950/20 text-red-100' : 'text-[#f3ead8]'
                      }`}
                  >
                    <div className="flex h-full items-center text-left">
                      <div className="w-full truncate text-sm font-semibold leading-5 sm:text-[0.95rem]">
                        {choice.text}
                      </div>
                    </div>
                  </button>

                  <button
                    type="button"
                    onClick={(e) => void handlePreview(choice.id, e)}
                    disabled={isTyping || isLoading}
                    className="akashic-icon-btn h-[3rem] w-[3rem] shrink-0 disabled:cursor-not-allowed disabled:opacity-50"
                    title="消耗 1 点直觉，窥探命运碎片"
                  >
                    <Eye className="h-4 w-4" />
                  </button>
                </div>

                {previews[choice.id] ? (
                  <div className="rounded-[0.85rem] border border-cyan-400/20 bg-cyan-950/10 px-3 py-1.5 text-[0.72rem] leading-5 text-cyan-100/90 sm:rounded-[1rem] sm:px-4 sm:text-xs">
                    {previews[choice.id]}
                  </div>
                ) : null}
              </div>
            )) : (
              <div className="akashic-panel px-3 py-3 text-sm leading-6 text-[#c3cde0]/85 sm:px-4">
                当前旅程已经收束，没有更多分支可选。你可以返回大厅，开启下一段新人生。
              </div>
            )}
          </div>

          <div className="flex flex-wrap gap-1.5 pt-0.5">
            <SecondaryButton
              onClick={() => setActiveObsession((prev) => !prev)}
              className={`min-h-0 gap-1.5 px-2.5 py-1.5 text-[0.72rem] leading-4 sm:text-xs ${activeObsession ? 'border-red-300/50 bg-red-950/25 text-red-100' : ''}`}
              disabled={isTyping || isLoading || isResolved}
            >
              <Flame className={`h-3.5 w-3.5 ${activeObsession ? 'animate-pulse' : ''}`} />
              执念
            </SecondaryButton>
            <SecondaryButton type="button" onClick={() => void handleSave()} disabled={isLoading || !currentNode || isResolved} className="min-h-0 gap-1.5 px-2.5 py-1.5 text-[0.72rem] leading-4 sm:text-xs">
              <Save className="h-3.5 w-3.5" />
              存档
            </SecondaryButton>
            <PrimaryButton
              type="button"
              className="min-h-0 gap-1.5 px-2.5 py-1.5 text-[0.72rem] leading-4 sm:text-xs"
              onClick={() => setFeedback(latestSaveId ? `最近存档：${latestSaveId}` : '本地演示模式下可先存档，稍后可继续扩展分享入口。')}
            >
              <Share2 className="h-3.5 w-3.5" />
              分享
            </PrimaryButton>
            <SecondaryButton type="button" onClick={() => setGameState('lobby')} className="min-h-0 gap-1.5 px-2.5 py-1.5 text-[0.72rem] leading-4 sm:text-xs">
              返回大厅
            </SecondaryButton>
          </div>

          {feedback ? <p className="text-xs text-[#d9cbb1] sm:text-sm">{feedback}</p> : null}
          {error && !feedback ? <p className="text-xs text-[#d9cbb1] sm:text-sm">{error}</p> : null}
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default GameplayPage;
