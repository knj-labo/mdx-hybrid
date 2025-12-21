#!/usr/bin/env node
import { spawn } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, resolve } from 'node:path'

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const harnessDir = resolve(repoRoot, 'fixtures/integration/astro-harness')

/**
 * Run the Astro harness in the given mode.
 * @param {'markflow'|'baseline'} mode
 * @param {{skipInstall?: boolean}} opts
 * @returns {Promise<{mode: string, durationMs: number}>}
 */
export async function runHarness(mode = 'markflow', opts = {}) {
  if (!['markflow', 'baseline'].includes(mode)) {
    throw new Error(`Unknown mode "${mode}", expected "markflow" or "baseline"`)
  }

  const skipInstall = !!opts.skipInstall
  const env = { ...process.env }

  const start = process.hrtime.bigint()

  if (!skipInstall) {
    await exec('pnpm', ['install', '--frozen-lockfile'], {
      cwd: harnessDir,
      env,
    })
  }

  const script = mode === 'baseline' ? 'build:baseline' : 'build'
  await exec('pnpm', ['run', script], {
    cwd: harnessDir,
    env,
  })

  const end = process.hrtime.bigint()
  const durationMs = Number(end - start) / 1_000_000
  return { mode, durationMs }
}

function exec(cmd, args, { cwd, env }) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(cmd, args, {
      cwd,
      env,
      stdio: 'inherit',
    })
    child.on('close', (code) => {
      if (code === 0) return resolvePromise()
      reject(new Error(`${cmd} ${args.join(' ')} exited with code ${code}`))
    })
  })
}

async function main() {
  const [, , modeArg = 'markflow', ...rest] = process.argv
  const skipInstall = rest.includes('--skip-install')
  try {
    const result = await runHarness(modeArg, { skipInstall })
    console.log(JSON.stringify(result, null, 2))
  } catch (err) {
    console.error(err.message || err)
    process.exit(1)
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main()
}
