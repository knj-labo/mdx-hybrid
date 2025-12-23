import { unified } from 'unified'
import remarkParse from 'remark-parse'
import remarkGfm from 'remark-gfm'
import remarkMdx from 'remark-mdx'
import {
  makeBlockquote,
  makeCode,
  makeHeading,
  makeListItem,
  makeMdx,
  makeParagraph,
  normalizeText,
} from './schema.mjs'

export async function parseWithUnified(markdown) {
  let tree
  try {
    tree = unified()
      .use(remarkParse)
      .use(remarkGfm)
      .use(remarkMdx)
      .parse(markdown)
  } catch (err) {
    tree = unified().use(remarkParse).use(remarkGfm).parse(markdown)
  }

  const blocks = []
  walk(
    tree,
    { listType: null, inListItem: false, inBlockquote: false },
    (node, state) => {
    switch (node.type) {
      case 'heading':
        blocks.push(makeHeading(node.depth, extractText(node)))
        break
      case 'paragraph':
        if (!state.inListItem && !state.inBlockquote) {
          const text = extractText(node)
          if (!text.includes(':::')) {
            blocks.push(makeParagraph(text))
          }
        }
        break
      case 'listItem':
        blocks.push(makeListItem(state.listType === 'ol', extractText(node)))
        break
      case 'code':
        blocks.push(makeCode(node.lang || '', node.value || ''))
        break
      case 'blockquote':
        blocks.push(makeBlockquote(stripDirectiveMarkers(extractText(node))))
        break
      case 'mdxJsxFlowElement':
      case 'mdxJsxTextElement':
        blocks.push(makeMdx(formatMdxElement(node)))
        break
      default:
        break
    }
  })

  return { blocks }
}

function walk(node, state, visit) {
  if (!node || typeof node !== 'object') return
  visit(node, state)
  let nextState = state
  if (node.type === 'list') {
    nextState = { ...state, listType: node.ordered ? 'ol' : 'ul' }
  }
  if (node.type === 'listItem') {
    nextState = { ...nextState, inListItem: true }
  }
  if (node.type === 'blockquote') {
    nextState = { ...nextState, inBlockquote: true }
  }
  if (Array.isArray(node.children)) {
    for (const child of node.children) {
      walk(child, nextState, visit)
    }
  }
}

function extractText(node) {
  const parts = []
  walk(node, { listType: null }, (child) => {
    if (child.type === 'text') {
      parts.push(child.value)
    } else if (child.type === 'inlineCode') {
      parts.push(child.value)
    }
  })
  return normalizeText(parts.join(' '))
}

function stripDirectiveMarkers(text) {
  return text
    .replace(/:{3,4}[A-Za-z][\w-]*(?:\[[^\]]*])?/g, '')
    .replace(/:{3,4}/g, '')
}

function formatMdxElement(node) {
  const name = node.name || 'mdx'
  if (name === 'Steps') return 'Steps'
  const attrs = Array.isArray(node.attributes)
    ? node.attributes
        .map((attr) => {
          if (!attr || !attr.name) return null
          if (typeof attr.value === 'string') {
            return `${attr.name}=${attr.value}`
          }
          if (attr.value && typeof attr.value.value === 'string') {
            return `${attr.name}=${attr.value.value}`
          }
          return attr.name
        })
        .filter(Boolean)
    : []
  return `${name}${attrs.length ? ' ' + attrs.join(' ') : ''}`
}
