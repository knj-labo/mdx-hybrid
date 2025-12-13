import { defineConfig } from 'astro/config'
import markflowContent from './plugins/markflow-content-plugin.mjs'
import { markflowPlugin } from '../../../packages/vite-plugin-markflow/src/index.js'

export default defineConfig({
  output: 'static',
  vite: {
    plugins: [markflowPlugin(), markflowContent()],
  },
})
