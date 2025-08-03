import { compile as mdxCompile, compileSync as mdxCompileSync } from '@mdx-js/mdx'
import type { CompileOptions, CompileResult, Engine } from './types.js'

/**
 * Creates a JavaScript MDX engine
 * @returns Engine instance for JavaScript-based MDX compilation
 */
export function createJSEngine(): Engine {
  /**
   * Transforms compile options to @mdx-js/mdx format
   * @param options - Compile options
   * @returns Transformed options for @mdx-js/mdx
   */
  const transformOptions = (options?: CompileOptions): any => {
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

  return {
    /**
     * Gets the engine name
     * @returns 'js' for JavaScript engine
     */
    getName(): string {
      return 'js'
    },

    /**
     * Checks if the engine is available
     * @returns Always true for JS engine
     */
    isAvailable(): boolean {
      // JS engine is always available
      return true
    },

    /**
     * Compiles MDX content asynchronously
     * @param content - MDX content to compile
     * @param options - Compile options
     * @returns Promise resolving to compilation result
     * @throws Error if compilation fails
     */
    async compile(content: string, options?: CompileOptions): Promise<CompileResult> {
      const startTime = performance.now()

      try {
        const result = await mdxCompile(content, transformOptions(options))
        const timing = performance.now() - startTime

        return {
          code: String(result.value),
          map: result.map,
          timing,
          data: result.data,
        }
      } catch (error) {
        throw new Error(
          `MDX compilation failed: ${error instanceof Error ? error.message : String(error)}`
        )
      }
    },

    /**
     * Compiles MDX content synchronously
     * @param content - MDX content to compile
     * @param options - Compile options
     * @returns Compilation result
     * @throws Error if compilation fails
     */
    compileSync(content: string, options?: CompileOptions): CompileResult {
      const startTime = performance.now()

      try {
        const result = mdxCompileSync(content, transformOptions(options))
        const timing = performance.now() - startTime

        return {
          code: String(result.value),
          map: result.map,
          timing,
          data: result.data,
        }
      } catch (error) {
        throw new Error(
          `MDX compilation failed: ${error instanceof Error ? error.message : String(error)}`
        )
      }
    },
  }
}
