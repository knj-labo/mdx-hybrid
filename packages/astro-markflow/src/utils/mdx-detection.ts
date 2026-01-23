/**
 * MDX pattern detection utilities.
 * @module utils/mdx-detection
 */

import { stripFrontmatter } from './frontmatter.js';

/**
 * Detect MDX patterns that markdown-rs cannot parse correctly.
 * This includes:
 * - MDX import statements (import ... from '...')
 * - MDX export statements (export const ..., export default ...)
 * These are JavaScript constructs that the MDAST pipeline cannot handle.
 */
export function hasProblematicMdxPatterns(source: string): boolean {
  // Skip frontmatter when checking for imports/exports
  const content = stripFrontmatter(source);

  // Check for MDX import/export statements at the start of lines
  // These patterns match JavaScript import/export syntax
  const mdxPatterns = [
    /^import\s+(?:\{[^}]*\}|\*\s+as\s+\w+|\w+)\s+from\s+['"][^'"]+['"]/m,
    /^import\s+['"][^'"]+['"]/m,
    /^export\s+(?:const|let|var|function|class|default)\b/m,
  ];

  return mdxPatterns.some((pattern) => pattern.test(content));
}
