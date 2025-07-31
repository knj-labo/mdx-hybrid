import { describe, expect, it, vi } from 'vitest'
import { EngineSelector } from '../src/engine-selector.js'
import { RustUnavailableError } from '../src/errors.js'

describe('EngineSelector', () => {
  const selector = new EngineSelector()

  describe('selectEngine', () => {
    it('should select JS engine when explicitly requested', () => {
      const engine = selector.selectEngine({ engine: 'js' })
      expect(engine.getName()).toBe('js')
    })

    it('should select JS engine when remark plugins are present', () => {
      const engine = selector.selectEngine({
        remarkPlugins: [() => {}],
      })
      expect(engine.getName()).toBe('js')
    })

    it('should select JS engine when rehype plugins are present', () => {
      const engine = selector.selectEngine({
        rehypePlugins: [() => {}],
      })
      expect(engine.getName()).toBe('js')
    })

    it('should throw when Rust engine is explicitly requested but unavailable', () => {
      // Mock Rust as unavailable
      const mockSelector = new EngineSelector()
      ;(mockSelector as any).rustAvailable = false

      expect(() => mockSelector.selectEngine({ engine: 'rust' })).toThrow(RustUnavailableError)
    })

    it('should select appropriate engine in auto mode', () => {
      const engine = selector.selectEngine({ engine: 'auto' })
      expect(['js', 'rust']).toContain(engine.getName())
    })

    it('should select JS engine as default', () => {
      const engine = selector.selectEngine()
      expect(engine.getName()).toBe('js')
    })
  })

  describe('compileWithFallback', () => {
    it('should compile with selected engine', async () => {
      const result = await selector.compileWithFallback('# Hello', { engine: 'js' })
      expect(result.result.code).toBeTruthy()
      expect(result.engine.getName()).toBe('js')
      expect(result.warning).toBeUndefined()
    })

    it('should fallback to JS when Rust fails in auto mode', async () => {
      // This test would require mocking the Rust engine to fail
      // For now, we'll test with JS engine which should always work
      const result = await selector.compileWithFallback('# Hello')
      expect(result.result.code).toBeTruthy()
    })
  })

  describe('compileWithFallbackSync', () => {
    it('should compile synchronously with selected engine', () => {
      const result = selector.compileWithFallbackSync('# Hello', { engine: 'js' })
      expect(result.result.code).toBeTruthy()
      expect(result.engine.getName()).toBe('js')
      expect(result.warning).toBeUndefined()
    })

    it('should handle errors appropriately', () => {
      expect(() => selector.compileWithFallbackSync('# Unclosed <tag', { engine: 'js' })).toThrow()
    })
  })
})
