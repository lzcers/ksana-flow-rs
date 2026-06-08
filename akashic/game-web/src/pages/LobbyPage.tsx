import React from "react";
import { Library, Play, TriangleAlert } from "lucide-react";
import { useNavigate } from "react-router-dom";
import {
  PageTitle,
  PrimaryButton,
  ScreenShell,
  SecondaryButton,
  SectionCard,
  StoryFrame,
  StatusPill,
} from "../components/AkashicUI";
import { appRoutes } from "../lib/appRoutes";
import { useGameUIStore } from "../store/gameUIStore";

const LobbyPage: React.FC = () => {
  const navigate = useNavigate();
  const resetGame = useGameUIStore((state) => state.resetGame);
  const isLoading = useGameUIStore((state) => state.isLoading);
  const error = useGameUIStore((state) => state.error);

  const handleStart = () => {
    resetGame();
    navigate(appRoutes.creation);
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
            <PageTitle title="阿卡夏·回响" subtitle="命运将从此刻展开" />
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
              <p className="text-base leading-8 text-[#d3d9e5]">
                从一个名字、一段烙印开始，写下属于你的命运。
              </p>
            </SectionCard>
            <div className="flex flex-col gap-3 sm:flex-row">
              <PrimaryButton
                onClick={handleStart}
                disabled={isLoading}
                className="flex-1"
              >
                <Play className="h-4 w-4" />
                进入回响
              </PrimaryButton>
              <SecondaryButton
                onClick={() => navigate(appRoutes.archives)}
                disabled={isLoading}
                className="flex-1"
              >
                <Library className="h-4 w-4" />
                存档与导入
              </SecondaryButton>
            </div>
          </div>
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default LobbyPage;
