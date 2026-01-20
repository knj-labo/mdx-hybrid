#!/usr/bin/env node
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

if (!process.env.MARKFLOW_HARNESS_E2E) {
  console.log('SKIP directives.aside.test.mjs (set MARKFLOW_HARNESS_E2E=1 to run)')
  process.exit(0)
}

const repoRoot = resolve(__dirname, '../../../..')

const { runHarness } = await import(resolve(repoRoot, 'scripts/run-astro-harness.mjs'))
const { createCompiler } = await import(resolve(repoRoot, 'crates/napi/index.js'))

const directivesPath = resolve(__dirname, '../content/docs/directives.mdx')

// 1) Build the Astro harness (markflow mode) and check rendered HTML contains Aside output.
await runHarness('markflow', { skipInstall: true })

const distHtml = resolve(__dirname, '../dist/index.html')
const html = readFileSync(distHtml, 'utf8')

// The Aside component renders to <aside class="aside aside--note">
assert(html.includes('aside--note'), 'dist output should contain rendered Aside from directives page')

// 2) Compile the directives page directly and check the JSX output contains Aside component.
const compiler = createCompiler()
const source = readFileSync(directivesPath, 'utf8')
const result = compiler.compile(source, directivesPath)

// The compiler generates JSX with spread syntax: <Aside {...{"type": "note"}}>
assert(result.code.includes('<Aside'), 'Compiled code should contain Aside component')
assert(result.code.includes('"type": "note"'), 'Compiled code should contain type="note" prop')

console.log('✅ directives.aside.test.mjs passed')
