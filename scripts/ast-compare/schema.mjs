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

export function normalizeCodeText(value) {
  return String(value ?? '').replace(/\r\n?/g, '\n')
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
    text: normalizeCodeText(text),
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
    // Lowercase for case-insensitive comparison (parse5 lowercases attributes)
    text: normalizeText(text).toLowerCase(),
  }
}
