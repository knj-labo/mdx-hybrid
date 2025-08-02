import { createEngineSelector } from './engine-selector.js'
import { CompilerError } from './errors.js'
import type { CompileOptions, CompileResult } from './types.js'

/**
 * Creates a compiler instance for MDX compilation
 * @returns Compiler with compile methods and engine info
 */
export function createCompiler() {
  const engineSelector = createEngineSelector()

  return {
    /**
     * Compile MDX to JavaScript asynchronously
     * @param content - MDX content to compile
     * @param options - Compile options
     * @returns Promise resolving to compilation result
     * @throws CompilerError if compilation fails
     */
    async compile(content: string, options?: CompileOptions): Promise<CompileResult> {
      try {
        const { result, warning } = await engineSelector.compileWithFallback(content, options)

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
    },

    /**
     * Compile MDX to JavaScript synchronously
     * @param content - MDX content to compile
     * @param options - Compile options
     * @returns Compilation result
     * @throws CompilerError if compilation fails
     */
    compileSync(content: string, options?: CompileOptions): CompileResult {
      try {
        const { result, warning } = engineSelector.compileWithFallbackSync(content, options)

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
    },

    /**
     * Get information about available engines
     * @returns Object indicating availability of JS and Rust engines
     */
    getEngineInfo(): { js: boolean; rust: boolean } {
      let jsAvailable = false
      let rustAvailable = false

      try {
        engineSelector.selectEngine({ engine: 'js' })
        jsAvailable = true
      } catch {
        // JS not available
      }

      try {
        engineSelector.selectEngine({ engine: 'rust' })
        rustAvailable = true
      } catch {
        // Rust not available
      }

      return {
        js: jsAvailable,
        rust: rustAvailable,
      }
    },
  }
}

// Convenience functions
let defaultCompiler: ReturnType<typeof createCompiler> | undefined

/**
 * Gets the default compiler instance (singleton)
 * @returns Default compiler instance
 */
function getDefaultCompiler(): ReturnType<typeof createCompiler> {
  if (!defaultCompiler) {
    defaultCompiler = createCompiler()
  }
  return defaultCompiler
}

/**
 * Compile MDX to JavaScript asynchronously using the default compiler
 * @param content - MDX content to compile
 * @param options - Compile options
 * @returns Promise resolving to compilation result
 * @throws CompilerError if compilation fails
 */
export async function compile(content: string, options?: CompileOptions): Promise<CompileResult> {
  return getDefaultCompiler().compile(content, options)
}

/**
 * Compile MDX to JavaScript synchronously using the default compiler
 * @param content - MDX content to compile
 * @param options - Compile options
 * @returns Compilation result
 * @throws CompilerError if compilation fails
 */
export function compileSync(content: string, options?: CompileOptions): CompileResult {
  return getDefaultCompiler().compileSync(content, options)
}