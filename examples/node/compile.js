import { readFile, writeFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { compile } from '@mdx-hybrid/core'

const __dirname = dirname(fileURLToPath(import.meta.url))

async function compileMDX() {
  console.log('🚀 MDX Hybrid - Node.js Example\n')

  try {
    // Read the MDX file
    const mdxPath = join(__dirname, 'example.mdx')
    const mdxContent = await readFile(mdxPath, 'utf-8')
    console.log('📄 Reading:', mdxPath)

    // Compile with auto engine selection
    console.log('\n⚡ Compiling with auto engine selection...')
    const autoResult = await compile(mdxContent)
    console.log(`✅ Compiled successfully in ${autoResult.timing}ms`)

    // Try compiling with JS engine explicitly
    console.log('\n🟨 Compiling with JS engine...')
    const jsResult = await compile(mdxContent, { engine: 'js' })
    console.log(`✅ JS engine completed in ${jsResult.timing}ms`)

    // Try compiling with Rust engine explicitly
    console.log('\n🦀 Compiling with Rust engine...')
    try {
      const rustResult = await compile(mdxContent, { engine: 'rust' })
      console.log(`✅ Rust engine completed in ${rustResult.timing}ms`)
      console.log(`⚡ Speedup: ${(jsResult.timing / rustResult.timing).toFixed(2)}x faster`)
    } catch (error) {
      console.log('❌ Rust engine not available:', error.message)
    }

    // Save the compiled output
    const outputPath = join(__dirname, 'output.js')
    await writeFile(outputPath, autoResult.code)
    console.log('\n💾 Saved compiled output to:', outputPath)

    // Show a snippet of the compiled code
    console.log('\n📝 Compiled code preview:')
    console.log(autoResult.code.substring(0, 200) + '...')
  } catch (error) {
    console.error('❌ Compilation failed:', error.message)
    process.exit(1)
  }
}

compileMDX()
