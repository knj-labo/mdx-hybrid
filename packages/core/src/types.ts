export interface CompileOptions {
  /**
   * Which engine to use for compilation
   * - 'js': Force JavaScript engine (@mdx-js/mdx)
   * - 'rust': Force Rust engine (mdxjs-rs)
   * - 'auto': Automatically select based on features (default)
   */
  engine?: 'js' | 'rust' | 'auto'

  /**
   * List of remark plugins to use
   * Note: Forces JavaScript engine when provided
   */
  remarkPlugins?: any[]

  /**
   * List of rehype plugins to use
   * Note: Forces JavaScript engine when provided
   */
  rehypePlugins?: any[]

  /**
   * Generate code for development
   * @default false
   */
  development?: boolean

  /**
   * Output JSX
   * @default false
   */
  jsx?: boolean

  /**
   * JSX runtime to use
   * @default 'automatic'
   */
  jsxRuntime?: 'automatic' | 'classic'

  /**
   * Place to import automatic JSX runtimes from
   * @default 'react'
   */
  jsxImportSource?: string

  /**
   * Whether to keep JSX
   * @default false
   */
  jsxKeep?: boolean

  /**
   * Pragma for JSX (used in classic runtime)
   * @default 'React.createElement'
   */
  pragma?: string

  /**
   * Pragma for JSX fragments (used in classic runtime)
   * @default 'React.Fragment'
   */
  pragmaFrag?: string

  /**
   * Import pragma from (used in classic runtime)
   * @default 'react'
   */
  pragmaImportSource?: string

  /**
   * Generate source maps
   * @default false
   */
  sourcemap?: boolean

  /**
   * Format of the output
   * @default 'program'
   */
  format?: 'program' | 'function-body'

  /**
   * Module format of the output
   * @default 'esm'
   */
  outputFormat?: 'esm' | 'cjs' | 'function-body'
}

export interface CompileResult {
  /**
   * Compiled JavaScript code
   */
  code: string

  /**
   * Source map (v3 specification)
   */
  map?: any

  /**
   * Compilation time in milliseconds
   */
  timing: number
}

export interface Engine {
  /**
   * Compile MDX to JavaScript asynchronously
   */
  compile(content: string, options?: CompileOptions): Promise<CompileResult>

  /**
   * Compile MDX to JavaScript synchronously
   */
  compileSync(content: string, options?: CompileOptions): CompileResult

  /**
   * Check if this engine is available
   */
  isAvailable(): boolean

  /**
   * Get engine name for debugging
   */
  getName(): string
}

export interface VitePluginOptions extends CompileOptions {
  /**
   * Include patterns for MDX files
   * @default /\.mdx?$/
   */
  include?: RegExp | RegExp[]

  /**
   * Exclude patterns for MDX files
   */
  exclude?: RegExp | RegExp[]
}