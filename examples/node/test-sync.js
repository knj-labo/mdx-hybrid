import { preloadEngines, compileSync } from '@mdx-hybrid/core'
import { readFileSync } from 'fs'
import { resolve } from 'path'
import { fileURLToPath } from 'url'

const __dirname = fileURLToPath(new URL('.', import.meta.url))

console.log('Testing sync compilation with preloading in ESM\n')

// Test 1: Try sync without preloading (should fail)
console.log('1. Testing sync without preloading...')
try {
  compileSync('# Hello')
  console.log('ERROR: Should have thrown an error!')
} catch (error) {
  console.log('Expected error:', error.message)
}

// Test 2: Preload engines
console.log('\n2. Preloading engines...')
await preloadEngines()
console.log('Engines preloaded')

// Test 3: Try sync after preloading (should work)
console.log('\n3. Testing sync after preloading...')
try {
  const mdxContent = readFileSync(resolve(__dirname, 'example.mdx'), 'utf-8')
  
  // Test with auto engine selection
  console.log('\nAuto engine:')
  const result1 = compileSync(mdxContent)
  console.log('Compiled successfully, code length:', result1.code.length)
  
  // Test with explicit JS engine
  console.log('\nJS engine:')
  const result2 = compileSync(mdxContent, { engine: 'js' })
  console.log('Compiled successfully, code length:', result2.code.length)
  
  // Test with explicit Rust engine
  console.log('\nRust engine:')
  const result3 = compileSync(mdxContent, { engine: 'rust' })
  console.log('Compiled successfully, code length:', result3.code.length)
  console.log('Speedup:', (result2.timing / result3.timing).toFixed(2) + 'x')
  
} catch (error) {
  console.log('Error:', error.message)
}

console.log('\nAll tests completed!')