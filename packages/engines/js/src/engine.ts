import { compile as mdxCompile, compileSync as mdxCompileSync } from '@mdx-js/mdx'
import type { CompileOptions, CompileResult, Engine } from './types.js'

export class JSEngine implements Engine {
  getName(): string {
    return 'js'
  }

  isAvailable(): boolean {
    // JS engine is always available
    return true
  }

  async compile(content: string, options?: CompileOptions): Promise<CompileResult> {
    const startTime = performance.now()

    try {
      const result = await mdxCompile(content, this.transformOptions(options))
      const timing = performance.now() - startTime

      return {
        code: String(result.value),
        map: result.map,
        timing,
      }
    } catch (error) {
      throw new Error(
        `MDX compilation failed: ${error instanceof Error ? error.message : String(error)}`
      )
    }
  }

  compileSync(content: string, options?: CompileOptions): CompileResult {
    const startTime = performance.now()

    try {
      const result = mdxCompileSync(content, this.transformOptions(options))
      const timing = performance.now() - startTime

      return {
        code: String(result.value),
        map: result.map,
        timing,
      }
    } catch (error) {
      throw new Error(
        `MDX compilation failed: ${error instanceof Error ? error.message : String(error)}`
      )
    }
  }

  private transformOptions(options?: CompileOptions): any {
    if (!options) return {}

    const mdxOptions: any = {
      remarkPlugins: options.remarkPlugins,
      rehypePlugins: options.rehypePlugins,
      development: options.development,
      jsx: options.jsx,
      jsxRuntime: options.jsxRuntime,
      jsxImportSource: options.jsxImportSource,
      pragma: options.pragma,
      pragmaFrag: options.pragmaFrag,
      pragmaImportSource: options.pragmaImportSource,
    }

    // Handle output format
    if (options.outputFormat) {
      mdxOptions.outputFormat = options.outputFormat === 'esm' ? 'program' : options.outputFormat
    }

    // Handle sourcemaps
    if (options.sourcemap) {
      // MDX expects a SourceMapGenerator constructor, not a boolean
      // We'll let MDX handle sourcemap generation by omitting this option
      // The sourcemap will be returned in the result if development mode is enabled
    }

    // Remove undefined values
    for (const key of Object.keys(mdxOptions)) {
      if (mdxOptions[key] === undefined) {
        delete mdxOptions[key]
      }
    }

    return mdxOptions
  }
}
