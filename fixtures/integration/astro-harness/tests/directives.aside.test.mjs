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

const { runHarnessBuild } = await import(resolve(repoRoot, 'scripts/run-astro-harness.mjs'))
const { createCompiler } = await import(resolve(repoRoot, 'crates/napi/index.js'))

const directivesPath = resolve(__dirname, '../content/docs/directives.mdx')

// 1) Build the Astro harness (markflow mode) and check rendered HTML contains Aside output.
await runHarnessBuild('astro', 'markflow', { inheritLogs: false })

const distHtml = resolve(__dirname, '../dist/index.html')
const html = readFileSync(distHtml, 'utf8')

assert(html.includes('<Aside type="note">'), 'dist output should contain Aside from directives page')

// 2) Compile the directives page directly and ensure Aside import is injected exactly once.
const compiler = createCompiler()
const source = readFileSync(directivesPath, 'utf8')
const result = await compiler.compile(source, directivesPath)

const importLine = "import { Aside } from '@astrojs/starlight/components';"
const occurrences = result.code.split(importLine).length - 1

assert(occurrences === 1, 'Aside import should be injected exactly once')
assert(result.code.includes('<Aside type="note">'), 'Compiled code should contain Aside markup')

console.log('✅ directives.aside.test.mjs passed')
