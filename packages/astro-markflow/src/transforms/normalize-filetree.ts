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
    if (!trimmed) {
      changed = true;
      return `<FileTree${attrs}><ul></ul></FileTree>`;
    }
    if (trimmed.startsWith('<ul') && trimmed.endsWith('</ul>')) {
      return match; // already compliant
    }
    changed = true;
    return `<FileTree${attrs}><ul>${inner}</ul></FileTree>`;
  });

  return { code: next, changed };
}
