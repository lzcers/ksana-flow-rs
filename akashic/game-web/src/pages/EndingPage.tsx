import React, { useMemo, useState } from 'react';
import {
  Crown,
  Flame,
  Orbit,
  Sparkles,
  Swords,
} from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { useShallow } from 'zustand/react/shallow';
import {
  PageTitle,
  PrimaryButton,
  ScreenShell,
  SecondaryButton,
  SectionCard,
  StoryFrame,
} from '../components/AkashicUI';
import { appRoutes } from '../lib/appRoutes';
import { useGameInternalStore } from '../store/gameStore';
import { useGameUIStore } from '../store/gameUIStore';

interface EndingPresentation {
  emblem: string;
  title: string;
  subtitle: string;
  cardTitle: string;
  cardBody: string;
  echoTitle: string;
  echoBody: string;
  accentClassName: string;
  borderClassName: string;
  glowClassName: string;
  icon: typeof Crown;
}

function endingPresentation(endingType: string | null): EndingPresentation {
  switch (endingType) {
    case 'triumph':
      return {
        emblem: '凯歌终章',
        title: '你把命运推到了所愿之处',
        subtitle: '回响在此刻收束成一枚耀眼的结晶。那些曾压在你身上的阴影，被你亲手改写成了抵达。',
        cardTitle: '胜利没有白白降临',
        cardBody: '你跨过了最艰难的门槛，让执念、选择与代价终于换回一个能够被称作圆满的回答。',
        echoTitle: '余辉仍会延续',
        echoBody: '即便故事停在这里，这份胜利也会继续影响仍留在世界里的每一个人。',
        accentClassName: 'text-amber-100',
        borderClassName: 'border-amber-300/35 bg-[linear-gradient(135deg,rgba(116,72,22,0.92),rgba(31,21,14,0.92))]',
        glowClassName: 'from-amber-300/20 via-amber-200/10 to-transparent',
        icon: Crown,
      };
    case 'tragedy':
      return {
        emblem: '沉坠终章',
        title: '你抵达了无法挽回的尽头',
        subtitle: '回响没有替你避开代价。它只是让最后一刻更清晰地落下，让失去成为这段旅程的名字。',
        cardTitle: '结局并不仁慈',
        cardBody: '你曾努力伸手，却还是让某些重要之物从掌心坠落。故事没有回头，代价也不会被抹去。',
        echoTitle: '沉默仍在扩散',
        echoBody: '那些没有来得及说出口的话，会在更久的时间里化作世界的暗流。',
        accentClassName: 'text-rose-100',
        borderClassName: 'border-rose-300/30 bg-[linear-gradient(135deg,rgba(76,20,31,0.92),rgba(22,14,20,0.94))]',
        glowClassName: 'from-rose-300/18 via-rose-200/8 to-transparent',
        icon: Flame,
      };
    case 'bittersweet':
      return {
        emblem: '残响终章',
        title: '你得到了答案，也失去了别的东西',
        subtitle: '命运给出的从不是完整的恩赐。它让你带着结果离开，也让你明白自己究竟付出了什么。',
        cardTitle: '所得与所失并肩而立',
        cardBody: '你没有空手而归，但也无法像故事开始时那样完整。正因如此，这段回响才显得真实。',
        echoTitle: '温热与刺痛同时留下',
        echoBody: '记忆会在未来某个时刻再次浮现，让你同时想起拥抱与裂口。',
        accentClassName: 'text-violet-100',
        borderClassName: 'border-violet-300/30 bg-[linear-gradient(135deg,rgba(55,33,98,0.92),rgba(20,16,34,0.94))]',
        glowClassName: 'from-violet-300/18 via-fuchsia-200/10 to-transparent',
        icon: Swords,
      };
    case 'open':
      return {
        emblem: '未竟终章',
        title: '这一段旅程停下了，世界却还没有',
        subtitle: '门在此处轻轻合上，但远方仍有风声。你离开这一页时，更多可能仍在暗处继续发芽。',
        cardTitle: '故事只是暂别',
        cardBody: '你触碰到了一个阶段的终点，却没有真的看见世界的全部。未被讲述的部分，仍在等待后来者。',
        echoTitle: '远处还有潮汐',
        echoBody: '就算此刻不再继续，你也知道命运并没有彻底熄灭，它只是把火留在了更远的地方。',
        accentClassName: 'text-sky-100',
        borderClassName: 'border-sky-300/30 bg-[linear-gradient(135deg,rgba(26,61,92,0.92),rgba(14,19,31,0.94))]',
        glowClassName: 'from-sky-300/18 via-cyan-200/10 to-transparent',
        icon: Orbit,
      };
    default:
      return {
        emblem: '终章已定',
        title: '故事在这里缓缓落下帷幕',
        subtitle: '回响替你收好这一路的余温与裂痕，让最后一页以自己的方式静静合拢。',
        cardTitle: '这一程已经完成',
        cardBody: '也许它没有一个容易归类的名字，但它确实已经抵达属于自己的终点。',
        echoTitle: '余韵会继续停留',
        echoBody: '就算你转身离开，这段旅程留下的波纹也不会立刻消散。',
        accentClassName: 'text-[#f6eddc]',
        borderClassName: 'border-[#c9b38f]/28 bg-[linear-gradient(135deg,rgba(64,48,29,0.92),rgba(17,17,19,0.94))]',
        glowClassName: 'from-[#d9cbb1]/14 via-[#d9cbb1]/8 to-transparent',
        icon: Sparkles,
      };
  }
}

const EndingPage: React.FC = () => {
  const navigate = useNavigate();
  const [feedback, setFeedback] = useState<string | null>(null);
  const {
    stateView,
    isLoading,
    error,
    createSave,
    resetGame,
  } = useGameUIStore(useShallow((state) => ({
    stateView: state.stateView,
    isLoading: state.isLoading,
    error: state.error,
    createSave: state.createSave,
    resetGame: state.resetGame,
  })));
  const roundStates = useGameInternalStore((state) => state.roundStates);
  const presentation = endingPresentation(stateView?.endingType ?? null);
  const Icon = presentation.icon;
  const lastRound = useMemo(() => (
    Object.values(roundStates)
      .filter((entry) => entry.narrationText || entry.selectedChoiceText)
      .sort((left, right) => right.round - left.round)[0]
  ), [roundStates]);
  const lastNarration = lastRound?.narrationText?.trim() || stateView?.latestHistory?.trim() || '最后一段余音还停留在风里。';
  const lastChoice = lastRound?.selectedChoiceText?.trim() || '你走到了命运为你收束的这一刻。';

  const handleSave = async () => {
    try {
      await createSave();
      setFeedback('这段终章已经被妥善收好。');
    } catch (saveError) {
      setFeedback(saveError instanceof Error ? saveError.message : '保存这段终章失败。');
    }
  };

  const handleBackToLobby = () => {
    resetGame();
    navigate(appRoutes.lobby);
  };

  return (
    <ScreenShell className="items-center">
      <StoryFrame className="relative max-w-4xl overflow-hidden px-4 py-5 md:px-6 md:py-6">
        <div className={`pointer-events-none absolute inset-0 bg-linear-to-br ${presentation.glowClassName}`} />
        <div className="relative z-10 space-y-5">
          <PageTitle
            title={presentation.title}
            subtitle={presentation.subtitle}
          />

          {(feedback || error) ? (
            <div className="rounded-[1.1rem] border border-[#d6c3a0]/25 bg-[#17151d]/82 px-4 py-3 text-sm text-[#f1e7d4]">
              {feedback ?? error}
            </div>
          ) : null}

          <SectionCard className={`relative overflow-hidden ${presentation.borderClassName}`}>
            <div className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
              <div className="space-y-3">
                <div className={`inline-flex items-center rounded-full border border-white/10 bg-black/15 px-3 py-1 text-xs tracking-[0.24em] ${presentation.accentClassName}`}>
                  {presentation.emblem}
                </div>
                <div className="space-y-2">
                  <h2 className="text-2xl font-semibold text-white sm:text-[2rem]">{presentation.cardTitle}</h2>
                  <p className="max-w-2xl text-sm leading-7 text-white/82 sm:text-base">
                    {presentation.cardBody}
                  </p>
                </div>
              </div>
              <div className="flex h-14 w-14 shrink-0 items-center justify-center rounded-2xl border border-white/12 bg-black/20 text-white/90 shadow-[0_16px_40px_rgba(0,0,0,0.22)]">
                <Icon className="h-7 w-7" />
              </div>
            </div>
          </SectionCard>

          <div className="grid gap-3 md:grid-cols-3">
            <SectionCard className="space-y-2">
              <p className="text-xs tracking-[0.24em] text-[#bca984]">终章余波</p>
              <p className="text-lg font-medium text-[#f6eddc]">{presentation.echoTitle}</p>
              <p className="text-sm leading-6 text-[#aeb6c6]">{presentation.echoBody}</p>
            </SectionCard>
            <SectionCard className="space-y-2">
              <p className="text-xs tracking-[0.24em] text-[#bca984]">最后场景</p>
              <p className="text-lg font-medium text-[#f6eddc]">
                {stateView?.currentScene || '终章现场'}
              </p>
              <p className="text-sm leading-6 text-[#aeb6c6]">
                {stateView?.currentLocation || '命运在此刻完成收束。'}
              </p>
            </SectionCard>
            <SectionCard className="space-y-2">
              <p className="text-xs tracking-[0.24em] text-[#bca984]">最后一步</p>
              <p className="text-sm leading-6 text-[#f3e8d2]">{lastChoice}</p>
            </SectionCard>
          </div>

          <SectionCard className="space-y-3">
            <p className="text-xs tracking-[0.24em] text-[#bca984]">最后的余音</p>
            <p className="whitespace-pre-wrap text-sm leading-7 text-[#d8dee9] sm:text-[0.95rem]">
              {lastNarration}
            </p>
          </SectionCard>

          <div className="flex flex-col gap-3 sm:flex-row sm:justify-center">
            <PrimaryButton onClick={() => void handleSave()} disabled={isLoading} className="min-w-44">
              {isLoading ? '封存终章中...' : '封存这段终章'}
            </PrimaryButton>
            <SecondaryButton onClick={handleBackToLobby} disabled={isLoading} className="min-w-44">
              回到回响厅
            </SecondaryButton>
          </div>
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default EndingPage;
