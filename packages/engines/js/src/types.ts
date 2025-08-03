export interface CompileOptions {
  engine?: 'js' | 'rust' | 'auto'
  remarkPlugins?: any[]
  rehypePlugins?: any[]
  development?: boolean
  jsx?: boolean
  jsxRuntime?: 'automatic' | 'classic'
  jsxImportSource?: string
  jsxKeep?: boolean
  pragma?: string
  pragmaFrag?: string
  pragmaImportSource?: string
  sourcemap?: boolean
  format?: 'program' | 'function-body'
  outputFormat?: 'esm' | 'cjs' | 'function-body'
}

export interface CompileResult {
  code: string
  map?: any
  timing: number
  data?: Record<string, any>
}

export interface Engine {
  compile(content: string, options?: CompileOptions): Promise<CompileResult>
  compileSync(content: string, options?: CompileOptions): CompileResult
  isAvailable(): boolean
  getName(): string
}
