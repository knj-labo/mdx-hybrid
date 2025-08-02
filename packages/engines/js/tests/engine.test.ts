import { describe, expect, it } from 'vitest'
import { createJSEngine } from '../src/engine.js'

describe('JSEngine', () => {
  const engine = createJSEngine()

  describe('getName', () => {
    it('should return "js"', () => {
      expect(engine.getName()).toBe('js')
    })
  })

  describe('isAvailable', () => {
    it('should always return true', () => {
      expect(engine.isAvailable()).toBe(true)
    })
  })

  describe('compile', () => {
    it('should compile simple MDX', async () => {
      const result = await engine.compile('# Hello World')
      expect(result.code).toContain('function _createMdxContent')
      expect(result.timing).toBeGreaterThan(0)
    })

    it('should compile MDX with JSX', async () => {
      const result = await engine.compile('# Title\n\n<Button>Click</Button>')
      expect(result.code).toContain('Button')
      expect(result.timing).toBeGreaterThan(0)
    })

    it('should handle options correctly', async () => {
      const result = await engine.compile('# Test', {
        development: true,
        jsx: true,
      })
      expect(result.code).toBeTruthy()
    })

    it('should throw on invalid MDX', async () => {
      await expect(engine.compile('<Button>Unclosed tag')).rejects.toThrow('MDX compilation failed')
    })
  })

  describe('compileSync', () => {
    it('should compile simple MDX synchronously', () => {
      const result = engine.compileSync('# Hello World')
      expect(result.code).toContain('function _createMdxContent')
      expect(result.timing).toBeGreaterThan(0)
    })

    it('should compile MDX with JSX synchronously', () => {
      const result = engine.compileSync('# Title\n\n<Button>Click</Button>')
      expect(result.code).toContain('Button')
      expect(result.timing).toBeGreaterThan(0)
    })

    it('should handle options correctly', () => {
      const result = engine.compileSync('# Test', {
        development: true,
        jsx: true,
      })
      expect(result.code).toBeTruthy()
    })

    it('should throw on invalid MDX', () => {
      expect(() => engine.compileSync('<Button>Unclosed tag')).toThrow('MDX compilation failed')
    })
  })

  describe('option transformation', () => {
    it('should transform outputFormat correctly', async () => {
      const result = await engine.compile('# Test', {
        outputFormat: 'esm',
      })
      expect(result.code).toBeTruthy()
    })

    it('should handle sourcemap option', async () => {
      const result = await engine.compile('# Test', {
        sourcemap: true,
      })
      expect(result.code).toBeTruthy()
      // Note: actual sourcemap testing would require checking result.map
    })
  })
})
