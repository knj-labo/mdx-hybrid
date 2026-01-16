#!/usr/bin/env node
import { readFile } from 'node:fs/promises'
import { basename, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { performance } from 'node:perf_hooks'

import { parseBlocks, parseFrontmatter } from '../crates/napi/index.js'

const __dirname = fileURLToPath(new URL('.', import.meta.url))
const repoRoot = resolve(__dirname, '..')
const userArgs = process.argv.slice(2).filter((arg) => arg !== '--')
const [inputArg = 'fixtures/core/markdown/hello.md'] = userArgs
const inputPath = resolve(repoRoot, inputArg)

const markdown = await readFile(inputPath, 'utf8')
console.log(`Running markflow-napi smoke test for ${basename(inputPath)}`)

// Test parseBlocks
const start = performance.now()
const result = parseBlocks(markdown, { enableDirectives: true })
const elapsed = performance.now() - start
console.log(`parseBlocks() returned ${result.blocks.length} blocks in ${elapsed.toFixed(3)}ms`)

// Test parseFrontmatter
const fmResult = parseFrontmatter(markdown)
console.log(`parseFrontmatter() returned ${Object.keys(fmResult.frontmatter).length} keys`)

// Show preview
const htmlContent = result.blocks
  .filter((b) => b.type === 'html')
  .map((b) => b.content)
  .join('')
console.log(`\nHTML preview:\n`, htmlContent.slice(0, 160), htmlContent.length > 160 ? '...' : '')
