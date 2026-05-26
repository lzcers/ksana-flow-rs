import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  // server: {
  //   allowedHosts: ["nas.ksana.net", "flow.ksana.net"],
  //   host: '0.0.0.0', // 绑定所有网络接口
  //   port: 5173, // 自定义端口（可选，默认 5173）
  //   open: false, // 是否自动打开浏览器（可选）
  // }
  resolve: {
    alias: {
      '@': '/src',
    },
  },
})
