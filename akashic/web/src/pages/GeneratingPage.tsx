import React, { useEffect, useMemo, useState } from 'react';
import {
  BookOpenText,
  LoaderCircle,
  Orbit,
  Sparkles,
} from 'lucide-react';
import { useShallow } from 'zustand/react/shallow';
import { useGameUIStore } from '../store/gameUIStore';
import type { StartupStage } from '../store/gameUIStore';
import {
  PrimaryButton,
  PageTitle,
  ScreenShell,
  SectionCard,
  StatusPill,
  StoryFrame,
} from '../components/AkashicUI';

type StepStatus = 'pending' | 'active' | 'done';

interface StartupStep {
  key: Exclude<StartupStage, 'idle'>;
  label: string;
  title: string;
  description: string;
}

interface StageHeadline {
  title: string;
  subtitle: string;
}

const startupSteps: StartupStep[] = [
  {
    key: 'generating_world',
    label: '世界生成中',
    title: '编织世界脉络',
    description: '正在收束时代纹理、核心矛盾与规则压力，让幻世拥有可持续展开的张力。',
  },
  {
    key: 'generating_protagonist',
    label: '人物生成中',
    title: '凝聚主角命运',
    description: '正在把你的烙印、欲望、弱点与性格倾向熔进人物设定，让主角更适合长期演绎。',
  },
  {
    key: 'creating_session',
    label: '注入幻世中',
    title: '唤起第一轮命运',
    description: '世界与主角设定已经落笔，正在将它们汇入故事，并唤起开场叙事。',
  },
];

const stageOrder: Exclude<StartupStage, 'idle'>[] = [
  'generating_world',
  'generating_protagonist',
  'creating_session',
  'ready_to_enter',
];

const rotatingMessages: Record<Exclude<StartupStage, 'idle'>, string[]> = {
  generating_world: [
    '正在编织时代压力',
    '正在收束世界规则与禁忌',
    '正在校准冲突将如何逼近主角',
  ],
  generating_protagonist: [
    '正在收束主角欲望',
    '正在打磨人物弱点与裂缝',
    '正在让性格倾向落成可演绎的行动方式',
  ],
  ready_to_enter: [
    '世界设定与人物设定已经就绪',
    '命运入口已被推开一线',
    '你可以选择现在步入幻世',
  ],
  creating_session: [
    '正在注入世界设定',
    '正在唤起第一轮命运',
    '正在为你铺开故事的开场',
  ],
};

function stepStatus(currentStage: StartupStage, targetStage: Exclude<StartupStage, 'idle'>): StepStatus {
  const currentIndex = stageOrder.indexOf(currentStage === 'idle' ? 'generating_world' : currentStage);
  const targetIndex = stageOrder.indexOf(targetStage);
  if (targetIndex < currentIndex) {
    return 'done';
  }
  if (targetIndex === currentIndex) {
    return 'active';
  }
  return 'pending';
}

function stageHeadline(stage: StartupStage, name: string): StageHeadline {
  switch (stage) {
    case 'generating_protagonist':
      return {
        title: '主角轮廓正在浮现',
        subtitle: `${name} 的欲望、弱点与行动倾向正在被收束成更适合展开剧情的命运底稿。`,
      };
    case 'creating_session':
      return {
        title: '幻世入口正在开启',
        subtitle: '世界设定与主角设定已经生成，正在将它们汇入幻世，并点亮第一轮叙事。',
      };
    case 'ready_to_enter':
      return {
        title: '设定已经落笔',
        subtitle: '世界设定与主角设定已经生成完毕。确认之后，它们将正式汇入幻世，展开你的第一轮命运。',
      };
    case 'generating_world':
    case 'idle':
    default:
      return {
        title: '世界纹理正在编织',
        subtitle: '幻世会先推演世界压力，再收束主角命运，让开场更像一个真正会继续生长的故事。',
      };
  }
}

const GeneratingPage: React.FC = () => {
  const { startupStage, character, world, isLoading, error, enterWorld } = useGameUIStore(useShallow((state) => ({
    startupStage: state.startupStage,
    character: state.character,
    world: state.world,
    isLoading: state.isLoading,
    error: state.error,
    enterWorld: state.enterWorld,
  })));
  const headline = stageHeadline(startupStage, character.name || '这位主角');
  const stageKey = startupStage === 'idle' ? 'generating_world' : startupStage;
  const currentMessages = useMemo(() => rotatingMessages[stageKey], [stageKey]);
  const [messageIndex, setMessageIndex] = useState(0);

  useEffect(() => {
    setMessageIndex(0);
  }, [stageKey]);

  useEffect(() => {
    if (currentMessages.length <= 1) {
      return undefined;
    }

    const timer = window.setInterval(() => {
      setMessageIndex((current) => (current + 1) % currentMessages.length);
    }, 2200);

    return () => window.clearInterval(timer);
  }, [currentMessages]);

  return (
    <ScreenShell className="items-center">
      <StoryFrame className="max-w-3xl p-4 md:p-6">
        <div className="space-y-4 md:space-y-6">
          <PageTitle title={headline.title} subtitle={headline.subtitle} />

          <div className="rounded-[1.1rem] border border-[#6d86b7]/25 bg-[#101827]/78 px-4 py-3 text-center text-sm text-[#c7d5f2] shadow-[0_10px_30px_rgba(3,8,18,0.25)]">
            {currentMessages[messageIndex]}
          </div>

          {error ? (
            <div className="rounded-[1.1rem] border border-[#7f3b3b]/50 bg-[#2a1216]/85 px-4 py-3 text-sm text-[#ffd7d7]">
              {error}
            </div>
          ) : null}

          <SectionCard className="space-y-4">
            <div className="flex flex-wrap gap-2">
              <StatusPill icon={Orbit} className="border-[#3b82f6]/30 bg-[#0f2141]/80 text-[#cfe0ff]">
                {world.era}
              </StatusPill>
              <StatusPill icon={Sparkles} className="border-[#8b5cf6]/30 bg-[#1b1733]/80 text-[#e3d8ff]">
                {character.background || '命运烙印待揭晓'}
              </StatusPill>
              <StatusPill icon={BookOpenText} className="border-[#7c6b55]/40 bg-[#171311]/80 text-[#efe4cd]">
                核心矛盾已经明确
              </StatusPill>
            </div>

            <div className="space-y-3">
              {startupSteps.map((step) => {
                const status = stepStatus(startupStage, step.key);
                const iconClassName = status === 'active'
                  ? 'text-[#7dd3fc] animate-spin'
                  : status === 'done'
                    ? 'text-[#f4d58d]'
                    : 'text-[#5f6c86]';

                return (
                  <div
                    key={step.key}
                    className={`rounded-[1.1rem] border px-4 py-4 transition-colors md:px-5 ${status === 'active'
                      ? 'border-[#60a5fa]/40 bg-[#101a2c]/92'
                      : status === 'done'
                        ? 'border-[#8a7755]/35 bg-[#14110f]/85'
                        : 'border-white/8 bg-[#0f1420]/70'
                      }`}
                  >
                    <div className="flex items-start gap-3">
                      <LoaderCircle className={`mt-0.5 h-5 w-5 shrink-0 ${iconClassName}`} />
                      <div className="space-y-1">
                        <p className="text-sm font-semibold tracking-wide text-[#efe4cd]">{step.label}</p>
                        <p className="text-base font-medium text-[#f8f1e3] md:text-lg">{step.title}</p>
                        <p className="text-sm leading-6 text-[#9ca7be]">{step.description}</p>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </SectionCard>

          {startupStage === 'ready_to_enter' ? (
            <div className="flex justify-center">
              <PrimaryButton onClick={() => void enterWorld()} disabled={isLoading} className="min-w-44">
                {isLoading ? '步入幻世中...' : '步入幻世'}
              </PrimaryButton>
            </div>
          ) : null}
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default GeneratingPage;
