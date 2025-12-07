import { defineConfig } from 'astro/config'
import markflowContent from './plugins/markflow-content-plugin.mjs'

export default defineConfig({
  output: 'static',
  vite: {
    plugins: [markflowContent()],
  },
})
