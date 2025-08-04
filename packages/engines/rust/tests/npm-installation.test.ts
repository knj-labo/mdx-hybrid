import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { execSync } from 'child_process'
import { mkdtempSync, rmSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'

describe('NPM Installation Test', () => {
  let tempDir: string

  beforeAll(() => {
    // Create a temporary directory for testing
    tempDir = mkdtempSync(join(tmpdir(), 'mdx-hybrid-test-'))
  })

  afterAll(() => {
    // Clean up temporary directory
    if (tempDir) {
      rmSync(tempDir, { recursive: true, force: true })
    }
  })

  it('should install rust engine with platform-specific binary', async () => {
    // Skip in CI if npm token not available
    if (process.env.CI && !process.env.NPM_TOKEN) {
      console.log('Skipping npm installation test - NPM_TOKEN not configured')
      return
    }

    // Create a test package.json
    const packageJson = {
      name: 'test-mdx-hybrid-installation',
      version: '1.0.0',
      dependencies: {
        '@jp-knj/mdx-hybrid-engine-rust': 'latest',
      },
    }

    const packageJsonPath = join(tempDir, 'package.json')
    execSync(
      `echo '${JSON.stringify(packageJson, null, 2)}' > "${packageJsonPath}"`,
      { stdio: 'inherit' }
    )

    try {
      // Install the package
      console.log('Installing @jp-knj/mdx-hybrid-engine-rust...')
      execSync('npm install --no-save', {
        cwd: tempDir,
        stdio: 'inherit',
        timeout: 60000, // 60 second timeout
      })

      // Test that the package can be required
      const testScript = `
        const rustEngine = require('@jp-knj/mdx-hybrid-engine-rust');
        console.log('isAvailable:', typeof rustEngine.isAvailable === 'function');
        console.log('compileSync:', typeof rustEngine.compileSync === 'function');
        
        if (rustEngine.isAvailable && rustEngine.isAvailable()) {
          const result = rustEngine.compileSync('# Hello World');
          console.log('Compilation successful:', result.code.length > 0);
        } else {
          console.log('Rust engine not available on this platform');
        }
      `

      const output = execSync(`node -e "${testScript}"`, {
        cwd: tempDir,
        encoding: 'utf8',
      })

      expect(output).toContain('isAvailable: true')
      expect(output).toContain('compileSync: true')
    } catch (error) {
      // If the package isn't published yet, skip the test
      if (error.message.includes('404')) {
        console.log('Package not yet published to npm - skipping test')
        return
      }
      throw error
    }
  }, 120000) // 2 minute timeout for the entire test
})