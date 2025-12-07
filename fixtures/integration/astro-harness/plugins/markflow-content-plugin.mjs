import { readdirSync, readFileSync, statSync } from 'node:fs'
import { resolve, relative } from 'node:path'
import matter from 'gray-matter'
import { parse as markflowParse } from '../../../../crates/napi/index.js'

const DOCS_DIR = resolve(
  new URL('../content/docs/', import.meta.url).pathname,
)

export default function markflowContentPlugin() {
  const useBaseline = process.env.MARKFLOW_HARNESS_BASELINE === '1'

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

      const files = collectMdxFiles(DOCS_DIR)
      const docs = []
      const compiler = useBaseline ? await createBaselineCompiler() : null

      for (const file of files) {
        const raw = readFileSync(file.fullPath, 'utf8')
        const { content, data } = matter(raw)
        let html
        if (useBaseline && compiler) {
          const processed = await compiler.process(content)
          html = String(processed)
        } else {
          html = markflowParse(raw)
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
  const { unified } = await import('unified')
  const { default: remarkParse } = await import('remark-parse')
  const { default: remarkGfm } = await import('remark-gfm')
  const { default: remarkRehype } = await import('remark-rehype')
  const { default: rehypeStringify } = await import('rehype-stringify')

  return unified()
    .use(remarkParse)
    .use(remarkGfm)
    .use(remarkRehype)
    .use(rehypeStringify)
}

function deriveTitle(markdown) {
  const match = markdown.match(/^#\s+(.+)$/m)
  return match ? match[1].trim() : 'Untitled'
}
