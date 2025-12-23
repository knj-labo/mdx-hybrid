import { readdir } from 'node:fs/promises'
import { resolve } from 'node:path'
import { parseWithUnified } from './from-unified.mjs'
import { parseWithMarkflow } from './from-markflow.mjs'

const DEFAULT_DIR = resolve(
  process.cwd(),
  'fixtures/integration/astro-harness/content/docs',
)

async function main() {
  const args = parseArgs(process.argv)
  const dir = args.dir || DEFAULT_DIR
  const files = await collectMdx(dir)
  for (const file of files) {
    try {
      const source = await readFile(file)
      const stripped = stripFrontmatter(source)
      const baseline = await parseWithUnified(stripped)
      const markflow = await parseWithMarkflow(stripped)
      const diff = findFirstDiff(baseline.blocks, markflow.blocks)
      if (diff) {
        console.log(
          JSON.stringify(
            {
              file,
              ...diff,
            },
            null,
            2,
          ),
        )
        process.exitCode = 1
        return
      }
    } catch (err) {
      console.log(
        JSON.stringify(
          {
            file,
            error: err?.message ?? String(err),
          },
          null,
          2,
        ),
      )
      process.exitCode = 1
      return
    }
  }
  console.log(JSON.stringify({ ok: true, files: files.length }, null, 2))
}

function parseArgs(argv) {
  const out = { dir: null }
  for (let i = 2; i < argv.length; i++) {
    const arg = argv[i]
    if (arg === '--dir' && argv[i + 1]) {
      out.dir = argv[i + 1]
      i++
    }
  }
  return out
}

async function readFile(path) {
  const { readFile } = await import('node:fs/promises')
  return readFile(path, 'utf8')
}

async function collectMdx(dir) {
  const entries = await readdir(dir, { withFileTypes: true })
  const files = []
  for (const entry of entries) {
    const full = resolve(dir, entry.name)
    if (entry.isDirectory()) {
      files.push(...(await collectMdx(full)))
    } else if (entry.name.endsWith('.mdx')) {
      files.push(full)
    }
  }
  return files
}

function findFirstDiff(left, right) {
  const max = Math.max(left.length, right.length)
  for (let i = 0; i < max; i++) {
    const a = left[i]
    const b = right[i]
    if (!a || !b) {
      return { index: i, left: a ?? null, right: b ?? null }
    }
    if (JSON.stringify(a) !== JSON.stringify(b)) {
      return { index: i, left: a, right: b }
    }
  }
  return null
}

function stripFrontmatter(source) {
  if (!source.startsWith('---')) return source
  const lines = source.split(/\r?\n/)
  if (lines[0].trim() !== '---') return source
  for (let i = 1; i < lines.length; i++) {
    const line = lines[i].trim()
    if (line === '---' || line === '...') {
      return lines.slice(i + 1).join('\n')
    }
  }
  return source
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((err) => {
    console.error(err.message || err)
    process.exit(1)
  })
}
