#!/usr/bin/env node
import { promises as fs } from 'node:fs'
import { dirname, resolve, relative } from 'node:path'
import { fileURLToPath } from 'node:url'
import { runHarness } from './run-astro-harness.mjs'

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const defaultSummary = resolve(
  repoRoot,
  'fixtures/integration/astro-harness/harness-summary.json',
)
const harnessDir = resolve(
  repoRoot,
  'fixtures/integration/astro-harness',
)

function parseArgs(argv) {
  const args = { summary: defaultSummary, skipInstall: false, mode: 'time' }
  for (let i = 2; i < argv.length; i++) {
    const arg = argv[i]
    if (arg === '--summary' && argv[i + 1]) {
      args.summary = resolve(repoRoot, argv[i + 1])
      i++
    } else if (arg === '--skip-install') {
      args.skipInstall = true
    } else if (arg === '--mode' && argv[i + 1]) {
      args.mode = argv[i + 1]
      i++
    } else if (arg.startsWith('--mode=')) {
      args.mode = arg.split('=')[1]
    }
  }
  return args
}

async function main() {
  const { summary, skipInstall, mode } = parseArgs(process.argv)

  const distBaseline = resolve(harnessDir, 'dist-baseline')
  const distMarkflow = resolve(harnessDir, 'dist-markflow')

  // clean previous outputs
  await Promise.all([
    fs.rm(distBaseline, { recursive: true, force: true }),
    fs.rm(distMarkflow, { recursive: true, force: true }),
  ])

  const baseline = await runHarness('baseline', { skipInstall })
  await moveDist('baseline', distBaseline)

  const markflow = await runHarness('markflow', { skipInstall })
  await moveDist('markflow', distMarkflow)

  let semantic = null
  if (mode === 'semantic') {
    semantic = await compareSemantic(distBaseline, distMarkflow)
  }

  const report = {
    timestamp: new Date().toISOString(),
    baselineMs: Math.round(baseline.durationMs),
    markflowMs: Math.round(markflow.durationMs),
    mode,
    semantic,
    note:
      mode === 'semantic'
        ? 'semantic diff: pass if differences=0'
        : 'build-only; no HTML diff',
  }

  await fs.mkdir(dirname(summary), { recursive: true })
  await fs.writeFile(summary, JSON.stringify(report, null, 2))
  console.log(`Harness summary written to ${summary}`)
  console.log(report)

  if (semantic && semantic.differences > 0) {
    process.exitCode = 1
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((err) => {
    console.error(err.message || err)
    process.exit(1)
  })
}

async function moveDist(label, targetDir) {
  const dist = resolve(harnessDir, 'dist')
  const exists = await fs
    .stat(dist)
    .then(() => true)
    .catch(() => false)
  if (!exists) {
    throw new Error(`dist not found after ${label} build at ${dist}`)
  }
  await fs.rm(targetDir, { recursive: true, force: true })
  await fs.rename(dist, targetDir)
}

async function compareSemantic(dirA, dirB) {
  const filesA = await collectHtml(dirA)
  const filesB = await collectHtml(dirB)
  const all = new Set([...filesA.keys(), ...filesB.keys()])

  const diffs = []
  for (const rel of all) {
    const a = filesA.get(rel)
    const b = filesB.get(rel)
    if (a === undefined || b === undefined) {
      diffs.push({ file: rel, reason: 'missing-in-one-side' })
      continue
    }
    const na = normalizeHtml(a)
    const nb = normalizeHtml(b)
    if (na !== nb) {
      const sa = stripTags(na)
      const sb = stripTags(nb)
      if (sa !== sb) {
        diffs.push({
          file: rel,
          reason: 'content-diff',
          ...summarizeDiff(sa, sb),
        })
      }
    }
  }

  return {
    compared: all.size,
    differences: diffs.length,
    samples: diffs.slice(0, 1),
  }
}

async function collectHtml(root) {
  const out = new Map()
  async function walk(dir) {
    const entries = await fs.readdir(dir, { withFileTypes: true })
    for (const entry of entries) {
      const full = resolve(dir, entry.name)
      if (entry.isDirectory()) {
        await walk(full)
      } else if (entry.name.endsWith('.html')) {
        const rel = relative(root, full)
        const data = await fs.readFile(full, 'utf8')
        out.set(rel, data)
      }
    }
  }
  await walk(root)
  return out
}

function normalizeHtml(html) {
  let processed = html
  const frontmatterRe = /<pre\s+class="frontmatter"[^>]*>[\s\S]*?<\/pre>/gi
  while (frontmatterRe.test(processed)) {
    processed = processed.replace(frontmatterRe, '')
  }
  const asideOpenRe = /<aside\b[^>]*class="[^"]*\baside[^"]*"[^>]*>/gi
  const asideCloseRe = /<\/aside>/gi
  while (asideOpenRe.test(processed)) {
    processed = processed.replace(asideOpenRe, '')
  }
  processed = processed.replace(asideCloseRe, '')
  processed = processed
    .replace(/<span\b[^>]*class="[^"]*\bmath-inline\b[^"]*"[^>]*>/gi, '')
    .replace(/<\/span>/gi, '')
  processed = processed
    .replace(/<pre><code\b[^>]*>/gi, '<code>')
    .replace(/<\/code><\/pre>/gi, '</code>')

  const noComments = processed.replace(/<!--[\s\S]*?-->/g, '')
  const tagRe = /<[^>]+>/g
  let out = ''
  let last = 0
  let m
  let codeDepth = 0
  while ((m = tagRe.exec(noComments))) {
    let text = noComments.slice(last, m.index)
    if (codeDepth === 0) {
      text = stripDirectiveMarkers(text)
    }
    out += normalizeText(text)

    const tag = m[0]
    const lower = tag.toLowerCase()
    if (
      lower.startsWith('<code') ||
      lower.startsWith('<pre') ||
      lower.startsWith('<script') ||
      lower.startsWith('<style')
    ) {
      codeDepth += 1
    } else if (
      lower.startsWith('</code') ||
      lower.startsWith('</pre') ||
      lower.startsWith('</script') ||
      lower.startsWith('</style')
    ) {
      codeDepth = Math.max(0, codeDepth - 1)
    }

    out += normalizeTag(tag)
    last = m.index + m[0].length
  }
  let tail = noComments.slice(last)
  if (codeDepth === 0) {
    tail = stripDirectiveMarkers(tail)
  }
  out += normalizeText(tail)
  return out.trim().replace(/\s+/g, ' ')
}

function stripTags(text) {
  const blockTagRe =
    /<\/?(?:p|div|section|article|header|footer|main|aside|nav|h[1-6]|ol|ul|li|table|thead|tbody|tfoot|tr|td|th|pre|blockquote|figure|figcaption|hr|br)(?:\s[^>]*)?>/gi
  return decodeHtmlEntities(
    text
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(blockTagRe, ' ')
    .replace(/<[^>]*>/g, '')
    .replace(/\$\$([^$]+)\$\$/g, '$1')
    .replace(/\$([^$]+)\$/g, '$1')
  )
    .trim()
    .replace(/\s+/g, ' ')
}

function decodeHtmlEntities(text) {
  return text
    .replace(/&nbsp;/g, ' ')
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;|&apos;/g, "'")
    .replace(/&#x([0-9a-fA-F]+);/g, (_, hex) =>
      String.fromCharCode(parseInt(hex, 16)),
    )
    .replace(/&#(\d+);/g, (_, num) =>
      String.fromCharCode(parseInt(num, 10)),
    )
}

function normalizeText(text) {
  return text.replace(/\s+/g, ' ')
}

function stripDirectiveMarkers(text) {
  return text
    .replace(/(^|[\s>]):::{3,4}[A-Za-z][\w-]*(?:\[[^\]]*])?/g, '$1')
    .replace(/(^|[\s>]):::{3,4}(?=\s|$)/g, '$1')
}

function normalizeTag(tag) {
  const closing = /^<\s*\/\s*([^\s>]+)\s*>/.exec(tag)
  if (closing) {
    return `</${closing[1].toLowerCase()}>`
  }

  const open = /^<\s*([^\s/>]+)([^>]*)>/.exec(tag)
  if (!open) return tag

  const name = open[1].toLowerCase()
  const attrPart = open[2] || ''
  const attrs = []
  const attrRe =
    /([^\s=\/>]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/g
  let m
  while ((m = attrRe.exec(attrPart))) {
    const key = m[1].toLowerCase()
    const val = m[2] ?? m[3] ?? m[4] ?? ''
    attrs.push([key, val])
  }
  attrs.sort((a, b) => a[0].localeCompare(b[0]))
  const attrStr = attrs
    .map(([k, v]) => (v === '' ? k : `${k}="${v}"`))
    .join(' ')
  const selfClosing = /\/\s*>$/.test(tag)
  return `<${name}${attrStr ? ' ' + attrStr : ''}${selfClosing ? '/>' : '>'}`
}

function summarizeDiff(a, b) {
  const maxPreview = 80
  const min = Math.min(a.length, b.length)
  let idx = 0
  while (idx < min && a[idx] === b[idx]) idx++
  const start = Math.max(0, idx - 30)
  const endA = Math.min(a.length, idx + maxPreview)
  const endB = Math.min(b.length, idx + maxPreview)
  return {
    diffIndex: idx,
    previewA: a.slice(start, endA),
    previewB: b.slice(start, endB),
  }
}
