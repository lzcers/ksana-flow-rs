import React from 'react';
import { ArrowUpRight, Compass, Sparkles } from 'lucide-react';

import { cn } from '../lib/utils';

interface StoryShareCardProps {
  summary: string;
  gameUrl: string;
  className?: string;
  eyebrow?: string;
  title?: string;
  sessionLabel?: string;
  ctaLabel?: string;
}

const StoryShareCard: React.FC<StoryShareCardProps> = ({
  summary,
  gameUrl,
  className,
  eyebrow = 'AKASHIC ECHO',
  title = '这一段命运，值得被分享',
  sessionLabel = '故事摘要',
  ctaLabel = '进入游戏',
}) => {
  const content = summary.trim();

  return (
    <article
      className={cn(
        'game-card relative overflow-hidden border-[rgba(116,103,80,0.58)] bg-[radial-gradient(circle_at_top,rgba(97,190,183,0.14),transparent_34%),linear-gradient(160deg,rgba(10,16,34,0.98),rgba(8,12,24,0.94))] py-0 shadow-[0_24px_80px_rgba(1,8,20,0.6)]',
        className,
      )}
    >
      <div className="pointer-events-none absolute inset-0">
        <div className="absolute -left-16 top-8 h-36 w-36 rounded-full bg-cyan-300/10 blur-3xl" />
        <div className="absolute right-0 top-0 h-48 w-48 bg-[radial-gradient(circle,rgba(232,204,130,0.2),transparent_62%)]" />
        <div className="absolute inset-x-8 top-0 h-px bg-gradient-to-r from-transparent via-[#d8c18f]/60 to-transparent" />
        <div className="absolute bottom-0 right-10 h-px w-28 bg-gradient-to-r from-transparent to-cyan-300/60" />
        <div className="absolute bottom-6 right-6 h-20 w-20 rounded-full border border-[#d8c18f]/10" />
      </div>

      <div className="relative border-b border-[rgba(116,103,80,0.42)] px-5 py-5 sm:px-6">
        <div className="flex items-start justify-between gap-4">
          <div className="space-y-3">
            <div className="inline-flex items-center gap-2 rounded-full border border-[#d8c18f]/20 bg-[#d8c18f]/10 px-3 py-1 text-[10px] tracking-[0.28em] text-[#e6d1a2] uppercase">
              <Sparkles className="h-3 w-3" />
              <span>{eyebrow}</span>
            </div>
            <div className="space-y-2">
              <p className="text-xs tracking-[0.28em] text-cyan-100/75 uppercase">{sessionLabel}</p>
              <h2 className="max-w-xl text-2xl leading-tight text-[#f4ecd8] sm:text-[2rem]">
                {title}
              </h2>
            </div>
          </div>

          <div className="hidden rounded-full border border-[rgba(116,103,80,0.58)] bg-[rgba(8,14,26,0.45)] px-3 py-1 text-xs text-[#a8b4c7] backdrop-blur-sm sm:inline-flex sm:items-center sm:gap-2">
            <Compass className="h-3.5 w-3.5 text-cyan-100/75" />
            <span>命运入口已附上</span>
          </div>
        </div>
      </div>

      <div className="relative px-5 py-5 sm:px-6 sm:py-6">
        <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_11rem] lg:items-end">
          <div className="relative">
            <div className="absolute -left-1 top-0 h-10 w-10 rounded-full bg-[#d8c18f]/8 blur-2xl" />
            <div className="relative rounded-[1.35rem] border border-[rgba(116,103,80,0.42)] bg-[rgba(8,14,26,0.38)] p-5 shadow-[inset_0_1px_0_rgba(255,255,255,0.04)] backdrop-blur-sm sm:p-6">
              <div className="mb-4 flex items-center gap-3">
                <div className="h-px flex-1 bg-gradient-to-r from-[#d8c18f]/70 to-transparent" />
                <span className="text-[11px] tracking-[0.28em] text-[#e6d1a2]/85 uppercase">Summary</span>
              </div>
              <p className="text-[1rem] leading-8 text-[#f3ead8]/92 sm:text-[1.05rem]">
                {content || '命运尚未留下可供摘录的回响。'}
              </p>
            </div>
          </div>

          <div className="rounded-[1.4rem] border border-cyan-100/15 bg-cyan-100/6 p-4 backdrop-blur-sm">
            <p className="text-[11px] tracking-[0.28em] text-cyan-100/75 uppercase">Game Portal</p>
            <p className="mt-3 text-sm leading-6 text-[#a8b4c7]">
              沿着这段摘要继续下沉，回到故事现场，把下一轮选择亲手推向结局。
            </p>
            <p className="mt-4 break-all rounded-xl border border-white/6 bg-black/10 px-3 py-2 text-[0.72rem] leading-5 text-[#d8c18f]">
              {gameUrl}
            </p>
          </div>
        </div>
      </div>

      <div className="relative flex flex-col items-start gap-4 border-t border-[rgba(116,103,80,0.42)] px-5 py-4 sm:flex-row sm:items-center sm:justify-between sm:px-6">
        <div className="space-y-1">
          <p className="text-xs tracking-[0.24em] text-[#8f98ab] uppercase">Share-ready Card</p>
          <p className="text-sm text-[#e9dec8]/80">适合在群聊、社区或活动页中展示这一段剧情回响。</p>
        </div>

        <a
          href={gameUrl}
          target="_blank"
          rel="noreferrer"
          className="inline-flex h-11 items-center justify-center gap-2 rounded-full bg-[#d8c18f] px-5 text-sm font-medium text-[#111624] shadow-[0_10px_30px_rgba(216,193,143,0.25)] transition-colors hover:bg-[#e4d1a9]"
        >
          <span>{ctaLabel}</span>
          <ArrowUpRight className="h-4 w-4" />
        </a>
      </div>
    </article>
  );
};

export default StoryShareCard;
