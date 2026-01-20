import { defineConfig } from 'astro/config'
import { fileURLToPath } from 'node:url'
import { resolve, dirname } from 'node:path'
import mdx from '@astrojs/mdx'
import markflowContent from './plugins/markflow-content-plugin.mjs'
import { markflowPlugin } from '../../../packages/astro-markflow/src/vite-plugin.ts'

const __dirname = dirname(fileURLToPath(import.meta.url))

export default defineConfig({
  output: 'static',
  // Add @astrojs/mdx integration to handle files that markflow can't compile
  // (files with import/export statements that are delegated via resolveId returning null)
  integrations: [mdx()],
  vite: {
    plugins: [markflowPlugin({ starlightComponents: true }), markflowContent()],
    resolve: {
      alias: {
        // Map Starlight components to local mock components for testing
        '@astrojs/starlight/components': resolve(__dirname, './src/components/starlight-shim.js'),
      },
    },
  },
})
