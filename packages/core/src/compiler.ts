import { EngineSelector } from './engine-selector.js'
import { CompilerError } from './errors.js'
import type { CompileOptions, CompileResult } from './types.js'

export class Compiler {
  private engineSelector: EngineSelector

  constructor() {
    this.engineSelector = new EngineSelector()
  }

  /**
   * Compile MDX to JavaScript asynchronously
   */
  async compile(content: string, options?: CompileOptions): Promise<CompileResult> {
    try {
      const { result, warning } = await this.engineSelector.compileWithFallback(content, options)

      if (warning && process.env.MDX_HYBRID_DEBUG) {
        console.warn(warning.message)
      }

      return result
    } catch (error) {
      throw new CompilerError(
        `Failed to compile MDX: ${error instanceof Error ? error.message : String(error)}`,
        'js', // Default to JS since we couldn't determine the engine in error case
        error instanceof Error ? error : undefined
      )
    }
  }

  /**
   * Compile MDX to JavaScript synchronously
   */
  compileSync(content: string, options?: CompileOptions): CompileResult {
    try {
      const { result, warning } = this.engineSelector.compileWithFallbackSync(content, options)

      if (warning && process.env.MDX_HYBRID_DEBUG) {
        console.warn(warning.message)
      }

      return result
    } catch (error) {
      throw new CompilerError(
        `Failed to compile MDX: ${error instanceof Error ? error.message : String(error)}`,
        'js', // Default to JS since we couldn't determine the engine in error case
        error instanceof Error ? error : undefined
      )
    }
  }

  /**
   * Get information about available engines
   */
  getEngineInfo(): { js: boolean; rust: boolean } {
    let jsAvailable = false
    let rustAvailable = false

    try {
      this.engineSelector.selectEngine({ engine: 'js' })
      jsAvailable = true
    } catch {
      // JS not available
    }

    try {
      this.engineSelector.selectEngine({ engine: 'rust' })
      rustAvailable = true
    } catch {
      // Rust not available
    }

    return {
      js: jsAvailable,
      rust: rustAvailable,
    }
  }
}

// Convenience functions
let defaultCompiler: Compiler | undefined

function getDefaultCompiler(): Compiler {
  if (!defaultCompiler) {
    defaultCompiler = new Compiler()
  }
  return defaultCompiler
}

/**
 * Compile MDX to JavaScript asynchronously using the default compiler
 */
export async function compile(content: string, options?: CompileOptions): Promise<CompileResult> {
  return getDefaultCompiler().compile(content, options)
}

/**
 * Compile MDX to JavaScript synchronously using the default compiler
 */
export function compileSync(content: string, options?: CompileOptions): CompileResult {
  return getDefaultCompiler().compileSync(content, options)
}
