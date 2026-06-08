import React, { useMemo, useState } from "react";
import { Compass, Download, Share2, Sparkles } from "lucide-react";

import { cn } from "../lib/utils";
import { createQrCodeMatrix, qrCodeToSvgPath } from "../lib/qrCode";
import { downloadStoryShareCardImage } from "../lib/shareCardImage";

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
  eyebrow = "AKASHIC ECHO",
  title = "这一段命运，值得被分享",
  sessionLabel = "故事摘要",
  ctaLabel = "分享链接",
}) => {
  const content = summary.trim();
  const [downloadError, setDownloadError] = useState<string | null>(null);
  const [isDownloading, setIsDownloading] = useState(false);
  const qrCode = useMemo(() => {
    try {
      return createQrCodeMatrix(gameUrl);
    } catch (error) {
      return error instanceof Error ? error.message : "二维码生成失败。";
    }
  }, [gameUrl]);
  const qrSvgSize = typeof qrCode === "string" ? 0 : qrCode.size + 8;
  const qrSvgPath = typeof qrCode === "string" ? "" : qrCodeToSvgPath(qrCode);

  const handleDownloadCard = async () => {
    setIsDownloading(true);
    setDownloadError(null);
    try {
      await downloadStoryShareCardImage({
        summary: content,
        gameUrl,
        eyebrow,
        title,
        sessionLabel,
      });
    } catch (error) {
      setDownloadError(
        error instanceof Error ? error.message : "分享卡片生成失败。",
      );
    } finally {
      setIsDownloading(false);
    }
  };

  return (
    <article
      className={cn(
        "game-card relative overflow-hidden border-[rgba(116,103,80,0.58)] bg-[radial-gradient(circle_at_top,rgba(97,190,183,0.14),transparent_34%),linear-gradient(160deg,rgba(10,16,34,0.98),rgba(8,12,24,0.94))] py-0 shadow-[0_24px_80px_rgba(1,8,20,0.6)]",
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

      <div className="relative border-b border-[rgba(116,103,80,0.42)] px-4 py-3.5 sm:px-6 sm:py-5">
        <div className="flex items-start justify-between gap-4">
          <div className="space-y-2 sm:space-y-3">
            <div className="inline-flex items-center gap-1.5 rounded-full border border-[#d8c18f]/20 bg-[#d8c18f]/10 px-2.5 py-0.5 text-[9px] tracking-[0.2em] text-[#e6d1a2] uppercase sm:gap-2 sm:px-3 sm:py-1 sm:text-[10px] sm:tracking-[0.28em]">
              <Sparkles className="h-3 w-3" />
              <span>{eyebrow}</span>
            </div>
            <div className="space-y-1 sm:space-y-2">
              <p className="text-[10px] tracking-[0.22em] text-cyan-100/75 uppercase sm:text-xs sm:tracking-[0.28em]">
                {sessionLabel}
              </p>
              <h2 className="max-w-xl text-xl leading-tight text-[#f4ecd8] sm:text-[2rem]">
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

      <div className="relative px-4 py-3.5 sm:px-6 sm:py-6">
        <div className="grid gap-3 sm:gap-4 lg:grid-cols-[minmax(0,1fr)_8.5rem] lg:items-end">
          <div className="relative">
            <div className="absolute -left-1 top-0 h-10 w-10 rounded-full bg-[#d8c18f]/8 blur-2xl" />
            <div className="relative rounded-[1.1rem] border border-[rgba(116,103,80,0.42)] bg-[rgba(8,14,26,0.38)] p-3.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.04)] backdrop-blur-sm sm:rounded-[1.35rem] sm:p-6">
              <div className="mb-2.5 flex items-center gap-3 sm:mb-4">
                <div className="h-px flex-1 bg-gradient-to-r from-[#d8c18f]/70 to-transparent" />
                <span className="text-[10px] tracking-[0.22em] text-[#e6d1a2]/85 uppercase sm:text-[11px] sm:tracking-[0.28em]">
                  Summary
                </span>
              </div>
              <p className="line-clamp-4 text-sm leading-6 text-[#f3ead8]/92 sm:line-clamp-none sm:text-[1.05rem] sm:leading-8">
                {content || "命运尚未留下可供摘录的回响。"}
              </p>
            </div>
          </div>

          <div className="grid grid-cols-[minmax(0,1fr)_5.5rem] items-center gap-3 rounded-[1.1rem] border border-cyan-100/15 bg-cyan-100/6 p-3 backdrop-blur-sm sm:grid-cols-[minmax(0,1fr)_7rem] sm:rounded-[1.4rem] sm:p-4 lg:block">
            <div>
              <p className="text-[10px] tracking-[0.22em] text-cyan-100/75 uppercase sm:text-[11px] sm:tracking-[0.28em]">
                Game Portal
              </p>
              <p className="mt-2 text-xs leading-5 text-[#a8b4c7] sm:mt-3 sm:text-sm sm:leading-6">
                扫描二维码复制一条独立分支，沿着这段摘要继续推进故事。
              </p>
            </div>
            <div className="rounded-[0.85rem] border border-white/10 bg-white p-1.5 shadow-[0_16px_36px_rgba(0,0,0,0.18)] sm:rounded-[1.1rem] sm:p-2.5 lg:mt-3">
              {typeof qrCode === "string" ? (
                <div className="flex aspect-square items-center justify-center rounded-xl bg-[#f4ecd8] px-3 text-center text-xs leading-5 text-[#111624]">
                  {qrCode}
                </div>
              ) : (
                <svg
                  viewBox={`0 0 ${qrSvgSize} ${qrSvgSize}`}
                  className="aspect-square w-full"
                  role="img"
                  aria-label="分享链接二维码"
                  shapeRendering="crispEdges"
                >
                  <rect width={qrSvgSize} height={qrSvgSize} fill="#ffffff" />
                  <path d={qrSvgPath} fill="#111624" />
                </svg>
              )}
            </div>
          </div>
        </div>
      </div>

      <div className="relative flex flex-col items-start gap-3 border-t border-[rgba(116,103,80,0.42)] px-4 py-3 sm:flex-row sm:items-center sm:justify-between sm:gap-4 sm:px-6 sm:py-4">
        <div className="space-y-1">
          <p className="text-[10px] tracking-[0.2em] text-[#8f98ab] uppercase sm:text-xs sm:tracking-[0.24em]">
            Share-ready Card
          </p>
          {downloadError ? (
            <p className="text-xs leading-5 text-amber-100/85">
              {downloadError}
            </p>
          ) : null}
        </div>

        <div className="grid w-full grid-cols-2 gap-2 sm:w-auto sm:flex sm:flex-row">
          <button
            type="button"
            onClick={() => void handleDownloadCard()}
            disabled={isDownloading}
            className="inline-flex h-10 items-center justify-center gap-1.5 rounded-full border border-[#d8c18f]/35 bg-[rgba(216,193,143,0.08)] px-3 text-xs font-medium text-[#f3ead8] transition-colors hover:bg-[rgba(216,193,143,0.14)] disabled:cursor-not-allowed disabled:opacity-60 sm:h-11 sm:gap-2 sm:px-5 sm:text-sm"
          >
            <Download className="h-4 w-4" />
            <span>{isDownloading ? "生成中..." : "分享卡片"}</span>
          </button>
          <a
            href={gameUrl}
            target="_blank"
            rel="noreferrer"
            className="inline-flex h-10 items-center justify-center gap-1.5 rounded-full bg-[#d8c18f] px-3 text-xs font-medium text-[#111624] shadow-[0_10px_30px_rgba(216,193,143,0.25)] transition-colors hover:bg-[#e4d1a9] sm:h-11 sm:gap-2 sm:px-5 sm:text-sm"
          >
            <Share2 className="h-4 w-4" />
            <span>{ctaLabel}</span>
          </a>
        </div>
      </div>
    </article>
  );
};

export default StoryShareCard;
