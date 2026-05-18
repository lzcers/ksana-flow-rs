import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tsconfigPaths from "vite-tsconfig-paths";
import { traeBadgePlugin } from 'vite-plugin-trae-solo-badge';

// https://vite.dev/config/
export default defineConfig({
  server: {
    proxy: {
      '/api': 'http://127.0.0.1:3001',
      '/healthz': 'http://127.0.0.1:3001',
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
    tsconfigPaths()
  ],
})
