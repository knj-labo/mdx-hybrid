#!/usr/bin/env node
import { runHarnessBuild } from './run-astro-harness.mjs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { mkdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs'
import { parse } from 'parse5'

const args = process.argv.slice(2)
const runsArg = args.find((arg) => arg.startsWith('--runs='))
const summaryArg = args.find((arg) => arg.startsWith('--summary='))
const targetArg = args.find((arg) => arg.startsWith('--target='))
const modeArg = args.find((arg) => arg.startsWith('--mode='))
const outputArg = args.find((arg) => arg.startsWith('--output='))
const runs = runsArg ? Number(runsArg.split('=')[1]) : 3
const summaryPath = summaryArg ? summaryArg.split('=')[1] : null
const target = targetArg ? targetArg.split('=')[1] : 'astro'
const mode = modeArg ? modeArg.split('=')[1] : 'string' // 'string' | 'semantic'
const outputPath = outputArg ? outputArg.split('=')[1] : null

if (!Number.isInteger(runs) || runs < 1) {
  console.error(
    'Usage: node scripts/compare-astro-harness.mjs [--runs=N] [--summary=path] [--target=name] [--mode=string|semantic] [--output=path]',
  )
  process.exit(1)
}

// Ensure NAPI binary exists before running harness to avoid cryptic failures.
const napiBinGlob = [
  'crates/napi/markflow.darwin-arm64.node',
  'crates/napi/markflow.darwin-x64.node',
  'crates/napi/markflow.darwin-universal.node',
  'crates/napi/markflow.linux-x64-gnu.node',
  'crates/napi/markflow.linux-x64-musl.node',
  'crates/napi/markflow.win32-x64-msvc.node',
  'crates/napi/markflow.win32-ia32-msvc.node',
]
const hasNapiBinary = napiBinGlob.some((p) => existsSync(resolve(p)))
if (!hasNapiBinary) {
  console.error(
    '❌ NAPI native binary not found. Build it first: `cd crates/napi && pnpm install && pnpm run build:napi`',
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

// Semantic diff (optional)
if (mode === 'semantic') {
  const baselineHtml = readFileSync(resolve('dist-baseline/index.html'), 'utf8')
  const markflowHtml = readFileSync(resolve('dist-markflow/index.html'), 'utf8')
  const diff = semanticDiff(baselineHtml, markflowHtml)
  const summaryDiff = diff.equal ? 'semantic: equal' : `semantic: ${diff.total} differences`
  console.log(`Diff result -> ${summaryDiff}`)
  if (outputPath) {
    mkdirSync(dirname(outputPath), { recursive: true })
    writeFileSync(outputPath, JSON.stringify(diff, null, 2))
  }
  if (!diff.equal) process.exit(1)
}

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

// ---------- helpers ----------

function diffHtml(a, b) {
  if (a === b) return { equal: true }
  return { equal: false, message: 'HTML differs (string compare)' }
}

function normalizeNode(node) {
  if (node.childNodes) {
    node.childNodes = node.childNodes.map(normalizeNode).filter(Boolean)
  }
  if (node.nodeName === '#text') {
    const text = (node.value || '').replace(/\\s+/g, ' ').trim()
    return text ? { ...node, value: text } : null
  }
  if (node.attrs) {
    node.attrs = [...node.attrs].sort((a, b) => a.name.localeCompare(b.name))
  }
  return node
}

function semanticDiffHtml(aHtml, bHtml) {
  const aDoc = parse(aHtml)
  const bDoc = parse(bHtml)
  const a = normalizeNode(aDoc)
  const b = normalizeNode(bDoc)
  const diffs = []
  walkDiff(a, b, [], diffs)
  return diffs
}

function walkDiff(a, b, path, diffs) {
  if (!a || !b) {
    diffs.push({ path, message: 'Node presence differs', a: !!a, b: !!b })
    return
  }
  if (a.nodeName !== b.nodeName) {
    diffs.push({ path, message: 'Tag differs', a: a.nodeName, b: b.nodeName })
  }
  if (a.attrs || b.attrs) {
    const attrsA = Object.fromEntries((a.attrs || []).map(({ name, value }) => [name, value]))
    const attrsB = Object.fromEntries((b.attrs || []).map(({ name, value }) => [name, value]))
    if (JSON.stringify(attrsA) !== JSON.stringify(attrsB)) {
      diffs.push({ path, message: 'Attrs differ', a: attrsA, b: attrsB })
    }
  }
  if (a.nodeName === '#text' && b.nodeName === '#text' && a.value !== b.value) {
    diffs.push({ path, message: 'Text differs', a: a.value, b: b.value })
  }
  const childrenA = a.childNodes || []
  const childrenB = b.childNodes || []
  const max = Math.max(childrenA.length, childrenB.length)
  for (let i = 0; i < max; i++) {
    walkDiff(childrenA[i], childrenB[i], path.concat(i), diffs)
  }
}

function semanticDiff(aHtml, bHtml) {
  const diffs = semanticDiffHtml(aHtml, bHtml)
  if (diffs.length === 0) return { equal: true }
  return { equal: false, semanticDiff: diffs.slice(0, 50), total: diffs.length }
}
