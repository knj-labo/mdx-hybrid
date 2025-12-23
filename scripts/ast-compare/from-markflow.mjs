import { parseFragment } from 'parse5'
import { parse } from '../../crates/napi/index.js'
import {
  makeBlockquote,
  makeCode,
  makeHeading,
  makeListItem,
  makeMdx,
  makeParagraph,
  normalizeText,
} from './schema.mjs'

const HTML_TAGS = new Set([
  'html',
  'head',
  'body',
  'main',
  'article',
  'section',
  'header',
  'footer',
  'nav',
  'aside',
  'div',
  'span',
  'p',
  'pre',
  'code',
  'blockquote',
  'h1',
  'h2',
  'h3',
  'h4',
  'h5',
  'h6',
  'ul',
  'ol',
  'li',
  'table',
  'thead',
  'tbody',
  'tfoot',
  'tr',
  'td',
  'th',
  'em',
  'strong',
  'a',
  'img',
  'hr',
  'br',
])

const CANONICAL_COMPONENTS = new Map([
  ['filetreeitem', 'FileTreeItem'],
  ['filetree', 'FileTree'],
  ['tabs', 'Tabs'],
  ['steps', 'Steps'],
  ['callout', 'Callout'],
])

export function parseWithMarkflow(markdown) {
  const html = parse(markdown)
  const fragment = parseFragment(html)
  const blocks = []
  walk(fragment, blocks, {
    listType: null,
    inPre: false,
    inListItem: false,
    inBlockquote: false,
  })
  return { blocks }
}

function walk(node, blocks, state) {
  if (!node || typeof node !== 'object') return

  const tag = getTagName(node)
  if (tag && tag.startsWith('#')) {
    const children = node.childNodes || []
    for (const child of children) {
      walk(child, blocks, state)
    }
    return
  }
  const nextState = { ...state }

  if (tag === 'ol') nextState.listType = 'ol'
  if (tag === 'ul') nextState.listType = 'ul'
  if (tag === 'pre') nextState.inPre = true
  if (tag === 'li') nextState.inListItem = true
  if (tag === 'blockquote') nextState.inBlockquote = true

  if (tag === 'aside' && hasClass(node, 'aside') && !state.inBlockquote) {
    if (hasClass(node, 'aside--note')) {
      return
    }
    const title = extractAsideTitle(node)
    const label = title ? `Aside title=${title}` : 'Aside'
    blocks.push(makeMdx(label))
    const body = extractAsideBody(node)
    if (body) {
      blocks.push(makeParagraph(body))
    }
    return
  } else if (tag === 'ol' && (hasClass(node, 'steps') || hasAttr(node, 'steps'))) {
    blocks.push(makeMdx('Steps'))
  } else if (tag === 'div' && hasClass(node, 'tabs')) {
    blocks.push(makeMdx('Tabs'))
  } else if (tag === 'ul' && hasClass(node, 'filetree')) {
    blocks.push(makeMdx('FileTree'))
  } else if (tag === 'div' && hasAttr(node, 'slot')) {
    blocks.push(makeMdx(formatSlotTag(node)))
    const codeBlocks = extractFencedCodeBlocks(collectRawText(node))
    for (const block of codeBlocks) {
      blocks.push(makeCode(block.lang, block.text))
    }
    return
  } else if (tag && !HTML_TAGS.has(tag)) {
    blocks.push(makeMdx(formatTag(node)))
    const body = collectText(node)
    if (body) {
      blocks.push(makeParagraph(body))
    }
  } else if (tag && tag.match(/^h[1-6]$/)) {
    blocks.push(makeHeading(Number(tag[1]), collectText(node)))
  } else if (tag === 'p') {
    if (!state.inListItem && !state.inBlockquote) {
      blocks.push(makeParagraph(collectText(node)))
    }
  } else if (tag === 'li') {
    blocks.push(makeListItem(nextState.listType === 'ol', collectText(node)))
  } else if (tag === 'blockquote') {
    blocks.push(makeBlockquote(collectText(node)))
  } else if (tag === 'code' && (state.inPre || parentIsPre(node))) {
    const lang = extractLanguage(node)
    blocks.push(makeCode(lang, collectText(node)))
  }

  const children = node.childNodes || []
  for (const child of children) {
    walk(child, blocks, nextState)
  }
}

function getTagName(node) {
  return node.tagName || node.nodeName || ''
}

function parentIsPre(node) {
  return node.parentNode && getTagName(node.parentNode) === 'pre'
}

function collectText(node) {
  const parts = []
  const walker = (child) => {
    if (!child || typeof child !== 'object') return
    if (child.nodeName === '#text') {
      parts.push(child.value)
    }
    const kids = child.childNodes || []
    for (const kid of kids) walker(kid)
  }
  walker(node)
  return normalizeText(parts.join(' '))
}

function extractLanguage(node) {
  const attrs = node.attrs || []
  const classAttr = attrs.find((attr) => attr.name === 'class')
  if (!classAttr) return ''
  const match = classAttr.value.match(/language-([\w-]+)/i)
  return match ? match[1] : ''
}

function formatTag(node) {
  const rawTag = getTagName(node)
  const tag = CANONICAL_COMPONENTS.get(rawTag) ?? rawTag
  const attrs = (node.attrs || []).map((attr) => {
    if (!attr || !attr.name) return null
    return `${attr.name}=${attr.value ?? ''}`
  })
  const filtered = attrs.filter(Boolean)
  return `${tag}${filtered.length ? ' ' + filtered.join(' ') : ''}`
}

function formatSlotTag(node) {
  const slot = getAttr(node, 'slot')
  const title = getAttr(node, 'data-title')
  const parts = ['div']
  if (slot) parts.push(`slot=${slot}`)
  if (title) parts.push(`data-title=${title}`)
  return parts.join(' ')
}

function hasAttr(node, name) {
  return Boolean(getAttr(node, name))
}

function getAttr(node, name) {
  const attrs = node.attrs || []
  const found = attrs.find((attr) => attr.name === name)
  return found ? found.value : null
}

function hasClass(node, className) {
  const value = getAttr(node, 'class')
  if (!value) return false
  return value.split(/\s+/).includes(className)
}

function extractAsideTitle(node) {
  const children = node.childNodes || []
  for (const child of children) {
    if (getTagName(child) === 'div' && hasClass(child, 'aside__title')) {
      return collectText(child)
    }
  }
  return ''
}

function extractAsideBody(node) {
  const children = node.childNodes || []
  const parts = []
  for (const child of children) {
    if (getTagName(child) === 'div' && hasClass(child, 'aside__title')) {
      continue
    }
    parts.push(collectText(child))
  }
  return normalizeText(parts.join(' '))
}

function collectRawText(node) {
  const parts = []
  const walker = (child) => {
    if (!child || typeof child !== 'object') return
    if (child.nodeName === '#text') {
      parts.push(child.value)
    }
    const kids = child.childNodes || []
    for (const kid of kids) walker(kid)
  }
  walker(node)
  return parts.join('\n')
}

function extractFencedCodeBlocks(text) {
  const lines = text.split(/\r?\n/)
  const blocks = []
  let inFence = false
  let lang = ''
  let buffer = []
  for (const line of lines) {
    const fence = line.match(/^\s*```([^`]*)$/)
    if (fence) {
      if (!inFence) {
        inFence = true
        lang = fence[1].trim()
        buffer = []
      } else {
        blocks.push({ lang, text: buffer.join('\n') })
        inFence = false
        lang = ''
        buffer = []
      }
      continue
    }
    if (inFence) buffer.push(line)
  }
  return blocks
}
