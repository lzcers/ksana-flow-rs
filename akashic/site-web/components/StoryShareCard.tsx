import { ArrowUpRight, Compass, Sparkles } from 'lucide-react'

import { cn } from '../lib/utils'

type StoryShareCardProps = {
  summary: string
  gameUrl: string
  className?: string
  eyebrow?: string
  title?: string
  sessionLabel?: string
  ctaLabel?: string
}

export default function StoryShareCard({
  summary,
  gameUrl,
  className,
  eyebrow = 'AKASHIC ECHO',
  title = '这一段命运，值得被分享',
  sessionLabel = '故事摘要',
  ctaLabel = '进入游戏',
}: StoryShareCardProps) {
  const content = summary.trim()

  return (
    <article
      className={cn(
        'game-card relative overflow-hidden border-border/60 bg-[radial-gradient(circle_at_top,rgba(120,220,210,0.12),transparent_34%),linear-gradient(160deg,rgba(10,16,34,0.98),rgba(8,12,24,0.92))] py-0 shadow-[0_24px_80px_rgba(1,8,20,0.6)]',
        className,
      )}
    >
      <div className="pointer-events-none absolute inset-0">
        <div className="absolute -left-16 top-8 h-36 w-36 rounded-full bg-accent/12 blur-3xl" />
        <div className="absolute right-0 top-0 h-48 w-48 bg-[radial-gradient(circle,rgba(232,204,130,0.22),transparent_62%)]" />
        <div className="absolute inset-x-8 top-0 h-px bg-gradient-to-r from-transparent via-primary/60 to-transparent" />
        <div className="absolute bottom-0 right-10 h-px w-28 bg-gradient-to-r from-transparent to-accent/60" />
        <div className="absolute bottom-6 right-6 h-20 w-20 rounded-full border border-primary/10" />
      </div>

      <div className="relative border-b border-border/50 px-6 py-6 sm:px-7">
        <div className="flex items-start justify-between gap-4">
          <div className="space-y-3">
            <div className="inline-flex items-center gap-2 rounded-full border border-primary/20 bg-primary/10 px-3 py-1 text-[10px] tracking-[0.28em] text-primary/90 uppercase">
              <Sparkles className="size-3" />
              <span>{eyebrow}</span>
            </div>
            <div className="space-y-2">
              <p className="text-xs tracking-[0.28em] text-accent/80 uppercase">{sessionLabel}</p>
              <h2 className="max-w-xl font-serif text-2xl leading-tight text-foreground sm:text-[2rem]">
                {title}
              </h2>
            </div>
          </div>

          <div className="hidden rounded-full border border-border/70 bg-background/40 px-3 py-1 text-xs text-muted-foreground backdrop-blur-sm sm:inline-flex sm:items-center sm:gap-2">
            <Compass className="size-3.5 text-accent" />
            <span>命运入口已附上</span>
          </div>
        </div>
      </div>

      <div className="relative px-6 py-6 sm:px-7 sm:py-7">
        <div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_11rem] lg:items-end">
          <div className="relative">
            <div className="absolute -left-1 top-0 h-10 w-10 rounded-full bg-primary/8 blur-2xl" />
            <div className="relative rounded-[1.4rem] border border-border/70 bg-background/30 p-5 shadow-[inset_0_1px_0_rgba(255,255,255,0.04)] backdrop-blur-sm sm:p-6">
              <div className="mb-4 flex items-center gap-3">
                <div className="h-px flex-1 bg-gradient-to-r from-primary/70 to-transparent" />
                <span className="text-[11px] tracking-[0.28em] text-primary/85 uppercase">Summary</span>
              </div>
              <p className="font-serif text-[1.02rem] leading-8 text-foreground/92 sm:text-[1.1rem]">
                {content || '命运尚未留下可供摘录的回响。'}
              </p>
            </div>
          </div>

          <div className="rounded-[1.5rem] border border-accent/20 bg-accent/8 p-4 backdrop-blur-sm">
            <p className="text-[11px] tracking-[0.28em] text-accent/80 uppercase">Game Portal</p>
            <p className="mt-3 text-sm leading-6 text-muted-foreground">
              沿着这段摘要继续下沉，回到故事现场，把下一轮选择亲手推向结局。
            </p>
          </div>
        </div>
      </div>

      <div className="relative flex flex-col items-start gap-4 border-t border-border/50 px-6 py-5 sm:flex-row sm:items-center sm:justify-between sm:px-7">
        <div className="space-y-1">
          <p className="text-xs tracking-[0.24em] text-muted-foreground uppercase">Share-ready Card</p>
          <p className="text-sm text-foreground/80">适合在社区、群聊或活动页中展示这一段剧情回响。</p>
        </div>

        <a
          href={gameUrl}
          target="_blank"
          rel="noreferrer"
          className="inline-flex h-11 items-center justify-center gap-2 rounded-full bg-primary px-5 text-sm font-medium text-primary-foreground shadow-[0_10px_30px_rgba(232,204,130,0.25)] transition-colors hover:bg-primary/92 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
        >
          <span>{ctaLabel}</span>
          <ArrowUpRight className="size-4" />
        </a>
      </div>
    </article>
  )
}
