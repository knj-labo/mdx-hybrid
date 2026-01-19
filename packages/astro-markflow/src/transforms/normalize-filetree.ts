/**
 * Ensures Starlight <FileTree> receives a single <ul> child.
 * If the slot already starts with <ul ...></ul>, it is left untouched.
 * If the slot is empty, inject an empty <ul>.
 * Otherwise the inner HTML is wrapped in <ul>...</ul>.
 */
export function normalizeFileTree(code: string): { code: string; changed: boolean } {
  if (!code || typeof code !== 'string') {
    return { code, changed: false };
  }

  const pattern = /<FileTree(\s[^>]*)?>([\s\S]*?)<\/FileTree>/g;
  let changed = false;

  const next = code.replace(pattern, (match, attrs = '', inner = '') => {
    const trimmed = inner.trim();
    const containsLi = /<li[\s>]/i.test(trimmed);

    // Empty slot: inject ul with an empty li
    if (!trimmed) {
      changed = true;
      return `<FileTree${attrs}><ul><li></li></ul></FileTree>`;
    }

    // Already a ul wrapper
    if (trimmed.startsWith('<ul') && trimmed.endsWith('</ul>')) {
      if (containsLi) return match; // compliant
      // add a placeholder li
      changed = true;
      return `<FileTree${attrs}>${trimmed.replace('</ul>', '<li></li></ul>')}</FileTree>`;
    }

    // Wrap and ensure at least one li
    changed = true;
    if (containsLi) {
      return `<FileTree${attrs}><ul>${inner}</ul></FileTree>`;
    }
    return `<FileTree${attrs}><ul><li>${inner}</li></ul></FileTree>`;
  });

  return { code: next, changed };
}
