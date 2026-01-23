/**
 * MDX pattern detection utilities.
 * @module utils/mdx-detection
 */

import { stripFrontmatter } from './frontmatter.js';
import type { MdxImportHandlingOptions } from '../types.js';

/**
 * Strip code fences from content to avoid false positives.
 */
function stripCodeFences(content: string): string {
  // Remove fenced code blocks (``` or ~~~)
  return content.replace(/^(?:```|~~~)[^\n]*\n[\s\S]*?^(?:```|~~~)\s*$/gm, '');
}

/**
 * Convert a glob-like pattern to a regex.
 * Supports * as wildcard.
 */
function patternToRegex(pattern: string): RegExp {
  const escaped = pattern.replace(/[.+^${}()|[\]\\]/g, '\\$&');
  const withWildcard = escaped.replace(/\*/g, '.*');
  return new RegExp(`^${withWildcard}$`);
}

/**
 * Check if an import source matches any of the allowed patterns.
 */
function isAllowedImport(importSource: string, allowImports: string[]): boolean {
  return allowImports.some((pattern) => {
    const regex = patternToRegex(pattern);
    return regex.test(importSource);
  });
}

/**
 * Extract import sources from content.
 */
function extractImportSources(content: string): string[] {
  const sources: string[] = [];
  // Match: import ... from 'source' or import 'source'
  const importRegex = /^import\s+(?:(?:\{[^}]*\}|\*\s+as\s+\w+|\w+)\s+from\s+)?['"]([^'"]+)['"]/gm;
  let match;
  while ((match = importRegex.exec(content)) !== null) {
    if (match[1]) {
      sources.push(match[1]);
    }
  }
  return sources;
}

/**
 * Detect MDX patterns that markdown-rs cannot parse correctly.
 * This includes:
 * - MDX import statements (import ... from '...')
 * - MDX export statements (export const ..., export default ...)
 * These are JavaScript constructs that the MDAST pipeline cannot handle.
 *
 * @param source - The markdown/MDX source content
 * @param options - MDX handling options
 */
export function hasProblematicMdxPatterns(
  source: string,
  options?: MdxImportHandlingOptions
): boolean {
  // Skip frontmatter when checking for imports/exports
  let content = stripFrontmatter(source);

  // Optionally strip code fences (default: true when options provided)
  const ignoreCodeFences = options?.ignoreCodeFences ?? true;
  if (ignoreCodeFences) {
    content = stripCodeFences(content);
  }

  // Check for export statements (always problematic)
  const exportPattern = /^export\s+(?:const|let|var|function|class|default)\b/m;
  if (exportPattern.test(content)) {
    return true;
  }

  // Check for import statements
  const importPatterns = [
    /^import\s+(?:\{[^}]*\}|\*\s+as\s+\w+|\w+)\s+from\s+['"][^'"]+['"]/m,
    /^import\s+['"][^'"]+['"]/m,
  ];

  const hasImports = importPatterns.some((pattern) => pattern.test(content));
  if (!hasImports) {
    return false;
  }

  // If no allowed imports configured, any import is problematic
  const allowImports = options?.allowImports;
  if (!allowImports || allowImports.length === 0) {
    return true;
  }

  // Check if all imports are from allowed sources
  const importSources = extractImportSources(content);
  const hasDisallowedImports = importSources.some(
    (source) => !isAllowedImport(source, allowImports)
  );

  return hasDisallowedImports;
}
