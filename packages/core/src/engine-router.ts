import { createRequire } from 'node:module'
import { RustPanicWarning, RustUnavailableError } from './errors.js'
import type { CompileOptions, Engine } from './types.js'

const require = createRequire(import.meta.url)

/**
 * Creates an engine router that routes MDX compilation to the appropriate engine
 * @returns Engine router with methods for engine routing and compilation
 */
export function createEngineRouter() {
  // Private state
  let jsEngine: Engine | undefined
  let rustEngine: Engine | undefined
  let rustChecked = false

  /**
   * Gets the JavaScript engine synchronously
   * @returns The JavaScript engine instance
   * @throws Error if JS engine cannot be loaded
   */
  const getJSEngineSync = (): Engine => {
    if (!jsEngine) {
      // In ESM-only mode, we can't use sync loading for JS engine
      throw new Error('Synchronous JS engine loading is not supported in ESM-only mode. Use async methods instead.')
    }
    return jsEngine!
  }

  /**
   * Gets the JavaScript engine asynchronously
   * @returns Promise resolving to the JavaScript engine instance
   * @throws Error if JS engine cannot be loaded
   */
  const getJSEngine = async (): Promise<Engine> => {
    if (!jsEngine) {
      try {
        // Dynamically import ESM module
        const jsEngineModule = await import('@mdx-hybrid/engine-js')
        const { createJSEngine } = jsEngineModule
        jsEngine = createJSEngine()
      } catch (error) {
        console.error('Failed to load JS engine:', error)
        throw new Error('JS engine is not available')
      }
    }
    return jsEngine!
  }

  /**
   * Gets the Rust engine if available (async)
   * @returns Promise resolving to the Rust engine instance or undefined if not available
   */
  const getRustEngineAsync = async (): Promise<Engine | undefined> => {
    if (!rustChecked) {
      rustChecked = true
      try {
        // Use createRequire to load CommonJS module in ESM context
        const rustModule = require('@mdx-hybrid/engine-rust')
        if (rustModule.isAvailable?.()) {
          // Create a wrapper for the Rust engine
          rustEngine = {
            getName: () => 'rust',
            isAvailable: () => true,
            compile: async (content: string, options?: CompileOptions) => {
              const rustOptions = transformOptionsForRust(options)
              const result = rustModule.compileSync(content, rustOptions)
              return result
            },
            compileSync: (content: string, options?: CompileOptions) => {
              const rustOptions = transformOptionsForRust(options)
              return rustModule.compileSync(content, rustOptions)
            },
          }
        }
      } catch (error) {
        // Rust engine is optional, so we just note it's not available
        if (process.env.MDX_HYBRID_DEBUG) {
          console.log('Rust engine not available:', error)
        }
      }
    }
    return rustEngine
  }

  /**
   * Gets the Rust engine if available (sync)
   * @returns The Rust engine instance or undefined if not available
   */
  const getRustEngine = (): Engine | undefined => {
    // For sync access, return already loaded engine or undefined
    return rustEngine
  }

  /**
   * Transforms compile options to Rust engine format
   * @param options - Compile options
   * @returns Transformed options for Rust engine
   */
  const transformOptionsForRust = (options?: CompileOptions): any => {
    if (!options) return undefined

    return {
      development: options.development,
      jsx: options.jsx,
      jsx_runtime: options.jsxRuntime,
      jsx_import_source: options.jsxImportSource,
      pragma: options.pragma,
      pragma_frag: options.pragmaFrag,
      pragma_import_source: options.pragmaImportSource,
    }
  }

  /**
   * Routes to the appropriate engine based on options (synchronous)
   * @param options - Compile options
   * @returns Selected engine instance
   * @throws RustUnavailableError if Rust engine is explicitly requested but not available
   */
  const routeEngine = (options?: CompileOptions): Engine => {
    // In ESM-only mode, we can only use engines that have been pre-loaded via async methods
    const engineType = options?.engine || 'auto'

    // Case 1: Explicit engine selection
    if (engineType === 'rust') {
      const engine = getRustEngine()
      if (!engine) {
        throw new RustUnavailableError('Rust engine not loaded. Use async methods to load engines first.')
      }
      return engine
    }

    if (engineType === 'js') {
      return getJSEngineSync()
    }

    // Case 2: Auto selection
    // Check if plugins are present (requires JS engine)
    if (options?.remarkPlugins?.length || options?.rehypePlugins?.length) {
      if (process.env.MDX_HYBRID_DEBUG) {
        console.log('Routing to JS engine due to plugin usage')
      }
      return getJSEngineSync()
    }

    // Case 3: Prefer Rust engine if available
    const engine = getRustEngine()
    if (engine) {
      if (process.env.MDX_HYBRID_DEBUG) {
        console.log('Routing to Rust engine for performance')
      }
      return engine
    }

    // Case 4: Fallback to JS engine
    if (process.env.MDX_HYBRID_DEBUG) {
      console.log('Routing to JS engine as fallback')
    }
    return getJSEngineSync()
  }

  /**
   * Routes to the appropriate engine based on options (asynchronous)
   * @param options - Compile options
   * @returns Promise resolving to selected engine instance
   * @throws RustUnavailableError if Rust engine is explicitly requested but not available
   */
  const routeEngineAsync = async (options?: CompileOptions): Promise<Engine> => {
    const engineType = options?.engine || 'auto'

    // Case 1: Explicit engine selection
    if (engineType === 'rust') {
      const engine = await getRustEngineAsync()
      if (!engine) {
        throw new RustUnavailableError()
      }
      return engine
    }

    if (engineType === 'js') {
      return getJSEngine()
    }

    // Case 2: Auto selection
    // Check if plugins are present (requires JS engine)
    if (options?.remarkPlugins?.length || options?.rehypePlugins?.length) {
      if (process.env.MDX_HYBRID_DEBUG) {
        console.log('Routing to JS engine due to plugin usage')
      }
      return getJSEngine()
    }

    // Case 3: Prefer Rust engine if available
    const engine = await getRustEngineAsync()
    if (engine) {
      if (process.env.MDX_HYBRID_DEBUG) {
        console.log('Routing to Rust engine for performance')
      }
      return engine
    }

    // Case 4: Fallback to JS engine
    if (process.env.MDX_HYBRID_DEBUG) {
      console.log('Routing to JS engine as fallback')
    }
    return getJSEngine()
  }

  /**
   * Compiles MDX content with automatic fallback from Rust to JS engine on failure
   * @param content - MDX content to compile
   * @param options - Compile options
   * @returns Promise resolving to compilation result with engine info and optional warning
   */
  const compileWithFallback = async (
    content: string,
    options?: CompileOptions
  ): Promise<{ result: any; engine: Engine; warning?: RustPanicWarning }> => {
    const selectedEngine = await routeEngineAsync(options)

    try {
      const result = await selectedEngine.compile(content, options)
      return { result, engine: selectedEngine }
    } catch (error) {
      // If Rust engine fails and we're in auto mode, try falling back to JS
      if (selectedEngine.getName() === 'rust' && (!options?.engine || options.engine === 'auto')) {
        const engine = await getJSEngine()
        if (process.env.MDX_HYBRID_DEBUG) {
          console.log('Rust engine failed, falling back to JS:', error)
        }
        const warning = new RustPanicWarning(error as Error)
        const result = await engine.compile(content, options)
        return { result, engine, warning }
      }
      throw error
    }
  }

  /**
   * Compiles MDX content synchronously with automatic fallback from Rust to JS engine on failure
   * @param content - MDX content to compile
   * @param options - Compile options
   * @returns Compilation result with engine info and optional warning
   */
  const compileWithFallbackSync = (
    content: string,
    options?: CompileOptions
  ): { result: any; engine: Engine; warning?: RustPanicWarning } => {
    const selectedEngine = routeEngine(options)

    try {
      const result = selectedEngine.compileSync(content, options)
      return { result, engine: selectedEngine }
    } catch (error) {
      // If Rust engine fails and we're in auto mode, try falling back to JS
      if (selectedEngine.getName() === 'rust' && (!options?.engine || options.engine === 'auto')) {
        const engine = getJSEngineSync()
        if (process.env.MDX_HYBRID_DEBUG) {
          console.log('Rust engine failed, falling back to JS:', error)
        }
        const warning = new RustPanicWarning(error as Error)
        const result = engine.compileSync(content, options)
        return { result, engine, warning }
      }
      throw error
    }
  }

  // Return public API
  return {
    routeEngine,
    routeEngineAsync,
    compileWithFallback,
    compileWithFallbackSync,
  }
}