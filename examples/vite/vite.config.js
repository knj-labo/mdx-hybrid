import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { mdxHybrid } from '@mdx-hybrid/vite'

export default defineConfig({
  plugins: [
    react(),
    mdxHybrid({
      // mdx-hybrid will auto-select the best engine
      // You can force a specific engine:
      // engine: 'rust', // or 'js'
    })
  ]
})