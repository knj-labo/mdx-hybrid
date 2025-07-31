import type { Plugin } from 'vite'
import { compile } from './compiler.js'
import type { VitePluginOptions } from './types.js'

export function mdxHybrid(options: VitePluginOptions = {}): Plugin {
  const { include = /\.mdx?$/, exclude, ...compileOptions } = options

  const filter = (id: string): boolean => {
    if (exclude) {
      const excludeList = Array.isArray(exclude) ? exclude : [exclude]
      if (excludeList.some((pattern) => pattern.test(id))) {
        return false
      }
    }

    const includeList = Array.isArray(include) ? include : [include]
    return includeList.some((pattern) => pattern.test(id))
  }

  return {
    name: 'mdx-hybrid',
    enforce: 'pre',

    async transform(code: string, id: string) {
      if (!filter(id)) return

      try {
        const result = await compile(code, {
          ...compileOptions,
          development: process.env.NODE_ENV !== 'production',
          sourcemap: true,
        })

        return {
          code: result.code,
          map: result.map,
        }
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error)
        this.error(`MDX compilation failed for ${id}: ${message}`)
      }
    },

    handleHotUpdate({ file, server }) {
      if (filter(file)) {
        // Trigger HMR for MDX files
        server.ws.send({
          type: 'full-reload',
          path: '*',
        })
      }
    },
  }
}
