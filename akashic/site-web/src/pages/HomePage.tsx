import { useState, useEffect, JSX } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import DynamicBackground from '../components/DynamicBackground'

const features = [
  {
    icon: (
      <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
        <circle cx="12" cy="12" r="3" />
        <path d="M12 1v4m0 14v4M1 12h4m14 0h4" />
        <path d="m4.22 4.22 2.83 2.83m9.9 9.9 2.83 2.83m-2.83-15.56 2.83 2.83m-15.56 9.9 2.83 2.83" />
      </svg>
    ),
    title: '自由设定',
    description: '你可以设定任何你想要的世界观、角色，让回响更符合你的想象',
  },
  {
    icon: (
      <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
        <path d="M12 2L2 7l10 5 10-5-10-5z" />
        <path d="M2 17l10 5 10-5" />
        <path d="M2 12l10 5 10-5" />
      </svg>
    ),
    title: '执念干涉',
    description: '你可以通过干涉来改变主角的行动，使主角执行你的意志',
  },
  // {
  //   icon: (
  //     <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
  //       <rect x="3" y="3" width="18" height="18" rx="2" />
  //       <path d="M9 9h6v6H9z" />
  //     </svg>
  //   ),
  //   title: '核心矛盾',
  //   description: '深入探索角色内心的挣扎与抉择',
  // },
  // {
  //   icon: (
  //     <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
  //       <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2z" />
  //       <path d="M12 6v6l4 2" />
  //     </svg>
  //   ),
  //   title: '轮回叙事',
  //   description: '每一轮命运都将揭示新的真相',
  // },
]

const storySteps = [
  {
    phase: '世界生成中',
    title: '编织世界脉络',
    description: '正在收束时代纹理、核心矛盾与规则压力，让回响拥有可持续展开的张力。',
    headerTitle: '世界正在苏醒',
    headerDesc: '时代的齿轮开始转动，规则与矛盾在虚空中编织成形。',
    statusText: '正在构建世界观',
    progress: 45,
    tags: [
      { icon: 'gear', text: '蒸汽朋克', color: 'accent' },
      { icon: 'layer', text: '时代纹理成型中', color: 'primary' },
      { icon: 'box', text: '规则压力就绪', color: 'accent' },
    ],
  },
  {
    phase: '人物生成中',
    title: '凝聚主角命运',
    description: '正在把你的烙印、欲望、弱点与性格倾向熔进人物设定，让主角更适合长期演绎。',
    headerTitle: '主角轮廓正在浮现',
    headerDesc: '你的欲望、弱点与行动倾向正在被收束成更适合展开剧情的命运底稿。',
    statusText: '正在收束主角欲望',
    progress: 65,
    tags: [
      { icon: 'spark', text: '命运烙印待揭晓', color: 'primary' },
      { icon: 'box', text: '核心矛盾已经明确', color: 'accent' },
      { icon: 'user', text: '性格倾向分析中', color: 'primary' },
    ],
  },
  {
    phase: '注入回响中',
    title: '唤起第一轮命运',
    description: '世界与主角设定已经落笔，正在将它们汇入故事，并唤起开场叙事。',
    headerTitle: '命运即将展开',
    headerDesc: '世界与灵魂已经交融，第一轮命运的序章正在被书写。',
    statusText: '正在编织开场叙事',
    progress: 85,
    tags: [
      { icon: 'gear', text: '蒸汽朋克', color: 'accent' },
      { icon: 'spark', text: '命运烙印已刻下', color: 'primary' },
      { icon: 'layer', text: '故事即将开始', color: 'accent' },
    ],
  },
]

const tagIcons: Record<string, JSX.Element> = {
  gear: (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
      <circle cx="12" cy="12" r="3" />
      <path d="M12 1v4m0 14v4M1 12h4m14 0h4" />
    </svg>
  ),
  layer: (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
      <path d="M12 2L2 7l10 5 10-5-10-5z" />
      <path d="M2 17l10 5 10-5" />
    </svg>
  ),
  box: (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
      <rect x="3" y="3" width="18" height="18" rx="2" />
      <path d="M9 9h6v6H9z" />
    </svg>
  ),
  spark: (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
      <path d="M12 2L2 7l10 5 10-5-10-5z" />
      <path d="M2 17l10 5 10-5" />
      <path d="M2 12l10 5 10-5" />
    </svg>
  ),
  user: (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
      <circle cx="12" cy="8" r="4" />
      <path d="M4 20c0-4 4-6 8-6s8 2 8 6" />
    </svg>
  ),
}

export default function HomePage() {
  const [currentStep, setCurrentStep] = useState(0)

  useEffect(() => {
    const interval = setInterval(() => {
      setCurrentStep((prev) => (prev + 1) % storySteps.length)
    }, 4000)
    return () => clearInterval(interval)
  }, [])

  const currentStory = storySteps[currentStep]

  return (
    <div className="min-h-screen">
      {/* 英雄区域 */}
      <section className="relative min-h-[calc(100vh-3.5rem)] lg:min-h-[calc(100vh-4rem)] flex items-center justify-center overflow-hidden">
        {/* 动态 Canvas 背景 */}
        <DynamicBackground />

        {/* 静态装饰层 */}
        <div className="absolute inset-0 pointer-events-none" style={{ zIndex: 2 }}>
          {/* 顶部渐变边框 */}
          <div className="absolute top-0 left-0 right-0 h-px bg-linear-to-r from-transparent via-accent/30 to-transparent" />
          {/* 底部渐变 */}
          <div className="absolute bottom-0 left-0 right-0 h-40 bg-linear-to-t from-background to-transparent" />
        </div>

        <div className="relative z-10 w-full max-w-6xl mx-auto px-6 py-12 lg:py-20">
          <motion.div
            initial={{ opacity: 0, y: 30 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8 }}
            className="lg:p-6"
          >
            <div className="grid gap-6 lg:grid-cols-[minmax(0,0.78fr)_minmax(0,1.12fr)] lg:items-center lg:gap-10">
              <div className="lg:pr-4">
                <div className="max-w-2xl mx-auto lg:mx-0 text-center lg:text-left">
                  <p className="text-[11px] tracking-[0.34em] text-accent/80 mb-4">
                    INTERACTIVE DESTINY SIMULATION
                  </p>
                  <h1 className="font-serif text-4xl md:text-5xl lg:text-5xl tracking-[0.08em] mb-4 text-glow">
                    <span className="text-primary">阿卡夏·回响</span>
                  </h1>

                  <p className="text-sm md:text-base text-muted-foreground mb-8 max-w-xl mx-auto lg:mx-0 leading-7">
                    从一个名字、一段烙印开始，让你在字里行间轻推命运。
                  </p>

                  <div className="mb-6 text-left bg-background/35">
                    <p className="text-sm text-foreground/90 leading-relaxed mb-4">
                      这里不是静止的设定页，而是一场会自己呼吸的命运预演。
                      世界、主角与序章会先在你眼前逐步成形，再把你送进真正的故事里。
                    </p>
                    <div className="flex flex-wrap gap-2">
                      <span className="game-chip">世界先被唤醒</span>
                      <span className="game-chip">主角随后成形</span>
                      <span className="game-chip">序章接着落笔</span>
                    </div>
                  </div>
                </div>

                <div className="space-y-3">
                  <motion.button
                    whileHover={{ scale: 1.02 }}
                    whileTap={{ scale: 0.98 }}
                    type="button"
                    onClick={() => {
                      window.location.href = 'https://game.akasa.fun'
                    }}
                    className="w-full sm:w-auto min-w-52 game-btn-primary px-8 py-4 flex items-center justify-center gap-2 mx-auto lg:mx-0"
                  >
                    <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <polygon points="5 3 19 12 5 21 5 3" />
                    </svg>
                    进入回响
                  </motion.button>
                </div>
              </div>

              <div className="game-card p-4 md:p-6 bg-background/30">
                <div className="flex items-center justify-between gap-4 mb-5">
                  <div className="text-left">
                    <p className="text-xs tracking-[0.3em] text-accent/80 mb-1">
                      阿卡夏记录
                    </p>
                    <h2 className="font-serif text-xl md:text-2xl text-foreground">
                      世界生成中
                    </h2>
                    <p className="hidden lg:block text-sm text-muted-foreground mt-2">
                      {/* 在桌面端并排观看世界生成、人物凝聚与序章注入的完整过程。 */}
                    </p>
                  </div>

                  <div className="flex justify-center gap-2">
                    {storySteps.map((_, index) => (
                      <button
                        key={index}
                        onClick={() => setCurrentStep(index)}
                        className={`w-2 h-2 rounded-full transition-all duration-300 ${index === currentStep
                          ? 'w-6 bg-accent'
                          : 'bg-border hover:bg-muted-foreground/50'
                          }`}
                        aria-label={`切换到第 ${index + 1} 步`}
                      />
                    ))}
                  </div>
                </div>

                <div className="grid gap-4 xl:grid-cols-[minmax(0,1.08fr)_minmax(18rem,0.92fr)] xl:items-start">
                  <div className="game-card p-4 text-left md:h-full">
                    <AnimatePresence mode="wait">
                      <motion.div
                        key={currentStep}
                        initial={{ opacity: 0, y: 10 }}
                        animate={{ opacity: 1, y: 0 }}
                        exit={{ opacity: 0, y: -10 }}
                        transition={{ duration: 0.4 }}
                      >
                        <h3 className="font-serif text-2xl text-foreground mb-2">
                          {currentStory.headerTitle}
                        </h3>
                        <p className="text-sm text-muted-foreground mb-6">
                          {currentStory.headerDesc}
                        </p>

                        <div className="game-card p-3 mb-6">
                          <div className="flex md:flex-col items-center justify-between gap-3 text-sm">
                            <span className="text-muted-foreground">{currentStory.statusText}</span>
                            <div className="w-24 h-1.5 game-progress">
                              <motion.div
                                className="game-progress-bar"
                                initial={{ width: 0 }}
                                animate={{ width: `${currentStory.progress}%` }}
                                transition={{ duration: 0.8 }}
                              />
                            </div>
                          </div>
                        </div>

                        <div className="flex flex-wrap gap-2">
                          {currentStory.tags.map((tag, idx) => (
                            <motion.span
                              key={tag.text}
                              initial={{ opacity: 0, scale: 0.9 }}
                              animate={{ opacity: 1, scale: 1 }}
                              transition={{ delay: idx * 0.1 }}
                              className="game-chip"
                            >
                              <span className={tag.color === 'accent' ? 'text-accent' : 'text-primary'}>
                                {tagIcons[tag.icon]}
                              </span>
                              {tag.text}
                            </motion.span>
                          ))}
                        </div>
                      </motion.div>
                    </AnimatePresence>
                  </div>

                  <div className="space-y-3">
                    {storySteps.map((step, index) => (
                      <motion.div
                        key={step.phase}
                        initial={{ opacity: 0, x: -20 }}
                        animate={{ opacity: 1, x: 0 }}
                        transition={{ delay: index * 0.12 }}
                        className={`game-card p-4 transition-all duration-300 cursor-pointer ${index === currentStep
                          ? 'border-accent/40 bg-accent/5'
                          : 'hover:border-border/60'
                          }`}
                        onClick={() => setCurrentStep(index)}
                      >
                        <div className="flex items-start gap-3 text-left">
                          <div className={`shrink-0 w-5 h-5 rounded-full flex items-center justify-center mt-0.5 transition-all duration-300 ${index === currentStep
                            ? 'loading-ring'
                            : index < currentStep
                              ? 'bg-accent/20 border border-accent/40'
                              : 'border border-border'
                            }`}>
                            {index < currentStep && (
                              <svg className="w-3 h-3 text-accent" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3">
                                <path d="M5 12l5 5L20 7" />
                              </svg>
                            )}
                            {index > currentStep && (
                              <div className="w-1.5 h-1.5 rounded-full bg-muted-foreground/30" />
                            )}
                          </div>
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-2 mb-1">
                              <span className={`text-xs transition-colors duration-300 ${index === currentStep ? 'text-accent' : 'text-muted-foreground'
                                }`}>
                                {step.phase}
                              </span>
                            </div>
                            <h4 className={`font-medium mb-1 transition-colors duration-300 ${index === currentStep ? 'text-foreground' : 'text-foreground/70'
                              }`}>
                              {step.title}
                            </h4>
                            <p className="text-sm text-muted-foreground leading-relaxed">
                              {step.description}
                            </p>
                          </div>
                        </div>
                      </motion.div>
                    ))}
                  </div>
                </div>
              </div>
            </div>
          </motion.div>
        </div>

        {/* 向下滚动提示 */}
        <motion.div
          className="absolute bottom-8 left-1/2 -translate-x-1/2"
          animate={{ y: [0, 8, 0] }}
          transition={{ duration: 2, repeat: Infinity }}
        >
          <div className="w-6 h-10 border-2 border-border rounded-full flex justify-center pt-2">
            <div className="w-1 h-2 bg-muted-foreground rounded-full" />
          </div>
        </motion.div>
      </section>

      {/* 特性介绍 */}
      <section className="py-24 px-6">
        <div className="max-w-6xl mx-auto">
          <div className="grid gap-8 lg:grid-cols-[minmax(0,0.7fr)_minmax(0,1.3fr)] lg:items-start">
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              className="text-center lg:text-left"
            >
              <p className="text-xs tracking-[0.3em] text-accent/80 mb-3">
                EXPERIENCE NOTES
              </p>
              <h2 className="font-serif text-2xl md:text-3xl text-foreground mb-3">
                游戏特色
              </h2>
              <p className="text-muted-foreground text-sm leading-7 max-w-md mx-auto lg:mx-0">
                移动端保持轻巧直接，桌面端则把留白、层次和叙事气氛完整铺开，
                让你更像坐在一张命运地图前，静看故事慢慢发亮。
              </p>
            </motion.div>

            <div className="grid md:grid-cols-2 gap-4">
              {features.map((feature, index) => (
                <motion.div
                  key={feature.title}
                  initial={{ opacity: 0, y: 20 }}
                  whileInView={{ opacity: 1, y: 0 }}
                  viewport={{ once: true }}
                  transition={{ delay: index * 0.1 }}
                  className="game-card p-5 flex items-start gap-4 hover:border-accent/30 transition-colors"
                >
                  <div className="shrink-0 w-10 h-10 rounded-lg bg-accent/10 flex items-center justify-center text-accent">
                    {feature.icon}
                  </div>
                  <div>
                    <h3 className="font-medium text-foreground mb-1">
                      {feature.title}
                    </h3>
                    <p className="text-sm text-muted-foreground">
                      {feature.description}
                    </p>
                  </div>
                </motion.div>
              ))}
            </div>
          </div>
        </div>
      </section>
    </div>
  )
}
