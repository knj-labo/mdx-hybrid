import path from 'node:path'

const DEFAULT_EXTENSIONS = new Set(['.md', '.mdx'])
let bindingPromise

async function loadMarkflowBinding() {
  if (!bindingPromise) {
    bindingPromise = import('markflow-napi').catch(async (error) => {
      if (error?.code !== 'ERR_MODULE_NOT_FOUND') {
        throw error
      }
      const fallbackUrl = new URL('../../../crates/napi/index.js', import.meta.url)
      return import(fallbackUrl)
    })
  }
  return bindingPromise
}

const stripQuery = (id) => {
  const queryIndex = id.indexOf('?')
  return queryIndex >= 0 ? id.slice(0, queryIndex) : id
}

const normalizePath = (value) => value.split(path.sep).join('/')

function deriveAstroUrl(filePath, rootDir) {
  if (!filePath) return undefined
  const normalizedFile = normalizePath(filePath)
  const root = rootDir ?? process.cwd()
  const pagesDir = normalizePath(path.join(root, 'src', 'pages'))
  if (!normalizedFile.startsWith(pagesDir)) {
    return undefined
  }
  let relative = normalizedFile.slice(pagesDir.length)
  if (relative.startsWith('/')) {
    relative = relative.slice(1)
  }
  if (!relative) {
    return '/'
  }
  if (relative.endsWith('.md') || relative.endsWith('.mdx')) {
    relative = relative.replace(/\.mdx?$/, '')
  }
  if (relative === '' || relative === 'index') {
    return '/'
  }
  if (relative.endsWith('/index')) {
    relative = relative.slice(0, -'/index'.length)
  }
  return `/${relative}`
}

function deriveFileOptions(id, rootDir) {
  const sourcePath = stripQuery(id)
  let absolutePath = sourcePath
  if (rootDir && !path.isAbsolute(sourcePath)) {
    absolutePath = path.resolve(rootDir, sourcePath)
  }
  const url = deriveAstroUrl(absolutePath, rootDir)
  const options = { file: absolutePath }
  if (url) {
    options.url = url
  }
  return options
}

const shouldCompile = (id) => {
  const cleanPath = stripQuery(id)
  return DEFAULT_EXTENSIONS.has(path.extname(cleanPath))
}

/**
 * Creates the Markflow Vite plugin that intercepts `.md`/`.mdx` files
 * before `@astrojs/mdx` runs.
 */
export function markflowPlugin(userOptions = {}) {
  let compiler
  let resolvedConfig

  const compilerOptions = userOptions.compiler ?? null
  const include = userOptions.include ?? shouldCompile

  return {
    name: 'vite-plugin-markflow',
    enforce: 'pre',
    async configResolved(config) {
      resolvedConfig = config
      const binding = await loadMarkflowBinding()
      const createCompiler = binding.createCompiler
        ? binding.createCompiler
        : (cfg) => new binding.MarkflowCompiler(cfg)
      compiler = createCompiler(compilerOptions)
    },
    transform(code, id) {
      if (!include(id)) {
        return null
      }
      if (!compiler) {
        throw new Error('Markflow compiler has not been initialized')
      }
      const fileOptions = deriveFileOptions(id, resolvedConfig?.root)
      const result = compiler.compile(code, stripQuery(id), fileOptions)
      if (Array.isArray(result?.imports)) {
        for (const dep of result.imports) {
          if (dep?.path) {
            this.addWatchFile(dep.path)
          }
        }
      }
      return {
        code: result.code,
        map: result.map ?? undefined,
      }
    },
  }
}
