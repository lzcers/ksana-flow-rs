type ChangelogEntry = {
  version: string
  date: string
  type: 'major' | 'minor' | 'patch'
  title: string
  description: string
  changes: {
    type: 'new' | 'improvement' | 'fix' | 'story'
    text: string
  }[]
}

const changelog: ChangelogEntry[] = [
  {
    version: '1.2.0',
    date: '2026-05-28',
    type: 'minor',
    title: '序章',
    description: '',
    changes: [
      { type: 'new', text: '互动文字游戏形态-第一版' },
    ],
  },
]

const typeLabels = {
  new: '新增',
  improvement: '优化',
  fix: '修复',
  story: '剧情',
}

export default function ChangelogPage() {
  return (
    <div className="min-h-screen py-24 px-6">
      <div className="max-w-6xl mx-auto">
        <div className="grid gap-8 xl:grid-cols-[minmax(18rem,0.8fr)_minmax(0,1.2fr)] xl:items-start">
          <div className="space-y-5 xl:sticky xl:top-28">
            <div>
              <p className="text-xs tracking-[0.32em] text-accent/80 mb-3">
                CHRONICLE
              </p>
              <h1 className="font-serif text-3xl md:text-4xl text-foreground mb-3">
                更新日志
              </h1>
            </div>

            <div className="game-card p-5">
              <p className="text-sm text-foreground mb-3">当前阶段</p>
              <div className="space-y-2 text-sm text-muted-foreground">
                <p>形态：互动文字游戏第一版</p>
                <p>重点：先把序章与世界唤醒流程落稳</p>
                <p>方向：继续补足体验与内容深度</p>
              </div>
            </div>
          </div>

          <div className="space-y-6">
            {changelog.map((entry) => (
              <section
                key={entry.version}
                className="p-5 lg:p-6"
              >
                <div className="grid gap-5 lg:grid-cols-[minmax(10rem,0.34fr)_minmax(0,1fr)] lg:gap-8">
                  <div className="lg:border-r lg:border-border/70 lg:pr-6">
                    <p className="text-xs tracking-[0.28em] text-accent/80 mb-3">
                      {entry.date}
                    </p>
                    <p className="font-serif text-2xl text-foreground mb-2">
                      v{entry.version}
                    </p>
                    <span className="inline-flex rounded-full border border-accent/30 bg-accent/8 px-3 py-1 text-xs text-accent">
                      {entry.type === 'major' ? '重大更新' : entry.type === 'minor' ? '阶段更新' : '修补记录'}
                    </span>
                  </div>

                  <div>
                    <h2 className="font-serif text-2xl text-foreground mb-2">
                      {entry.title}
                    </h2>
                    {entry.description && (
                      <p className="text-foreground/80 leading-7 mb-4">
                        {entry.description}
                      </p>
                    )}
                    <ul className="space-y-3">
                      {entry.changes.map((change, index) => (
                        <li
                          key={`${entry.version}-${index}`}
                          className="game-card p-4 bg-background/35"
                        >
                          <p className="text-sm text-foreground mb-1">
                            {typeLabels[change.type]}
                          </p>
                          <p className="text-sm text-muted-foreground leading-7">
                            {change.text}
                          </p>
                        </li>
                      ))}
                    </ul>
                  </div>
                </div>
              </section>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}
