import React, { useEffect, useState } from 'react';
import { Image as ImageIcon, RotateCcw, Share2, Sparkles } from 'lucide-react';
import {
  PageTitle,
  PrimaryButton,
  ScreenShell,
  SecondaryButton,
  SectionCard,
  StatusPill,
  StoryFrame,
} from '../components/AkashicUI';
import { useGameStore } from '../store/gameStore';
import { generateEndingShareCard } from '../lib/api';

const EndingPage: React.FC = () => {
  const { endingData, fetchEnding, latestArchiveId, resetGame, setGameState, error } = useGameStore();
  const [showFlashback, setShowFlashback] = useState<number | null>(null);
  const [feedback, setFeedback] = useState<string | null>(null);

  useEffect(() => {
    if (!endingData) {
      void fetchEnding();
    }
  }, [endingData, fetchEnding]);

  if (!endingData) return null;

  const handleRestart = () => {
    resetGame();
    setGameState('lobby');
  };

  const handleShare = async () => {
    if (!latestArchiveId) {
      setFeedback('当前结局尚未归档，暂时无法生成分享卡。');
      return;
    }

    try {
      const card = await generateEndingShareCard(latestArchiveId);
      setFeedback(`分享卡已生成：${card.imageUrl}`);
    } catch (shareError) {
      setFeedback(shareError instanceof Error ? shareError.message : '生成分享卡失败。');
    }
  };

  return (
    <ScreenShell className="items-stretch">
      <StoryFrame className="p-5 md:p-6">
        <div className="space-y-5">
          <div
            className="relative overflow-hidden rounded-[1.8rem] border border-[#6f6655] bg-cover bg-center px-6 py-8 md:px-8"
            style={{ backgroundImage: `linear-gradient(180deg, rgba(4, 10, 24, 0.3), rgba(4, 10, 24, 0.82)), url("${endingData.cgs[2]}")` }}
          >
            <div className="space-y-4">
              <div className="flex flex-wrap gap-3">
                <StatusPill icon={Sparkles}>结局归档</StatusPill>
                <StatusPill icon={ImageIcon}>人生回响</StatusPill>
              </div>
              <PageTitle title="此生回响录" subtitle="你的故事已被封存为一页可回看的命运档案。" />
            </div>
          </div>

          <SectionCard>
            <h2 className="mb-4 text-2xl font-semibold text-[#f6eddc]">生平纪事</h2>
            <p className="whitespace-pre-wrap text-base leading-8 text-[#d3d9e5]">{endingData.biography}</p>
          </SectionCard>

          <SectionCard>
            <h2 className="mb-5 text-2xl font-semibold text-[#f6eddc]">蝴蝶涟漪</h2>
            <div className="space-y-4">
              {endingData.turningPoints.map((point, index) => (
                <button
                  key={index}
                  type="button"
                  onClick={() => setShowFlashback(showFlashback === index ? null : index)}
                  className="w-full rounded-[1.4rem] border border-[#6f6655]/70 bg-[#0b1323]/80 p-5 text-left transition-colors hover:border-[#d4b688]/70"
                >
                  <p className="text-sm text-[#9ca7be]">{point.cause}</p>
                  <p className="mt-2 text-lg font-medium text-[#f3ead8]">{point.effect}</p>
                  {showFlashback === index ? (
                    <p className="mt-4 border-t border-white/10 pt-4 text-sm leading-7 text-[#c4d0ea]">
                      如果当时走向另一条路，也许这段命运会改写成截然不同的故事。
                    </p>
                  ) : null}
                </button>
              ))}
            </div>
          </SectionCard>

          <SectionCard>
            <h2 className="mb-4 text-2xl font-semibold text-[#f6eddc]">精神遗产</h2>
            <div className="rounded-[1.5rem] border border-[#6f6655]/70 bg-[#0b1323]/80 px-6 py-5 text-xl italic leading-9 text-[#e3d5bf]">
              “{endingData.legacy}”
            </div>
          </SectionCard>

          <SectionCard>
            <h2 className="mb-5 text-2xl font-semibold text-[#f6eddc]">记忆切片</h2>
            <div className="grid gap-4 md:grid-cols-3">
              {endingData.cgs.map((cg, i) => (
                <div key={i} className="group relative aspect-video overflow-hidden rounded-[1.25rem] border border-[#6f6655]/60">
                  <img
                    src={cg}
                    alt={`Memory ${i + 1}`}
                    className="h-full w-full object-cover transition-transform duration-500 group-hover:scale-105"
                  />
                  <div className="absolute inset-0 bg-gradient-to-t from-[#08111d]/90 to-transparent opacity-0 transition-opacity duration-300 group-hover:opacity-100" />
                </div>
              ))}
            </div>
          </SectionCard>

          <div className="flex flex-col gap-3 sm:flex-row sm:justify-end">
            <SecondaryButton type="button" onClick={() => void handleShare()}>
              <Share2 className="h-4 w-4" />
              生成分享卡
            </SecondaryButton>
            <PrimaryButton onClick={handleRestart}>
              <RotateCcw className="h-4 w-4" />
              重归大厅
            </PrimaryButton>
          </div>
          {feedback ? <p className="text-sm text-[#d9cbb1]">{feedback}</p> : null}
          {error && !feedback ? <p className="text-sm text-[#d9cbb1]">{error}</p> : null}
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default EndingPage;
