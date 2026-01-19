/**
 * Ensures Starlight <Steps> receives a single <ol> child.
 * If the slot already starts with <ol ...></ol>, it is left untouched.
 * Otherwise the inner HTML is wrapped in <ol><li>...</li></ol>.
 */
export function normalizeSteps(code: string): { code: string; changed: boolean } {
  if (!code || typeof code !== 'string') {
    return { code, changed: false };
  }

  const pattern = /<Steps(\s[^>]*)?>([\s\S]*?)<\/Steps>/g;
  let changed = false;

  const next = code.replace(pattern, (match, attrs = '', inner = '') => {
    const trimmed = inner.trim();
    if (trimmed.startsWith('<ol') && trimmed.endsWith('</ol>')) {
      return match; // already compliant
    }
    changed = true;
    return `<Steps${attrs}><ol><li>${inner}</li></ol></Steps>`;
  });

  return { code: next, changed };
}
