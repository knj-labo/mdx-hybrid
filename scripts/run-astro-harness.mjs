#!/usr/bin/env node
import { spawnSync } from 'node:child_process'
import { performance } from 'node:perf_hooks'
import { resolve } from 'node:path'
import { rmSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const harnessDir = resolve(process.cwd(), 'fixtures/integration/astro-harness')
const distDir = resolve(harnessDir, 'dist')
const thisFile = fileURLToPath(import.meta.url)

const validModes = ['markflow', 'baseline']

export function runHarnessBuild(mode, { inheritLogs = true, cleanDist = true } = {}) {
  if (!validModes.includes(mode)) {
    throw new Error(`Unknown mode "${mode}" (expected ${validModes.join(', ')})`)
  }

  if (cleanDist) {
    rmSync(distDir, { recursive: true, force: true })
  }

  const env = { ...process.env }
  if (mode === 'baseline') {
    env.MARKFLOW_HARNESS_BASELINE = '1'
  } else {
    delete env.MARKFLOW_HARNESS_BASELINE
  }

  if (inheritLogs) {
    console.log(`🏗️  Running Astro harness build (${mode})`)
  }

  const start = performance.now()
  const result = spawnSync('pnpm', ['astro', 'build'], {
    cwd: harnessDir,
    stdio: inheritLogs ? 'inherit' : 'ignore',
    env,
  })
  const duration = performance.now() - start

  if (result.status !== 0) {
    const message = `Harness build failed (mode=${mode})`
    if (inheritLogs) {
      console.error(message)
    }
    throw new Error(message)
  }

  if (inheritLogs) {
    console.log(`✅ Harness build (${mode}) finished in ${duration.toFixed(2)}ms`)
  }

  return duration
}

async function main() {
  const mode = process.argv[2] ?? 'markflow'
  if (!validModes.includes(mode)) {
    console.error('Usage: node scripts/run-astro-harness.mjs [markflow|baseline]')
    process.exit(1)
  }

  try {
    runHarnessBuild(mode)
  } catch (err) {
    console.error(err.message)
    process.exit(1)
  }
}

const invokedPath = process.argv[1] ? resolve(process.cwd(), process.argv[1]) : ''
const isDirectRun = invokedPath === thisFile
if (isDirectRun) {
  await main()
}
