import { Outlet, Link, useLocation } from 'react-router-dom'
import { motion, AnimatePresence } from 'framer-motion'
import { useState } from 'react'

const navItems = [
  { path: '/', label: '首页' },
  { path: '/feedback', label: '反馈' },
  { path: '/changelog', label: '更新日志' },
]

export default function Layout() {
  const location = useLocation()
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)

  return (
    <div className="min-h-screen bg-background">
      {/* 背景微光 */}
      <div className="fixed inset-0 pointer-events-none overflow-hidden">
        <div className="absolute top-0 left-1/3 w-[600px] h-[400px] bg-accent/3 rounded-full blur-[150px]" />
        <div className="absolute bottom-1/4 right-1/4 w-[500px] h-[300px] bg-primary/3 rounded-full blur-[150px]" />
      </div>

      {/* 导航栏 */}
      <header className="fixed top-0 left-0 right-0 z-50 border-b border-border/50 bg-background/80 backdrop-blur-xl">
        <nav className="max-w-6xl mx-auto px-6 h-14 lg:h-16 flex items-center justify-between gap-6">
          <Link to="/" className="flex items-center gap-2">
            <img
              src="/logo.svg"
              alt="阿卡夏笔记图标"
              className="w-7 h-7 shrink-0"
            />
            <div>
              <span className="block font-serif text-base tracking-wider text-foreground">
                阿卡夏·回响
              </span>
              <span className="hidden lg:block text-[11px] tracking-[0.28em] text-muted-foreground/70">
                INTERACTIVE DESTINY
              </span>
            </div>
          </Link>

          {/* 桌面导航 */}
          <div className="hidden md:flex items-center gap-1">
            {navItems.map((item) => (
              <Link
                key={item.path}
                to={item.path}
                className={`relative px-4 py-2 text-sm rounded-lg transition-colors ${location.pathname === item.path
                  ? 'text-primary bg-primary/10'
                  : 'text-muted-foreground hover:text-foreground hover:bg-card/50'
                  }`}
              >
                {item.label}
              </Link>
            ))}
          </div>

          {/* 移动端菜单按钮 */}
          <button
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
            className="md:hidden p-2 text-muted-foreground hover:text-foreground"
            aria-label="Toggle menu"
          >
            <svg
              className="w-5 h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              {mobileMenuOpen ? (
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={1.5}
                  d="M6 18L18 6M6 6l12 12"
                />
              ) : (
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={1.5}
                  d="M4 6h16M4 12h16M4 18h16"
                />
              )}
            </svg>
          </button>
        </nav>

        {/* 移动端菜单 */}
        <AnimatePresence>
          {mobileMenuOpen && (
            <motion.div
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: 'auto' }}
              exit={{ opacity: 0, height: 0 }}
              className="md:hidden border-t border-border/50 bg-background/95 backdrop-blur-xl"
            >
              <div className="px-6 py-3 space-y-1">
                {navItems.map((item) => (
                  <Link
                    key={item.path}
                    to={item.path}
                    onClick={() => setMobileMenuOpen(false)}
                    className={`block py-3 px-4 rounded-lg text-sm ${location.pathname === item.path
                      ? 'text-primary bg-primary/10'
                      : 'text-muted-foreground'
                      }`}
                  >
                    {item.label}
                  </Link>
                ))}
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </header>

      {/* 主内容 */}
      <main className="pt-14 lg:pt-16">
        <AnimatePresence mode="wait">
          <motion.div
            key={location.pathname}
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -20 }}
            transition={{ duration: 0.3 }}
          >
            <Outlet />
          </motion.div>
        </AnimatePresence>
      </main>

      {/* 页脚 */}
      <footer className="border-t border-border/50 bg-card/30">
        <div className="max-w-6xl mx-auto px-6 py-10 lg:py-12">
          <div className="flex flex-col gap-5 lg:flex-row lg:items-center lg:justify-between">
            <div className="flex items-center gap-2">
              <img
                src="/logo.svg"
                alt="阿卡夏笔记图标"
                className="w-6 h-6 shrink-0"
              />
              <div>
                <span className="block font-serif text-sm tracking-wider text-foreground">
                  阿卡夏·回响
                </span>
              </div>
            </div>
            <div className="flex flex-col gap-2 text-xs text-muted-foreground lg:items-end">
              <p>© 2026 阿卡夏工作室. 保留所有权利.</p>
              <div className="hidden lg:flex items-center gap-2 text-[11px] tracking-[0.28em] text-muted-foreground/60">
                <span className="h-px w-10 bg-linear-to-r from-transparent to-border/80" />
              </div>
            </div>
          </div>
        </div>
      </footer>
    </div>
  )
}
