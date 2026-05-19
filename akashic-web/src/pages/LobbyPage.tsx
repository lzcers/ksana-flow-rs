import React from 'react';
import { Play, Sparkles } from 'lucide-react';
import {
  PageTitle,
  PrimaryButton,
  ScreenShell,
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
        <div className="relative z-10">
          <div className="space-y-6">
            <div className="flex flex-wrap gap-3">
              <StatusPill icon={Sparkles}>Akashic Engine</StatusPill>
            </div>
            <PageTitle
              title="幻世 · 人生回响"
              subtitle="开始你的命运"
            />
            <SectionCard>
              <p className="text-base leading-8 text-[#d3d9e5]">
                你想开启怎样的人生？
              </p>
            </SectionCard>
            <div className="flex">
              <PrimaryButton onClick={handleStart} className="flex-1">
                <Play className="h-4 w-4" />
                开启新人生
              </PrimaryButton>
            </div>
          </div>

        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default LobbyPage;
