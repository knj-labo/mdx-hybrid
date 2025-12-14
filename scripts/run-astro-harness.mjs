#!/usr/bin/env node
import { spawnSync } from 'node:child_process'
import { performance } from 'node:perf_hooks'
import { resolve, dirname } from 'node:path'
import { rmSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)
const repoRoot = resolve(__dirname, '..')
const thisFile = __filename

const harnesses = {
  astro: {
    label: 'astro-harness',
    cwd: resolve(repoRoot, 'fixtures/integration/astro-harness'),
    distDirs: ['dist'],
    command: 'pnpm',
    args: ['astro', 'build'],
  },
  'withastro-docs': {
    label: 'withastro-docs',
    cwd: resolve(repoRoot, 'fixtures/integration/withastro-docs/repo'),
    distDirs: ['dist'],
    command: 'pnpm',
    args: ['astro', 'build'],
    env: {
      MARKFLOW_HARNESS_OFFLINE: '1',
      SKIP_OG: '1',
    },
  },
}

const validTargets = Object.keys(harnesses)
const defaultTarget = 'astro'
const validModes = ['markflow', 'baseline']

function cleanDistDirs(target, enabled) {
  if (!enabled) return
  for (const dist of target.distDirs ?? []) {
    const fullPath = resolve(target.cwd, dist)
    rmSync(fullPath, { recursive: true, force: true })
  }
}

function resolveArgs(targetOrMode, maybeMode, maybeOptions) {
  let targetName = defaultTarget
  let mode
  let options

  if (targetOrMode && validTargets.includes(targetOrMode)) {
    targetName = targetOrMode
    if (typeof maybeMode === 'string') {
      mode = maybeMode
      options = maybeOptions
    } else {
      mode = 'markflow'
      options = maybeMode
    }
  } else {
    mode = typeof targetOrMode === 'string' ? targetOrMode : undefined
    options = maybeMode
  }

  return {
    targetName,
    mode: mode ?? 'markflow',
    options: options ?? {},
  }
}

export function runHarnessBuild(targetOrMode, maybeMode, maybeOptions) {
  const { targetName, mode, options } = resolveArgs(targetOrMode, maybeMode, maybeOptions)
  if (!validTargets.includes(targetName)) {
    throw new Error(`Unknown harness target "${targetName}" (expected ${validTargets.join(', ')})`)
  }
  if (!validModes.includes(mode)) {
    throw new Error(`Unknown mode "${mode}" (expected ${validModes.join(', ')})`)
  }

  const { inheritLogs = true, cleanDist = true } = options
  const target = harnesses[targetName]

  cleanDistDirs(target, cleanDist)

  const env = { ...process.env, ...(target.env ?? {}) }
  if (mode === 'baseline') {
    env.MARKFLOW_HARNESS_BASELINE = '1'
  } else {
    delete env.MARKFLOW_HARNESS_BASELINE
  }

  if (inheritLogs) {
    console.log(`🏗️  Running ${target.label} build (${mode})`)
  }

  const start = performance.now()
  const result = spawnSync(target.command, target.args, {
    cwd: target.cwd,
    stdio: inheritLogs ? 'inherit' : 'ignore',
    env,
  })
  const duration = performance.now() - start

  if (result.status !== 0) {
    const message = `Harness build failed (target=${targetName}, mode=${mode})`
    if (inheritLogs) {
      console.error(message)
    }
    throw new Error(message)
  }

  if (inheritLogs) {
    console.log(`✅ Harness build (${target.label}, ${mode}) finished in ${duration.toFixed(2)}ms`)
  }

  return duration
}

function printUsage() {
  console.error(
    `Usage: node scripts/run-astro-harness.mjs [target] [mode]\n` +
      `Targets: ${validTargets.join(', ')} (default: ${defaultTarget})\n` +
      `Modes: ${validModes.join(', ')} (default: markflow)`,
  )
}

async function main() {
  const args = process.argv.slice(2)
  let targetArg
  let modeArg

  if (args.length === 0) {
    targetArg = defaultTarget
    modeArg = 'markflow'
  } else if (args.length === 1) {
    if (validTargets.includes(args[0])) {
      targetArg = args[0]
      modeArg = 'markflow'
    } else if (validModes.includes(args[0])) {
      targetArg = defaultTarget
      modeArg = args[0]
    } else {
      printUsage()
      process.exit(1)
    }
  } else {
    if (!validTargets.includes(args[0]) || !validModes.includes(args[1])) {
      printUsage()
      process.exit(1)
    }
    targetArg = args[0]
    modeArg = args[1]
  }

  try {
    runHarnessBuild(targetArg, modeArg)
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
