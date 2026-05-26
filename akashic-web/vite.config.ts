import { defineConfig } from 'vite'
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import path from 'path'

// https://vite.dev/config/
export default defineConfig({
  server: {
    host: '0.0.0.0',
    proxy: {
      '/api': 'http://127.0.0.1:3001',
    },
  },
  resolve: {
    // 核心配置：让 @ 指向 src 目录
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  build: {
    sourcemap: 'hidden',
  },
  plugins: [
    react({
      // babel: {
      //   plugins: [
      //     'react-dev-locator',
      //   ],
      // },
    }),
    tailwindcss(),

  ],
})
