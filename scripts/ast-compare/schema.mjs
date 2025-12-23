export const BLOCK_TYPES = new Set([
  'heading',
  'paragraph',
  'list_item',
  'code',
  'blockquote',
  'mdx',
])

export function normalizeText(value) {
  return String(value ?? '')
    .replace(/\$\$([^$]+)\$\$/g, '$1')
    .replace(/\$([^$]+)\$/g, '$1')
    .replace(/\*\*/g, '')
    .replace(/__/g, '')
    .trim()
    .replace(/\s+/g, ' ')
}

export function makeHeading(depth, text) {
  return {
    type: 'heading',
    depth,
    text: normalizeText(text),
  }
}

export function makeParagraph(text) {
  return {
    type: 'paragraph',
    text: normalizeText(text),
  }
}

export function makeListItem(ordered, text) {
  return {
    type: 'list_item',
    ordered: Boolean(ordered),
    text: normalizeText(text),
  }
}

export function makeCode(lang, text) {
  return {
    type: 'code',
    lang: normalizeText(lang),
    text: normalizeText(text),
  }
}

export function makeBlockquote(text) {
  return {
    type: 'blockquote',
    text: normalizeText(text),
  }
}

export function makeMdx(text) {
  return {
    type: 'mdx',
    text: normalizeText(text),
  }
}
