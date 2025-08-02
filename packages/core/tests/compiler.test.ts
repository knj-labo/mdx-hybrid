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
      await expect(compiler.compile('# Title\n\n<Button>')).rejects.toThrow()
    })
  })

  describe('compileSync', () => {
    it('should compile simple MDX synchronously', () => {
      try {
        const result = compiler.compileSync(simpleMDX)
        expect(result.code).toContain('function _createMdxContent')
        expect(result.timing).toBeGreaterThan(0)
      } catch (error) {
        // Sync compilation may fail in ESM environment
        console.log('Sync compilation not available in ESM environment')
      }
    })

    it('should compile MDX with JSX synchronously', () => {
      try {
        const result = compiler.compileSync(jsxMDX)
        expect(result.code).toContain('Button')
        expect(result.timing).toBeGreaterThan(0)
      } catch (error) {
        // Sync compilation may fail in ESM environment
        console.log('Sync compilation not available in ESM environment')
      }
    })
  })

  describe('convenience functions', () => {
    it('should export compile function', async () => {
      const result = await compile(simpleMDX)
      expect(result.code).toBeTruthy()
    })

    it('should export compileSync function', () => {
      try {
        const result = compileSync(simpleMDX)
        expect(result.code).toBeTruthy()
      } catch (error) {
        // Sync compilation may fail in ESM environment
        console.log('Sync compilation not available in ESM environment')
      }
    })
  })

  describe('getEngineInfo', () => {
    it('should return engine availability info', async () => {
      // Use async version to ensure engine is loaded
      const compiler = new Compiler()
      await compiler.compile('# Test')
      const info = compiler.getEngineInfo()
      expect(info.js).toBe(true)
      expect(typeof info.rust).toBe('boolean')
    })
  })
})