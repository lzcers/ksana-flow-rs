import React, { useEffect, useMemo, useState } from 'react';
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

const locationByEra: Record<string, string> = {
  蒸汽朋克: '铸铁之城 · 下环区',
  星际拓荒: '边境星港 · 第七码头',
  东方玄幻: '雾隐城 · 长街',
  末日废土: '余烬聚落 · 风口哨站',
};

const GameplayPage: React.FC = () => {
  const {
    storyNodes,
    currentNodeId,
    makeChoice,
    obsessionPoints,
    intuitionPoints,
    worldNews,
    useIntuition,
    useObsession,
    character,
    world,
  } = useGameStore();
  const [isTyping, setIsTyping] = useState(true);
  const [activeObsession, setActiveObsession] = useState(false);
  const [previews, setPreviews] = useState<Record<string, string>>({});
  const [feedback, setFeedback] = useState<string | null>(null);

  const currentNode = storyNodes.find((node) => node.id === currentNodeId);
  const daysLeft = useMemo(() => Math.max(30 - Math.max(storyNodes.length - 1, 0) * 6, 0), [storyNodes.length]);
  const currentLocation = locationByEra[world.era] ?? '灰雾城区 · 未知坐标';

  useEffect(() => {
    if (!feedback) return undefined;

    const timer = window.setTimeout(() => setFeedback(null), 2200);
    return () => window.clearTimeout(timer);
  }, [feedback]);

  if (!currentNode) return null;

  const handlePreview = (choiceId: string, choiceText: string, e: React.MouseEvent) => {
    e.stopPropagation();

    if (previews[choiceId]) return;

    if (!useIntuition()) {
      setFeedback('直觉不足，无法窥探未来片段。');
      return;
    }

    setPreviews((prev) => ({
      ...prev,
      [choiceId]: `未来的模糊片段在你眼前闪现：若选择“${choiceText}”，某个被遗忘的人会比你更早抵达真相。`,
    }));
    setFeedback('你窥见了一角尚未到来的命运。');
  };

  const handleChoiceClick = (choiceId: string) => {
    if (activeObsession) {
      if (!useObsession()) {
        setFeedback('执念已经耗尽，无法继续强行扭动命运。');
        return;
      }
      makeChoice(choiceId, true);
    } else {
      makeChoice(choiceId, false);
    }

    setIsTyping(true);
    setActiveObsession(false);
    setPreviews({});
    setFeedback(null);
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
                    onClick={() => handleChoiceClick(choice.id)}
                    disabled={isTyping}
                    className={`akashic-choice min-h-[5rem] flex-1 disabled:cursor-not-allowed disabled:opacity-50 ${
                      activeObsession ? 'border-red-400/45 bg-red-950/20 text-red-100' : 'text-[#f3ead8]'
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
                    onClick={(e) => handlePreview(choice.id, choice.text, e)}
                    disabled={isTyping}
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
              disabled={isTyping}
            >
              <Flame className={`h-4 w-4 ${activeObsession ? 'animate-pulse' : ''}`} />
              倾注执念
            </SecondaryButton>
            <SecondaryButton type="button">
              <Save className="h-4 w-4" />
              存档
            </SecondaryButton>
            <PrimaryButton type="button">
              <Share2 className="h-4 w-4" />
              存档/分享
            </PrimaryButton>
          </div>

          {feedback ? <p className="text-sm text-[#d9cbb1]">{feedback}</p> : null}
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default GameplayPage;
