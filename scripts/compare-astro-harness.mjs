#!/usr/bin/env node
import { runHarnessBuild } from './run-astro-harness.mjs'
import { dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { mkdirSync, writeFileSync } from 'node:fs'

const args = process.argv.slice(2)
const runsArg = args.find((arg) => arg.startsWith('--runs='))
const summaryArg = args.find((arg) => arg.startsWith('--summary='))
const targetArg = args.find((arg) => arg.startsWith('--target='))
const runs = runsArg ? Number(runsArg.split('=')[1]) : 3
const summaryPath = summaryArg ? summaryArg.split('=')[1] : null
const target = targetArg ? targetArg.split('=')[1] : 'astro'

if (!Number.isInteger(runs) || runs < 1) {
  console.error(
    'Usage: node scripts/compare-astro-harness.mjs [--runs=N] [--summary=path] [--target=name]',
  )
  process.exit(1)
}

const modes = ['markflow', 'baseline']
const summary = {}

for (const mode of modes) {
  summary[mode] = []
  for (let i = 0; i < runs; i++) {
    const duration = runHarnessBuild(target, mode, { inheritLogs: false, cleanDist: true })
    summary[mode].push(duration)
    console.log(`✅ ${mode} run ${i + 1}/${runs}: ${duration.toFixed(2)}ms`)
  }
}

function stats(values) {
  const total = values.reduce((sum, value) => sum + value, 0)
  const avg = total / values.length
  const min = Math.min(...values)
  const max = Math.max(...values)
  return { avg, min, max }
}

const markflowStats = stats(summary.markflow)
const baselineStats = stats(summary.baseline)
const speedup = baselineStats.avg / markflowStats.avg

console.log(`\n📊 ${target} Harness Comparison`)
console.log(`Runs: ${runs}`)
console.log(
  `Markflow  -> avg: ${markflowStats.avg.toFixed(2)}ms (min ${markflowStats.min.toFixed(2)} / max ${markflowStats.max.toFixed(2)})`,
)
console.log(
  `Baseline  -> avg: ${baselineStats.avg.toFixed(2)}ms (min ${baselineStats.min.toFixed(2)} / max ${baselineStats.max.toFixed(2)})`,
)
console.log(`Speedup   -> baseline / markflow = ${speedup.toFixed(2)}x`)

if (summaryPath) {
  const summary = {
    runs,
    target,
    markflow: markflowStats,
    baseline: baselineStats,
    speedup,
  }
  mkdirSync(dirname(summaryPath), { recursive: true })
  writeFileSync(summaryPath, JSON.stringify(summary, null, 2))
}
