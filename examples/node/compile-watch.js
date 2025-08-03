import { readFile, watch, writeFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { compile } from '@jp-knj/mdx-hybrid-core'

const __dirname = dirname(fileURLToPath(import.meta.url))

async function compileMDXFile(filePath) {
  try {
    const content = await readFile(filePath, 'utf-8')
    const start = performance.now()

    const result = await compile(content, {
      development: true,
      jsx: true,
      jsxRuntime: 'automatic',
    })

    const outputPath = filePath.replace('.mdx', '.compiled.js')
    await writeFile(outputPath, result.code)

    const duration = performance.now() - start
    console.log(`✅ Compiled ${filePath} in ${duration.toFixed(2)}ms`)
  } catch (error) {
    console.error(`❌ Failed to compile ${filePath}:`, error.message)
  }
}

async function watchMDXFiles() {
  console.log('👀 Watching for MDX file changes...\n')

  const mdxFiles = ['example.mdx']

  // Compile all files initially
  for (const file of mdxFiles) {
    const filePath = join(__dirname, file)
    await compileMDXFile(filePath)
  }

  // Watch for changes
  const watcher = watch(__dirname, { recursive: false })

  console.log('\n🔄 Watching for changes (Ctrl+C to exit)...')

  for await (const event of watcher) {
    if (event.filename?.endsWith('.mdx')) {
      console.log(`\n📝 Change detected in ${event.filename}`)
      await compileMDXFile(join(__dirname, event.filename))
    }
  }
}

watchMDXFiles().catch(console.error)
