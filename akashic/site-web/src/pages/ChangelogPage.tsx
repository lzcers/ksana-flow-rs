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
      <div className="max-w-3xl mx-auto">
        <div className="mb-12">
          <h1 className="font-serif text-3xl md:text-4xl text-foreground mb-3">
            更新日志
          </h1>
          <p className="text-muted-foreground leading-7">
            记录每一次进化的足迹
          </p>
        </div>

        <div className="space-y-10">
          {changelog.map((entry) => (
            <section
              key={entry.version}
              className="border-b border-border pb-8 last:border-b-0 last:pb-0"
            >
              <p className="text-sm text-muted-foreground mb-2">
                {entry.date} · v{entry.version}
              </p>
              <h2 className="font-serif text-2xl text-foreground mb-2">
                {entry.title}
              </h2>
              <p className="text-foreground/80 leading-7 mb-4">
                {entry.description}
              </p>
              <ul className="space-y-2">
                {entry.changes.map((change, index) => (
                  <li
                    key={`${entry.version}-${index}`}
                    className="text-sm text-muted-foreground leading-7"
                  >
                    {typeLabels[change.type]}：{change.text}
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </div>
      </div>
    </div>
  )
}
