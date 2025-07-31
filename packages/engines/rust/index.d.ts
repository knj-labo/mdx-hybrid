export interface CompileOptions {
  development?: boolean
  jsx?: boolean
  jsxRuntime?: string
  jsxImportSource?: string
  pragma?: string
  pragmaFrag?: string
  pragmaImportSource?: string
}

export interface CompileResult {
  code: string
  map?: string
  timing: number
}

export function compileSync(content: string, options?: CompileOptions): CompileResult
export function isAvailable(): boolean
