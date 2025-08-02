import { defineConfig } from 'astro/config'
import react from '@astrojs/react'
import { mdxHybrid } from '@mdx-hybrid/core/astro'

export default defineConfig({
  integrations: [
    react(),
    mdxHybrid({
      // mdx-hybrid will auto-select the best engine
      // You can force a specific engine:
      // engine: 'rust', // or 'js'
    })
  ]
})