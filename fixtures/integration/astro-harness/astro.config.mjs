import { defineConfig } from 'astro/config'
import mdx from '@astrojs/mdx'
import markflowContent from './plugins/markflow-content-plugin.mjs'
import { markflowPlugin } from '../../../packages/astro-markflow/src/vite-plugin.ts'

export default defineConfig({
  output: 'static',
  // Add @astrojs/mdx integration to handle files that markflow can't compile
  // (files with import/export statements that are delegated via resolveId returning null)
  integrations: [mdx()],
  vite: {
    plugins: [markflowPlugin(), markflowContent()],
  },
})
