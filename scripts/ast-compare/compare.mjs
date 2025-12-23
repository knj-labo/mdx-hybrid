import { readFile } from 'node:fs/promises'
import { parseWithUnified } from './from-unified.mjs'
import { parseWithMarkflow } from './from-markflow.mjs'

async function main() {
  const args = parseArgs(process.argv)
  if (!args.file) {
    console.error('Usage: node scripts/ast-compare/compare.mjs --file <path>')
    process.exit(1)
  }

  const source = await readFile(args.file, 'utf8')
  const stripped = stripFrontmatter(source)
  const baseline = await parseWithUnified(stripped)
  const markflow = await parseWithMarkflow(stripped)

  const diff = findFirstDiff(baseline.blocks, markflow.blocks)
  if (diff) {
    console.log(JSON.stringify(diff, null, 2))
    process.exitCode = 1
  } else {
    console.log(JSON.stringify({ ok: true, blocks: baseline.blocks.length }, null, 2))
  }
}

function parseArgs(argv) {
  const out = { file: null, options: {} }
  for (let i = 2; i < argv.length; i++) {
    const arg = argv[i]
    if (arg === '--file' && argv[i + 1]) {
      out.file = argv[i + 1]
      i++
    }
  }
  return out
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

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((err) => {
    console.error(err.message || err)
    process.exit(1)
  })
}
