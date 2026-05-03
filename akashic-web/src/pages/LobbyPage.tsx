import React from 'react';
import { BookOpen, Play, Sparkles } from 'lucide-react';
import {
  PageTitle,
  PrimaryButton,
  ScreenShell,
  SecondaryButton,
  SectionCard,
  StoryFrame,
  StatusPill,
} from '../components/AkashicUI';
import { useGameStore } from '../store/gameStore';

const LobbyPage: React.FC = () => {
  const setGameState = useGameStore((state) => state.setGameState);
  const resetGame = useGameStore((state) => state.resetGame);

  const handleStart = () => {
    resetGame();
    setGameState('creation');
  };

  return (
    <ScreenShell>
      <StoryFrame className="overflow-hidden p-6 md:p-8">
        <div
          className="absolute inset-0 bg-cover bg-center bg-no-repeat opacity-20"
          style={{
            backgroundImage:
              'url("https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=A%20mystical%20ancient%20archive%20with%20rainy%20blue%20atmosphere%2C%20dark%20fantasy%20ui%20background%2C%20cinematic%20concept%20art&image_size=landscape_16_9")',
          }}
        />
        <div className="relative z-10 grid gap-6 lg:grid-cols-[1.2fr_0.8fr]">
          <div className="space-y-6">
            <div className="flex flex-wrap gap-3">
              <StatusPill icon={Sparkles}>Akashic Engine</StatusPill>
              <StatusPill icon={BookOpen}>选择将生成真实回响</StatusPill>
            </div>
            <PageTitle
              title="幻世 · 人生回响"
              subtitle="每一次进入，都会以角色、时代与抉择重写一段命运。参考图中的沉浸式黑金叙事界面将从这里开始。"
            />
            <SectionCard className="max-w-2xl">
              <p className="text-base leading-8 text-[#d3d9e5]">
                从角色塑形、命运开场，到回响结局，整个流程采用统一的深色仪式感面板。
                你将在一个被规则与叙事共同编织的世界里，感受每次选择带来的分叉与代价。
              </p>
            </SectionCard>
            <div className="flex flex-col gap-4 sm:max-w-md sm:flex-row">
              <PrimaryButton onClick={handleStart} className="flex-1">
                <Play className="h-4 w-4" />
                开启新人生
              </PrimaryButton>
            <SecondaryButton className="flex-1" onClick={() => setGameState('corridor')}>
                <BookOpen className="h-4 w-4" />
                我的回廊
              </SecondaryButton>
            </div>
          </div>

          <SectionCard className="flex min-h-[22rem] flex-col justify-between">
            <div className="space-y-3">
              <p className="text-xs uppercase tracking-[0.35em] text-[#8f9bb4]">Current Ritual</p>
              <h2 className="text-3xl font-semibold text-[#f6eddc]">书写一个新的原点</h2>
              <p className="text-sm leading-7 text-[#aeb8cb]">
                角色的姓名、外貌、烙印与时代矛盾会共同塑造初始叙事。进入故事后，
                你可以消耗直觉窥探未来，也可以倾注执念强行放大抉择的结果。
              </p>
            </div>

            <div className="rounded-[1.6rem] border border-[#6f6655] bg-[#0a1222]/90 p-5">
              <p className="text-sm text-[#9ca7be]">准备阶段</p>
              <p className="mt-2 text-xl font-semibold text-[#efe4cd]">角色创建 → 命运开场 → 人生回响</p>
            </div>
          </SectionCard>
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default LobbyPage;
