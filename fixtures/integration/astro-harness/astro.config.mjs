import { defineConfig } from 'astro/config'
import markflowContent from './plugins/markflow-content-plugin.mjs'
import { markflowPlugin } from '../../../packages/astro-markflow/src/vite-plugin.ts'

export default defineConfig({
  output: 'static',
  vite: {
    plugins: [markflowPlugin(), markflowContent()],
  },
})
