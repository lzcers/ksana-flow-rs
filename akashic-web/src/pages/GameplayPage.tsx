import React, { useEffect, useRef, useState } from 'react';
import {
  Clock3,
  Eye,
  Flame,
  Save,
  ScanFace,
  Share2,
  Sparkles,
  UserRound,
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
import { subscribeGameSessionStream } from '../lib/api';
import type { StoryStreamEventMap } from '../lib/api';

const GameplayPage: React.FC = () => {
  const {
    sessionId,
    currentNode,
    submitChoice,
    obsessionPoints,
    intuitionPoints,
    worldNews,
    previewChoice,
    character,
    stateView,
    daysLeft,
    createSave,
    latestSaveId,
    isLoading,
    error,
    refreshSession,
  } = useGameStore();
  const [isTyping, setIsTyping] = useState(true);
  const [activeObsession, setActiveObsession] = useState(false);
  const [previews, setPreviews] = useState<Record<string, string>>({});
  const [feedback, setFeedback] = useState<string | null>(null);
  const bootstrappedSessionIdRef = useRef<string | null>(null);

  const currentLocation = stateView?.currentLocation ?? '灰雾城区 · 未知坐标';

  useEffect(() => {
    if (!sessionId || currentNode || bootstrappedSessionIdRef.current === sessionId) {
      return;
    }

    bootstrappedSessionIdRef.current = sessionId;

  }, [currentNode, sessionId]);

  useEffect(() => {
    if (!feedback) return undefined;

    const timer = window.setTimeout(() => setFeedback(null), 2200);
    return () => window.clearTimeout(timer);
  }, [feedback]);

  useEffect(() => {
    if (!sessionId) {
      return undefined;
    }

    const unsubscribe = subscribeGameSessionStream(sessionId, {
      onEvent: (event) => {
        switch (event.event) {
          case 'task.updated': {
            const { task, update } = event.data as StoryStreamEventMap['task.updated'];

            if (update.chunk) {
              console.log('[GameplayPage][stream chunk]', {
                entity: task.entity,
                kind: task.kind,
                status: update.status,
                chunk: update.chunk,
              });
            }

            if (update.output) {
              console.log('[GameplayPage][stream output]', {
                entity: task.entity,
                kind: task.kind,
                status: update.status,
                output: update.output,
              });
            }

            if (update.error) {
              console.error('[GameplayPage][stream error]', {
                entity: task.entity,
                kind: task.kind,
                status: update.status,
                error: update.error,
              });
            }
            break;
          }
          case 'session.snapshot': {
            const snapshot = event.data as StoryStreamEventMap['session.snapshot'];
            console.log('[GameplayPage][session snapshot]', {
              sessionId: snapshot.sessionId,
              turnIndex: snapshot.stateView.turnIndex,
              text: snapshot.currentNode.text,
            });
            break;
          }
          case 'stream.warning':
            console.warn(
              '[GameplayPage][stream warning]',
              event.data as StoryStreamEventMap['stream.warning'],
            );
            break;
          default:
            break;
        }
      },
      onError: (streamError) => {
        console.error('[GameplayPage][stream connection error]', streamError);
      },
    });

    return unsubscribe;
  }, [sessionId]);

  if (!currentNode) {
    return (
      <ScreenShell className="items-stretch">
        <StoryFrame className="relative max-w-4xl overflow-hidden px-4 py-8 sm:px-5 sm:py-10 md:px-6 md:py-12">
          <div className="space-y-3 text-center">
            <p className="text-lg font-semibold text-[#f6eddc]">
              {isLoading ? '命运正在显影...' : '已进入故事，正在同步当前章节。'}
            </p>
            <p className="text-sm leading-6 text-[#9ca7be]">
              {error ?? '稍候片刻，开场叙事与可选行动会在这里展开。'}
            </p>
            {!isLoading && sessionId ? (
              <div className="pt-2">
                <PrimaryButton type="button" onClick={() => void refreshSession()}>
                  重新同步故事
                </PrimaryButton>
              </div>
            ) : null}
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
      setFeedback('当前旅程已被封存进回廊。');
    } catch (saveError) {
      setFeedback(saveError instanceof Error ? saveError.message : '存档失败。');
    }
  };

  return (
    <ScreenShell className="items-stretch">
      <StoryFrame className="relative max-w-4xl overflow-hidden px-3 py-3 sm:px-4 sm:py-4 md:px-5 md:py-5">
        <div
          className="pointer-events-none absolute inset-x-5 top-20 h-56 rounded-[1.8rem] bg-cover bg-center opacity-35 md:inset-x-6"
          style={{ backgroundImage: `url("${currentNode.image}")` }}
        />
        <div className="pointer-events-none absolute inset-0 bg-gradient-to-b from-transparent via-[#08111d]/35 to-[#08111d]" />

        <div className="relative z-10 space-y-5">
          <div className="flex flex-wrap gap-3">
            <StatusPill icon={Flame}>执念 {obsessionPoints}/5</StatusPill>
            <StatusPill icon={Eye}>直觉 {intuitionPoints}</StatusPill>
            <StatusPill icon={Clock3}>{daysLeft}日</StatusPill>
          </div>

          <section className="akashic-panel flex min-h-[13.5rem] items-center justify-center px-4 py-6 text-center sm:min-h-[15rem] sm:px-5 sm:py-7 md:min-h-[16rem] md:px-6 md:py-8">
            <div className="space-y-3">
              <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full border border-[#6f6655] bg-[#23282f]/90 text-[#c8bca9] sm:h-18 sm:w-18 md:h-20 md:w-20">
                <ScanFace className="h-8 w-8 md:h-10 md:w-10" />
              </div>
              <div className="inline-flex items-center justify-center gap-2 text-[#d8ccb8]">
                <UserRound className="h-4 w-4" />
                <span className="text-xl font-semibold sm:text-2xl">{character.name || '无名旅人'}</span>
              </div>
              <p className="text-sm text-[#9ca7be]">{currentLocation}</p>
              <p className="mx-auto max-w-xl text-sm leading-6 text-[#c3cde0]/85">
                {character.appearance || '一张尚未被叙事完全揭露的面孔。'}
              </p>
            </div>
          </section>

          {worldNews ? (
            <div className="akashic-pill w-fit border-amber-300/50 bg-[#1d1820]/95 text-amber-100">
              <Sparkles className="h-4 w-4 text-amber-200" />
              <span>{worldNews}</span>
            </div>
          ) : null}

          <section className="akashic-panel px-4 py-4 sm:px-5 sm:py-5 md:px-7 md:py-6">
            <div className="rounded-[1.15rem] bg-[#040912]/90 pl-4 sm:rounded-[1.35rem] sm:pl-5 md:rounded-[1.5rem] md:pl-6">
              <div className="border-l-[3px] border-[#d4b688] py-4 pr-4 sm:py-5 sm:pr-5 md:py-6 md:pr-6">
                <div className="min-h-[6rem] whitespace-pre-wrap text-[1.05rem] font-semibold leading-[1.8] text-[#f6eddc] sm:min-h-[6.75rem] sm:text-[1.3rem] md:min-h-[7.5rem] md:text-[2rem]">
                  <Typewriter text={currentNode.text} speed={28} onComplete={() => setIsTyping(false)} />
                </div>
              </div>
            </div>
            <p className="mt-3 text-xs tracking-wide text-[#7f8aa2]">
              {isTyping ? '轻触文字区域可跳过打字效果。' : '命运已经停顿，请做出你的选择。'}
            </p>
          </section>

          <div className="space-y-3">
            {currentNode.choices.map((choice) => (
              <div key={choice.id} className="space-y-2">
                <div className="flex gap-3">
                  <button
                    onClick={() => void handleChoiceClick(choice.id)}
                    disabled={isTyping || isLoading || choice.disabled}
                    className={`akashic-choice min-h-[5rem] flex-1 disabled:cursor-not-allowed disabled:opacity-50 ${activeObsession ? 'border-red-400/45 bg-red-950/20 text-red-100' : 'text-[#f3ead8]'
                      }`}
                  >
                    <div className="text-left">
                      <div className="text-lg font-semibold leading-7 sm:text-xl sm:leading-8">{choice.text}</div>
                      <div className="mt-1 text-xs leading-5 text-[#9ca7be] sm:text-sm sm:leading-6">
                        {activeObsession ? '执念将放大这次抉择的代价与回响' : '或许能遇到关键人物，或触发另一条线索'}
                      </div>
                    </div>
                  </button>

                  <button
                    type="button"
                    onClick={(e) => void handlePreview(choice.id, e)}
                    disabled={isTyping || isLoading}
                    className="akashic-icon-btn shrink-0 disabled:cursor-not-allowed disabled:opacity-50"
                    title="消耗 1 点直觉，窥探命运碎片"
                  >
                    <Eye className="h-5 w-5" />
                  </button>
                </div>

                {previews[choice.id] ? (
                  <div className="rounded-[1rem] border border-cyan-400/20 bg-cyan-950/10 px-4 py-3 text-sm leading-6 text-cyan-100/90 sm:rounded-[1.2rem] sm:px-5">
                    {previews[choice.id]}
                  </div>
                ) : null}
              </div>
            ))}
          </div>

          <div className="flex flex-wrap gap-3 pt-1">
            <SecondaryButton
              onClick={() => setActiveObsession((prev) => !prev)}
              className={activeObsession ? 'border-red-300/50 bg-red-950/25 text-red-100' : ''}
              disabled={isTyping || isLoading}
            >
              <Flame className={`h-4 w-4 ${activeObsession ? 'animate-pulse' : ''}`} />
              倾注执念
            </SecondaryButton>
            <SecondaryButton type="button" onClick={() => void handleSave()} disabled={isLoading}>
              <Save className="h-4 w-4" />
              存档
            </SecondaryButton>
            <PrimaryButton
              type="button"
              onClick={() => setFeedback(latestSaveId ? `最近存档：${latestSaveId}` : '请先存档，再进行分享。')}
            >
              <Share2 className="h-4 w-4" />
              存档/分享
            </PrimaryButton>
          </div>

          {feedback ? <p className="text-sm text-[#d9cbb1]">{feedback}</p> : null}
          {error && !feedback ? <p className="text-sm text-[#d9cbb1]">{error}</p> : null}
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default GameplayPage;
