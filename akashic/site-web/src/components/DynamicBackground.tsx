import { useEffect, useRef } from 'react'

interface Particle {
  x: number
  y: number
  vx: number
  vy: number
  size: number
  opacity: number
  r: number
  g: number
  b: number
}

interface FloatingOrb {
  x: number
  y: number
  size: number
  speed: number
  angle: number
  r: number
  g: number
  b: number
  alpha: number
}

export default function DynamicBackground() {
  const canvasRef = useRef<HTMLCanvasElement>(null)

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return

    const ctx = canvas.getContext('2d')
    if (!ctx) return

    let animationId: number
    let particles: Particle[] = []
    let orbs: FloatingOrb[] = []
    let mouseX = 0
    let mouseY = 0

    const resize = () => {
      canvas.width = window.innerWidth
      canvas.height = window.innerHeight
    }

    const initParticles = () => {
      particles = []
      // 增加粒子数量到 150-200
      const count = Math.min(200, Math.max(150, Math.floor((canvas.width * canvas.height) / 8000)))

      for (let i = 0; i < count; i++) {
        // 金色: rgb(212, 165, 116) 或 青色: rgb(74, 158, 173)
        const isGold = Math.random() > 0.6
        // 给每个粒子一个基础速度，让它们持续运动
        const angle = Math.random() * Math.PI * 2
        const speed = Math.random() * 0.8 + 0.3
        particles.push({
          x: Math.random() * canvas.width,
          y: Math.random() * canvas.height,
          vx: Math.cos(angle) * speed,
          vy: Math.sin(angle) * speed,
          size: Math.random() * 2.5 + 0.5,
          opacity: Math.random() * 0.6 + 0.2,
          r: isGold ? 212 : 74,
          g: isGold ? 165 : 158,
          b: isGold ? 116 : 173,
        })
      }
    }

    const initOrbs = () => {
      orbs = [
        { x: canvas.width * 0.3, y: canvas.height * 0.3, size: 300, speed: 0.0003, angle: 0, r: 74, g: 158, b: 173, alpha: 0.08 },
        { x: canvas.width * 0.7, y: canvas.height * 0.5, size: 250, speed: 0.0005, angle: Math.PI, r: 212, g: 165, b: 116, alpha: 0.06 },
        { x: canvas.width * 0.5, y: canvas.height * 0.7, size: 200, speed: 0.0004, angle: Math.PI / 2, r: 74, g: 158, b: 173, alpha: 0.05 },
      ]
    }

    const drawOrbs = () => {
      orbs.forEach((orb) => {
        orb.angle += orb.speed
        const offsetX = Math.sin(orb.angle) * 50
        const offsetY = Math.cos(orb.angle * 0.7) * 30

        const gradient = ctx.createRadialGradient(
          orb.x + offsetX,
          orb.y + offsetY,
          0,
          orb.x + offsetX,
          orb.y + offsetY,
          orb.size
        )
        gradient.addColorStop(0, `rgba(${orb.r}, ${orb.g}, ${orb.b}, ${orb.alpha})`)
        gradient.addColorStop(1, 'transparent')

        ctx.fillStyle = gradient
        ctx.beginPath()
        ctx.arc(orb.x + offsetX, orb.y + offsetY, orb.size, 0, Math.PI * 2)
        ctx.fill()
      })
    }

    const drawParticles = () => {
      particles.forEach((p, i) => {
        // 更新位置
        p.x += p.vx
        p.y += p.vy

        // 鼠标交互 - 轻微排斥
        const dx = p.x - mouseX
        const dy = p.y - mouseY
        const dist = Math.sqrt(dx * dx + dy * dy)
        if (dist < 150 && dist > 0) {
          const force = (150 - dist) / 150 * 0.02
          p.vx += (dx / dist) * force
          p.vy += (dy / dist) * force
        }

        // 边界处理
        if (p.x < 0) p.x = canvas.width
        if (p.x > canvas.width) p.x = 0
        if (p.y < 0) p.y = canvas.height
        if (p.y > canvas.height) p.y = 0

        // 速度衰减
        p.vx *= 0.99
        p.vy *= 0.99

        // 绘制粒子
        ctx.beginPath()
        ctx.arc(p.x, p.y, p.size, 0, Math.PI * 2)
        ctx.fillStyle = `rgba(${p.r}, ${p.g}, ${p.b}, ${p.opacity})`
        ctx.fill()

        // 连接附近粒子
        for (let j = i + 1; j < particles.length; j++) {
          const p2 = particles[j]
          const lineDx = p.x - p2.x
          const lineDy = p.y - p2.y
          const lineDist = Math.sqrt(lineDx * lineDx + lineDy * lineDy)

          if (lineDist < 120) {
            ctx.beginPath()
            ctx.moveTo(p.x, p.y)
            ctx.lineTo(p2.x, p2.y)
            ctx.strokeStyle = `rgba(74, 158, 173, ${0.15 * (1 - lineDist / 120)})`
            ctx.lineWidth = 0.5
            ctx.stroke()
          }
        }
      })
    }

    const drawGrid = () => {
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.025)'
      ctx.lineWidth = 1

      const gridSize = 60
      const time = Date.now() * 0.0001

      for (let x = 0; x < canvas.width; x += gridSize) {
        const offset = Math.sin(time + x * 0.01) * 2
        ctx.beginPath()
        ctx.moveTo(x, 0)
        ctx.lineTo(x + offset, canvas.height)
        ctx.stroke()
      }

      for (let y = 0; y < canvas.height; y += gridSize) {
        const offset = Math.cos(time + y * 0.01) * 2
        ctx.beginPath()
        ctx.moveTo(0, y)
        ctx.lineTo(canvas.width, y + offset)
        ctx.stroke()
      }
    }

    const drawLightBeams = () => {
      const time = Date.now() * 0.001

      // 从顶部射下的光束
      for (let i = 0; i < 3; i++) {
        const x = canvas.width * (0.2 + i * 0.3) + Math.sin(time + i) * 50
        const gradient = ctx.createLinearGradient(x, 0, x + 100, canvas.height)
        gradient.addColorStop(0, 'rgba(212, 165, 116, 0.04)')
        gradient.addColorStop(0.5, 'rgba(212, 165, 116, 0.015)')
        gradient.addColorStop(1, 'transparent')

        ctx.fillStyle = gradient
        ctx.beginPath()
        ctx.moveTo(x, 0)
        ctx.lineTo(x + 150, canvas.height)
        ctx.lineTo(x + 50, canvas.height)
        ctx.closePath()
        ctx.fill()
      }
    }

    const animate = () => {
      ctx.clearRect(0, 0, canvas.width, canvas.height)

      drawLightBeams()
      drawOrbs()
      drawGrid()
      drawParticles()

      animationId = requestAnimationFrame(animate)
    }

    const handleMouseMove = (e: MouseEvent) => {
      mouseX = e.clientX
      mouseY = e.clientY
    }

    resize()
    initParticles()
    initOrbs()
    animate()

    window.addEventListener('resize', () => {
      resize()
      initParticles()
      initOrbs()
    })
    window.addEventListener('mousemove', handleMouseMove)

    return () => {
      cancelAnimationFrame(animationId)
      window.removeEventListener('resize', resize)
      window.removeEventListener('mousemove', handleMouseMove)
    }
  }, [])

  return (
    <canvas
      ref={canvasRef}
      className="absolute inset-0"
      style={{ zIndex: 1 }}
    />
  )
}
