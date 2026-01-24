import { parseFragment } from 'parse5'
import { compileIr } from '../../crates/napi/index.js'
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
  const result = compileIr(markdown, 'test.md', null, null)
  const html = result.html
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

  // Handle JSX Aside from compileIr: <Aside {...{"type": "note", "title": "Title"}}>
  if (tag === 'aside' && hasJsxSpreadAttr(node)) {
    const spreadAttrs = extractJsxSpreadAttrs(node)
    const asideType = spreadAttrs.type || ''
    const title = spreadAttrs.title || ''
    // Unified format: Aside type=<type> title=<title>
    const parts = ['Aside']
    if (asideType) parts.push(`type=${asideType}`)
    if (title) parts.push(`title=${title}`)
    blocks.push(makeMdx(parts.join(' ')))
    const body = collectText(node)
    if (body) {
      blocks.push(makeParagraph(body))
    }
    return
  }
  // Handle HTML aside with class="aside aside--note" (legacy format)
  if (tag === 'aside' && hasClass(node, 'aside') && !state.inBlockquote) {
    const asideType = extractAsideType(node)
    const title = extractAsideTitle(node)
    // Unified format: Aside type=<type> title=<title>
    const parts = ['Aside']
    if (asideType) parts.push(`type=${asideType}`)
    if (title) parts.push(`title=${title}`)
    blocks.push(makeMdx(parts.join(' ')))
    const body = extractAsideBody(node)
    if (body) {
      blocks.push(makeParagraph(body))
    }
    return
  }
  if (tag === 'ol' && (hasClass(node, 'steps') || hasAttr(node, 'steps'))) {
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
    blocks.push(makeCode(lang, collectRawTextExact(node)))
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

function hasJsxSpreadAttr(node) {
  // parse5 parses JSX spread like {...{"type": "note"}} as an attribute name starting with {
  const attrs = node.attrs || []
  return attrs.some((attr) => attr.name.startsWith('{...{'))
}

function extractJsxSpreadAttrs(node) {
  // parse5 splits JSX spread {...{"type": "note", "title": "Important"}} into multiple attrs:
  // Each attr.name contains a fragment: {...{"type":, "note",, "title":, "important"}}
  // We reconstruct and parse the JSON object
  const attrs = node.attrs || []
  // Collect all attr names that are part of the spread
  const parts = []
  let inSpread = false
  for (const attr of attrs) {
    if (attr.name.startsWith('{...{')) {
      inSpread = true
    }
    if (inSpread) {
      parts.push(attr.name)
      if (attr.name.includes('}}')) {
        break
      }
    }
  }
  // Reconstruct: {..."type": "note", "title": "important"}}
  const raw = parts.join(' ')
  // Extract JSON object from {...{...}}
  const jsonMatch = raw.match(/\{\.\.\.(\{.*\})\}/)
  if (!jsonMatch) return {}
  try {
    return JSON.parse(jsonMatch[1])
  } catch {
    // Manual extraction as fallback
    const result = {}
    const pairRegex = /"(\w+)":\s*"([^"]*)"/g
    let match
    while ((match = pairRegex.exec(raw)) !== null) {
      result[match[1]] = match[2]
    }
    return result
  }
}

function extractAsideType(node) {
  const classes = getAttr(node, 'class')
  if (!classes) return ''
  // Extract type from aside--note, aside--tip, aside--caution, etc.
  const match = classes.match(/\baside--(\w+)\b/)
  return match ? match[1] : ''
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

function collectRawTextExact(node) {
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
  return parts.join('')
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
