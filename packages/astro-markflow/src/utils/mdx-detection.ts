/**
 * MDX pattern detection utilities.
 * @module utils/mdx-detection
 */

/**
 * Detect MDX patterns that markdown-rs cannot parse correctly.
 * This includes:
 * - MDX import statements (import ... from '...')
 * - MDX export statements (export const ..., export default ...)
 * These are JavaScript constructs that the MDAST pipeline cannot handle.
 */
export function hasProblematicMdxPatterns(source: string): boolean {
  // Skip frontmatter when checking for imports/exports
  const frontmatterMatch = source.match(/^---\r?\n[\s\S]*?\r?\n---\r?\n?/);
  const contentStart = frontmatterMatch ? frontmatterMatch[0].length : 0;
  const content = source.slice(contentStart);

  // Check for MDX import/export statements at the start of lines
  // These patterns match JavaScript import/export syntax
  const mdxPatterns = [
    /^import\s+(?:\{[^}]*\}|\*\s+as\s+\w+|\w+)\s+from\s+['"][^'"]+['"]/m,
    /^import\s+['"][^'"]+['"]/m,
    /^export\s+(?:const|let|var|function|class|default)\b/m,
  ];

  return mdxPatterns.some((pattern) => pattern.test(content));
}
