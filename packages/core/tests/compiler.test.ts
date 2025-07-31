import { describe, expect, it } from 'vitest'
import { Compiler, compile, compileSync } from '../src/compiler.js'

describe('Compiler', () => {
  const compiler = new Compiler()
  const simpleMDX = '# Hello\n\nThis is MDX'
  const jsxMDX = '# Title\n\n<Button>Click me</Button>'

  describe('compile', () => {
    it('should compile simple MDX', async () => {
      const result = await compiler.compile(simpleMDX)
      expect(result.code).toContain('function _createMdxContent')
      expect(result.timing).toBeGreaterThan(0)
    })

    it('should compile MDX with JSX', async () => {
      const result = await compiler.compile(jsxMDX)
      expect(result.code).toContain('Button')
      expect(result.timing).toBeGreaterThan(0)
    })

    it('should respect engine option', async () => {
      const result = await compiler.compile(simpleMDX, { engine: 'js' })
      expect(result.code).toBeTruthy()
      expect(result.timing).toBeGreaterThan(0)
    })

    it('should throw on invalid MDX', async () => {
      await expect(compiler.compile('# Title\n\n<Button>')).rejects.toThrow(
        'MDX compilation failed'
      )
    })
  })

  describe('compileSync', () => {
    it('should compile simple MDX synchronously', () => {
      const result = compiler.compileSync(simpleMDX)
      expect(result.code).toContain('function _createMdxContent')
      expect(result.timing).toBeGreaterThan(0)
    })

    it('should compile MDX with JSX synchronously', () => {
      const result = compiler.compileSync(jsxMDX)
      expect(result.code).toContain('Button')
      expect(result.timing).toBeGreaterThan(0)
    })
  })

  describe('convenience functions', () => {
    it('should export compile function', async () => {
      const result = await compile(simpleMDX)
      expect(result.code).toBeTruthy()
    })

    it('should export compileSync function', () => {
      const result = compileSync(simpleMDX)
      expect(result.code).toBeTruthy()
    })
  })

  describe('getEngineInfo', () => {
    it('should return engine availability info', () => {
      const info = compiler.getEngineInfo()
      expect(info.js).toBe(true)
      expect(typeof info.rust).toBe('boolean')
    })
  })
})
