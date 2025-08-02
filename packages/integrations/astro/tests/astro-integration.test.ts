import { describe, expect, it } from 'vitest'
import { mdxHybrid } from '../src/index.js'

describe('Astro Integration', () => {
  it('should create an astro integration', () => {
    const integration = mdxHybrid()
    expect(integration.name).toBe('@mdx-hybrid/astro')
    expect(integration.hooks).toBeDefined()
    expect(integration.hooks['astro:config:setup']).toBeDefined()
  })

  it('should accept options', () => {
    const integration = mdxHybrid({
      engine: 'rust',
      include: /\.mdx$/,
      exclude: /node_modules/,
    })
    expect(integration).toBeDefined()
  })

  it('should have proper vite plugin structure', () => {
    const integration = mdxHybrid()
    let vitePlugin: any

    // Mock the astro config setup
    const mockUpdateConfig = (config: any) => {
      vitePlugin = config.vite.plugins[0]
    }

    const mockAddPageExtension = (ext: string) => {
      expect(ext).toBe('.mdx')
    }

    // Call the setup hook
    integration.hooks['astro:config:setup']!({
      config: {} as any,
      command: 'dev',
      addPageExtension: mockAddPageExtension,
      updateConfig: mockUpdateConfig,
    } as any)

    // Check vite plugin structure
    expect(vitePlugin).toBeDefined()
    expect(vitePlugin.name).toBe('mdx-hybrid:astro')
    expect(vitePlugin.enforce).toBe('pre')
    expect(vitePlugin.transform).toBeDefined()
    expect(vitePlugin.configureServer).toBeDefined()
  })
})
