import { motion } from 'framer-motion'
import { useState } from 'react'

type FeedbackType = 'bug' | 'suggestion' | 'story' | 'other'

const feedbackTypes: { value: FeedbackType; label: string; icon: React.ReactNode }[] = [
  {
    value: 'bug',
    label: '问题反馈',
    icon: (
      <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
        <path d="M12 9v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
    )
  },
  {
    value: 'suggestion',
    label: '功能建议',
    icon: (
      <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
        <path d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
      </svg>
    )
  },
  {
    value: 'other',
    label: '其他',
    icon: (
      <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
        <path d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
      </svg>
    )
  },
]

export default function FeedbackPage() {
  const [type, setType] = useState<FeedbackType>('suggestion')
  const [email, setEmail] = useState('')
  const [content, setContent] = useState('')
  const [submitted, setSubmitted] = useState(false)

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    setSubmitted(true)
  }

  if (submitted) {
    return (
      <div className="min-h-screen py-24 px-6">
        <div className="max-w-md mx-auto">
          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            className="game-card p-8 text-center"
          >
            <div className="w-16 h-16 mx-auto mb-6 rounded-full bg-accent/20 flex items-center justify-center">
              <svg
                className="w-8 h-8 text-accent"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M5 13l4 4L19 7"
                />
              </svg>
            </div>
            <h2 className="font-serif text-2xl text-foreground mb-3">
              感谢你的反馈
            </h2>
            <p className="text-sm text-muted-foreground mb-6">
              我们已收到你的消息，会尽快处理。
              <br />
              你的每一条建议都是我们前进的动力。
            </p>
            <button
              onClick={() => {
                setSubmitted(false)
                setContent('')
              }}
              className="game-btn-secondary px-6 py-3"
            >
              提交新反馈
            </button>
          </motion.div>
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen py-24 px-6">
      <div className="max-w-lg mx-auto">
        {/* 标题 */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="text-center mb-10"
        >
          <h1 className="font-serif text-3xl md:text-4xl text-foreground mb-3">
            意见反馈
          </h1>
          <p className="text-muted-foreground text-sm">
            你的声音对我们至关重要
          </p>
        </motion.div>

        {/* 表单 */}
        <motion.form
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
          onSubmit={handleSubmit}
          className="game-card p-6"
        >
          {/* 反馈类型 */}
          <div className="mb-6">
            <label className="block text-sm text-muted-foreground mb-3">
              反馈类型
            </label>
            <div className="grid grid-cols-2 gap-2">
              {feedbackTypes.map((item) => (
                <button
                  key={item.value}
                  type="button"
                  onClick={() => setType(item.value)}
                  className={`p-3 rounded-xl border text-left transition-all flex items-center gap-3 ${type === item.value
                    ? 'border-accent bg-accent/10 text-foreground'
                    : 'border-border bg-card/30 text-muted-foreground hover:border-border/80'
                    }`}
                >
                  <span className={type === item.value ? 'text-accent' : ''}>{item.icon}</span>
                  <span className="text-sm">{item.label}</span>
                </button>
              ))}
            </div>
          </div>

          {/* 邮箱 */}
          <div className="mb-6">
            <label
              htmlFor="email"
              className="block text-sm text-muted-foreground mb-2"
            >
              联系邮箱 <span className="text-muted-foreground/50">(可选)</span>
            </label>
            <input
              id="email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="your@email.com"
              className="w-full px-4 py-3 rounded-xl border border-border bg-card/50 text-foreground placeholder:text-muted-foreground/40 focus:outline-none focus:border-accent/50 focus:ring-1 focus:ring-accent/20 transition-all"
            />
            <p className="mt-2 text-xs text-muted-foreground/60">
              如需我们回复，请留下邮箱
            </p>
          </div>

          {/* 反馈内容 */}
          <div className="mb-6">
            <label
              htmlFor="content"
              className="block text-sm text-muted-foreground mb-2"
            >
              反馈内容
            </label>
            <textarea
              id="content"
              value={content}
              onChange={(e) => setContent(e.target.value)}
              required
              rows={5}
              placeholder="请详细描述你的想法、遇到的问题或建议..."
              className="w-full px-4 py-3 rounded-xl border border-border bg-card/50 text-foreground placeholder:text-muted-foreground/40 focus:outline-none focus:border-accent/50 focus:ring-1 focus:ring-accent/20 transition-all resize-none"
            />
          </div>

          {/* 提交按钮 */}
          <motion.button
            type="submit"
            whileHover={{ scale: 1.01 }}
            whileTap={{ scale: 0.99 }}
            className="w-full game-btn-primary py-3.5"
          >
            提交反馈
          </motion.button>
        </motion.form>

        {/* FAQ */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.2 }}
          className="mt-12"
        >
          <h3 className="font-serif text-lg text-foreground mb-4 text-center">
            常见问题
          </h3>
          <div className="space-y-3">
            {[
              {
                q: '存档数据丢失怎么办？',
                a: '游戏数据存储在本地浏览器中。如果清除了浏览器缓存，存档可能会丢失。我们建议定期导出存档。',
              },
            ].map((item, i) => (
              <div key={i} className="game-card p-4">
                <h4 className="text-sm text-foreground mb-1.5">{item.q}</h4>
                <p className="text-xs text-muted-foreground leading-relaxed">
                  {item.a}
                </p>
              </div>
            ))}
          </div>
        </motion.div>
      </div>
    </div>
  )
}
