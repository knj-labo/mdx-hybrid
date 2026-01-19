import { readdirSync, readFileSync, statSync } from 'node:fs'
import { resolve, relative } from 'node:path'
import { createRequire } from 'node:module'
import matter from 'gray-matter'
import { parse as markflowParse, parseWithOptions } from '../../../../crates/napi/index.js'

const DOCS_DIR = resolve(
  new URL('../content/docs/', import.meta.url).pathname,
)

// ESM-compatible require helper for baseline compiler deps
const require = createRequire(import.meta.url)

export default function markflowContentPlugin() {
  const useBaseline =
    process.env.MARKFLOW_HARNESS_BASELINE === '1' ||
    process.env.MARKFLOW_DISABLE === '1'
  const skipFrontmatter = process.env.MARKFLOW_HARNESS_SKIP_FRONTMATTER === '1'
  const disableSmartypants =
    process.env.MARKFLOW_HARNESS_DISABLE_SMARTYPANTS === '1'

  return {
    name: 'markflow-content-plugin',
    resolveId(id) {
      if (id === 'virtual:markflow-docs') {
        return '\0markflow-docs'
      }
      return null
    },
    async load(id) {
      if (id !== '\0markflow-docs') return null

      const usingBaseline =
        useBaseline ||
        typeof markflowParse !== 'function' ||
        typeof parseWithOptions !== 'function';

      if (usingBaseline) {
        return 'export const docs = [];'
      }

      const files = collectMdxFiles(DOCS_DIR)
      const docs = []
      const compiler = useBaseline ? await createBaselineCompiler() : null

      for (const file of files) {
        const raw = readFileSync(file.fullPath, 'utf8')
        const { content, data } = matter(raw)
        let html
        if (useBaseline && compiler) {
          const baselineSource = preprocessBaseline(content)
          const processed = await compiler.process(baselineSource)
          html = String(processed)
        } else {
          html = disableSmartypants
            ? parseWithOptions(raw, {
                enforceImgLoadingLazy: true,
                enableSmartypants: false,
              })
            : markflowParse(raw)
          if (skipFrontmatter) {
            html = stripFrontmatterHtml(html)
          }
        }

        docs.push({
          slug: file.slug,
          title: data.title ?? deriveTitle(content),
          description: data.description ?? '',
          html,
        })
      }

      return `export const docs = ${JSON.stringify(docs)};`
    },
  }
}

function collectMdxFiles(dir, base = dir) {
  const entries = readdirSync(dir)
  const files = []
  for (const entry of entries) {
    const fullPath = resolve(dir, entry)
    const stats = statSync(fullPath)
    if (stats.isDirectory()) {
      files.push(...collectMdxFiles(fullPath, base))
    } else if (entry.endsWith('.mdx')) {
      const rel = relative(base, fullPath)
      files.push({
        fullPath,
        slug: rel.replace(/\\/g, '/').replace(/\.mdx$/, ''),
      })
    }
  }
  return files
}

async function createBaselineCompiler() {
  // Use CJS require to avoid Vite SSR module runner issues in harness baseline mode
  const { unified } = require('unified')
  const remarkParse = require('remark-parse')
  const remarkGfm = require('remark-gfm')
  const remarkSmartypants = require('remark-smartypants')
  const remarkRehype = require('remark-rehype')
  const rehypeSlug = require('rehype-slug')
  const rehypeStringify = require('rehype-stringify')

  return unified()
    .use(remarkParse)
    .use(remarkGfm)
    .use(remarkSmartypants, { dashes: false })
    .use(remarkRehype)
    .use(rehypeSlug)
    .use(rehypeStringify)
}

function deriveTitle(markdown) {
  const match = markdown.match(/^#\s+(.+)$/m)
  return match ? match[1].trim() : 'Untitled'
}

function preprocessBaseline(markdown) {
  const lines = markdown.split(/\r?\n/)
  const out = []
  let fence = null
  // Harness normalization: keep ESM filtering line-based for stability.
  for (const line of lines) {
    const fenceMatch = line.match(/^\s*(?:>\s*)?(```|~~~)/)
    if (fenceMatch) {
      const marker = fenceMatch[1]
      fence = fence === marker ? null : fence ?? marker
      out.push(line)
      continue
    }

    if (fence) {
      out.push(line)
      continue
    }

    if (isMdxEsmLine(line)) {
      continue
    }

    const openMatch = line.match(
      /^(\s*(?:>\s*)*)(:{3,4})([A-Za-z][\w-]*)(?:\s*\[([^\]]+)\])?/,
    )
    if (openMatch) {
      const prefix = openMatch[1] ?? ''
      const rawTitle = openMatch[4]?.trim()
      if (rawTitle) {
        out.push(`${prefix}${rawTitle}`)
      }
      continue
    }

    const closeMatch = line.match(/^(\s*(?:>\s*)*)(:{3,4})\s*$/)
    if (closeMatch) {
      continue
    }

    out.push(line)
  }

  return out.join('\n')
}

function isMdxEsmLine(line) {
  const trimmed = line.trim()
  if (!/^(import|export)\s/.test(trimmed)) return false
  if (trimmed.startsWith('export {') || trimmed.startsWith('export *')) return true
  if (trimmed.startsWith('import') && /\sfrom\s/.test(trimmed)) return true
  return trimmed.startsWith('export') && /\sfrom\s/.test(trimmed)
}

function stripFrontmatterHtml(html) {
  if (!html) return html
  return html.replace(
    /<pre\s+class="frontmatter"[^>]*>[\s\S]*?<\/pre>/gi,
    '',
  )
}
