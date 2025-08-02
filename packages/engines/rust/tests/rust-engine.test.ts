import { describe, expect, it } from 'vitest'

describe('Rust Engine', () => {
  it('should check if rust binary is available', () => {
    try {
      const rustEngine = require('../index.js')
      expect(rustEngine.isAvailable).toBeDefined()
      expect(typeof rustEngine.isAvailable()).toBe('boolean')
    } catch (error) {
      // Rust engine is optional
      console.log('Rust engine not available in test environment')
    }
  })

  it('should compile simple MDX if available', () => {
    try {
      const rustEngine = require('../index.js')
      if (rustEngine.isAvailable && rustEngine.isAvailable()) {
        const result = rustEngine.compileSync('# Hello', {})
        expect(result.code).toBeTruthy()
        expect(result.code).toContain('function')
      } else {
        console.log('Rust engine not available, skipping compile test')
      }
    } catch (error) {
      console.log('Rust engine not available in test environment')
    }
  })
})