import { RustPanicWarning, RustUnavailableError } from './errors.js'
import type { CompileOptions, Engine } from './types.js'

export class EngineSelector {
  private jsEngine?: Engine
  private rustEngine?: Engine
  private rustChecked = false

  private getJSEngineSync(): Engine {
    if (!this.jsEngine) {
      try {
        // For sync operations, use require with proper module resolution
        const jsEngineModule = require('@mdx-hybrid/engine-js')
        const { JSEngine } = jsEngineModule.JSEngine ? jsEngineModule : jsEngineModule.default || jsEngineModule
        this.jsEngine = new JSEngine()
      } catch (error) {
        console.error('Failed to load JS engine:', error)
        throw new Error('JS engine is not available')
      }
    }
    return this.jsEngine!
  }

  private async getJSEngine(): Promise<Engine> {
    if (!this.jsEngine) {
      try {
        // Dynamically import ESM module
        const jsEngineModule = await import('@mdx-hybrid/engine-js')
        const { JSEngine } = jsEngineModule
        this.jsEngine = new JSEngine()
      } catch (error) {
        console.error('Failed to load JS engine:', error)
        throw new Error('JS engine is not available')
      }
    }
    return this.jsEngine!
  }

  private getRustEngine(): Engine | undefined {
    if (!this.rustChecked) {
      this.rustChecked = true
      try {
        const rustModule = require('@mdx-hybrid/engine-rust')
        if (rustModule.isAvailable?.()) {
          // Create a wrapper class for the Rust engine
          this.rustEngine = {
            getName: () => 'rust',
            isAvailable: () => true,
            compile: async (content: string, options?: CompileOptions) => {
              const rustOptions = this.transformOptionsForRust(options)
              const result = rustModule.compileSync(content, rustOptions)
              return result
            },
            compileSync: (content: string, options?: CompileOptions) => {
              const rustOptions = this.transformOptionsForRust(options)
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
    return this.rustEngine
  }

  selectEngine(options?: CompileOptions): Engine {
    const engineType = options?.engine || 'auto'

    // Case 1: Explicit engine selection
    if (engineType === 'rust') {
      const rustEngine = this.getRustEngine()
      if (!rustEngine) {
        throw new RustUnavailableError()
      }
      return rustEngine
    }

    if (engineType === 'js') {
      return this.getJSEngineSync()
    }

    // Case 2: Auto selection
    // Check if plugins are present (requires JS engine)
    if (options?.remarkPlugins?.length || options?.rehypePlugins?.length) {
      if (process.env.MDX_HYBRID_DEBUG) {
        console.log('Selecting JS engine due to plugin usage')
      }
      return this.getJSEngineSync()
    }

    // Case 3: Prefer Rust engine if available
    const rustEngine = this.getRustEngine()
    if (rustEngine) {
      if (process.env.MDX_HYBRID_DEBUG) {
        console.log('Selecting Rust engine for performance')
      }
      return rustEngine
    }

    // Case 4: Fallback to JS engine
    if (process.env.MDX_HYBRID_DEBUG) {
      console.log('Selecting JS engine as fallback')
    }
    return this.getJSEngineSync()
  }

  async selectEngineAsync(options?: CompileOptions): Promise<Engine> {
    const engineType = options?.engine || 'auto'

    // Case 1: Explicit engine selection
    if (engineType === 'rust') {
      const rustEngine = this.getRustEngine()
      if (!rustEngine) {
        throw new RustUnavailableError()
      }
      return rustEngine
    }

    if (engineType === 'js') {
      return this.getJSEngine()
    }

    // Case 2: Auto selection
    // Check if plugins are present (requires JS engine)
    if (options?.remarkPlugins?.length || options?.rehypePlugins?.length) {
      if (process.env.MDX_HYBRID_DEBUG) {
        console.log('Selecting JS engine due to plugin usage')
      }
      return this.getJSEngine()
    }

    // Case 3: Prefer Rust engine if available
    const rustEngine = this.getRustEngine()
    if (rustEngine) {
      if (process.env.MDX_HYBRID_DEBUG) {
        console.log('Selecting Rust engine for performance')
      }
      return rustEngine
    }

    // Case 4: Fallback to JS engine
    if (process.env.MDX_HYBRID_DEBUG) {
      console.log('Selecting JS engine as fallback')
    }
    return this.getJSEngine()
  }

  private transformOptionsForRust(options?: CompileOptions): any {
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

  async compileWithFallback(
    content: string,
    options?: CompileOptions
  ): Promise<{ result: any; engine: Engine; warning?: RustPanicWarning }> {
    const selectedEngine = await this.selectEngineAsync(options)

    try {
      const result = await selectedEngine.compile(content, options)
      return { result, engine: selectedEngine }
    } catch (error) {
      // If Rust engine fails and we're in auto mode, try falling back to JS
      if (selectedEngine.getName() === 'rust' && (!options?.engine || options.engine === 'auto')) {
        const jsEngine = await this.getJSEngine()
        if (process.env.MDX_HYBRID_DEBUG) {
          console.log('Rust engine failed, falling back to JS:', error)
        }
        const warning = new RustPanicWarning(error as Error)
        const result = await jsEngine.compile(content, options)
        return { result, engine: jsEngine, warning }
      }
      throw error
    }
  }

  compileWithFallbackSync(
    content: string,
    options?: CompileOptions
  ): { result: any; engine: Engine; warning?: RustPanicWarning } {
    const selectedEngine = this.selectEngine(options)

    try {
      const result = selectedEngine.compileSync(content, options)
      return { result, engine: selectedEngine }
    } catch (error) {
      // If Rust engine fails and we're in auto mode, try falling back to JS
      if (selectedEngine.getName() === 'rust' && (!options?.engine || options.engine === 'auto')) {
        const jsEngine = this.getJSEngineSync()
        if (process.env.MDX_HYBRID_DEBUG) {
          console.log('Rust engine failed, falling back to JS:', error)
        }
        const warning = new RustPanicWarning(error as Error)
        const result = jsEngine.compileSync(content, options)
        return { result, engine: jsEngine, warning }
      }
      throw error
    }
  }
}