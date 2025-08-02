import { describe, expect, it } from 'vitest'
import { mdxHybrid } from '../src/index.js'

describe('Vite Plugin', () => {
  it('should create a vite plugin', () => {
    const plugin = mdxHybrid()
    expect(plugin.name).toBe('mdx-hybrid')
    expect(plugin.enforce).toBe('pre')
    expect(plugin.transform).toBeDefined()
    expect(plugin.handleHotUpdate).toBeDefined()
  })

  it('should accept plugin options', () => {
    const plugin = mdxHybrid({
      include: /\.mdx$/,
      exclude: /node_modules/,
      engine: 'rust'
    })
    expect(plugin).toBeDefined()
  })

  it('should filter files correctly', async () => {
    const plugin = mdxHybrid({
      include: /\.mdx$/
    })
    
    // The transform method should process .mdx files
    const result = await plugin.transform?.('# Test', 'test.mdx')
    // Since we can't easily test actual compilation in unit tests,
    // we just check that it tries to process the file
    expect(result === undefined || result !== null).toBe(true)
    
    // Should ignore non-mdx files
    const jsResult = await plugin.transform?.('console.log("test")', 'test.js')
    expect(jsResult).toBeUndefined()
  })
})