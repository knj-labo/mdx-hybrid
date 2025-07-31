/**
 * Error thrown when Rust engine is explicitly requested but not available
 */
export class RustUnavailableError extends Error {
  constructor(
    message = 'Rust engine is not available. Please ensure @mdx-hybrid/engine-rust is properly installed.'
  ) {
    super(message)
    this.name = 'RustUnavailableError'
  }
}

/**
 * Warning when Rust engine panics and falls back to JS
 */
export class RustPanicWarning extends Error {
  constructor(public originalError: Error) {
    super(`Rust engine encountered an error, falling back to JS engine: ${originalError.message}`)
    this.name = 'RustPanicWarning'
  }
}

/**
 * General compiler error from either engine
 */
export class CompilerError extends Error {
  constructor(
    message: string,
    public engine: 'js' | 'rust',
    public originalError?: Error
  ) {
    super(message)
    this.name = 'CompilerError'
  }
}
