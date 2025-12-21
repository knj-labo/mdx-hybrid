import { defineConfig } from 'astro/config'

export default defineConfig({
  srcDir: 'src',
  integrations: [],
  vite: {
    logLevel: 'error', // suppress noisy warnings in CI logs
    build: {
      rollupOptions: {
        // Silence known benign unused import warning from Astro internal remotePattern.js
        onwarn(warning, defaultHandler) {
          const msg = typeof warning.message === 'string' ? warning.message : ''
          if (
            (warning.code === 'UNUSED_EXTERNAL_IMPORT' ||
              msg.includes('matchHostname') ||
              msg.includes('remotePattern.js')) &&
            typeof warning.id === 'string' &&
            warning.id.includes('remotePattern.js')
          ) {
            return
          }
          defaultHandler(warning)
        },
      },
    },
  },
})
