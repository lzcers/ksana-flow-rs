import React, { useState } from 'react';
import { Library, FolderOpen, Play, Sparkles, TriangleAlert } from 'lucide-react';
import {
  PageTitle,
  PrimaryButton,
  ScreenShell,
  SecondaryButton,
  SectionCard,
  StoryFrame,
  StatusPill,
} from '../components/AkashicUI';
import { useGameUIStore } from '../store/gameStore';

const LobbyPage: React.FC = () => {
  const setGameState = useGameUIStore((state) => state.setGameState);
  const resetGame = useGameUIStore((state) => state.resetGame);
  const loadSave = useGameUIStore((state) => state.loadSave);
  const isLoading = useGameUIStore((state) => state.isLoading);
  const error = useGameUIStore((state) => state.error);
  const [slotId, setSlotId] = useState('');

  const handleStart = () => {
    resetGame();
    setGameState('creation');
  };

  const handleLoad = async () => {
    try {
      await loadSave(slotId);
    } catch {
      // Store already exposes the failure reason.
    }
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
            {error ? (
              <StatusPill
                icon={TriangleAlert}
                className="border-[#7f3b3b]/50 bg-[#2a1216]/85 text-[#ffd7d7]"
                iconClassName="text-[#ff9b9b]"
              >
                {error}
              </StatusPill>
            ) : null}
            <SectionCard>
              <div className="space-y-4">
                <p className="text-base leading-8 text-[#d3d9e5]">
                  你想开启怎样的人生？
                </p>
                <div className="space-y-3">
                  <p className="text-sm text-[#9ca7be]">
                    或者输入一个存档槽 ID，直接回到上一次命运停驻之处。
                  </p>
                  <input
                    type="text"
                    value={slotId}
                    onChange={(event) => setSlotId(event.target.value)}
                    className="akashic-field"
                    placeholder="例如：slot-main"
                  />
                </div>
              </div>
            </SectionCard>
            <div className="flex flex-col gap-3 sm:flex-row">
              <PrimaryButton onClick={handleStart} disabled={isLoading} className="flex-1">
                <Play className="h-4 w-4" />
                开启新人生
              </PrimaryButton>
              <SecondaryButton
                onClick={() => setGameState('archive_list')}
                disabled={isLoading}
                className="flex-1"
              >
                <Library className="h-4 w-4" />
                存档列表
              </SecondaryButton>
              <SecondaryButton
                onClick={handleLoad}
                disabled={isLoading || !slotId.trim()}
                className="flex-1"
              >
                <FolderOpen className="h-4 w-4" />
                {isLoading ? '读取中...' : '读取存档'}
              </SecondaryButton>
            </div>
          </div>

        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default LobbyPage;
