#!/usr/bin/env node
import { promises as fs } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { runHarness } from './run-astro-harness.mjs'

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const defaultSummary = resolve(
  repoRoot,
  'fixtures/integration/astro-harness/harness-summary.json',
)

function parseArgs(argv) {
  const args = { summary: defaultSummary, skipInstall: false }
  for (let i = 2; i < argv.length; i++) {
    const arg = argv[i]
    if (arg === '--summary' && argv[i + 1]) {
      args.summary = resolve(repoRoot, argv[i + 1])
      i++
    } else if (arg === '--skip-install') {
      args.skipInstall = true
    }
  }
  return args
}

async function main() {
  const { summary, skipInstall } = parseArgs(process.argv)

  const baseline = await runHarness('baseline', { skipInstall })
  const markflow = await runHarness('markflow', { skipInstall })

  const report = {
    timestamp: new Date().toISOString(),
    baselineMs: Math.round(baseline.durationMs),
    markflowMs: Math.round(markflow.durationMs),
    note: 'build-only; no HTML diff',
  }

  await fs.mkdir(dirname(summary), { recursive: true })
  await fs.writeFile(summary, JSON.stringify(report, null, 2))
  console.log(`Harness summary written to ${summary}`)
  console.log(report)
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((err) => {
    console.error(err.message || err)
    process.exit(1)
  })
}
