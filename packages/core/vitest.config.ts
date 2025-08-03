import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    server: {
      deps: {
        inline: ['@jp-knj/mdx-hybrid-engine-js', '@mdx-js/mdx'],
      },
    },
  },
  resolve: {
    conditions: ['node', 'import', 'module', 'default'],
  },
})
