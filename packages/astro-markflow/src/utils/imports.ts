/**
 * Import statement manipulation utilities
 * @module utils/imports
 */

import { stripCodeFences } from './mdx-detection.js';

/**
 * Collect all imported names from JavaScript/JSX code.
 * Handles default imports, namespace imports, and named imports.
 *
 * @example
 * const code = `
 *   import React from 'react';
 *   import { useState, useEffect } from 'react';
 *   import * as utils from './utils';
 * `;
 * const names = collectImportedNames(code);
 * // Set { 'React', 'useState', 'useEffect', 'utils' }
 */
export function collectImportedNames(code: string): Set<string> {
  const imported = new Set<string>();
  if (!code || typeof code !== 'string') {
    return imported;
  }
  // Strip code fences to avoid false positives from code examples
  const codeWithoutFences = stripCodeFences(code);
  const lines = codeWithoutFences.split(/\r?\n/);
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed.startsWith('import ') || trimmed.startsWith('import(')) {
      continue;
    }

    // Default import: import Foo from 'module'
    const defaultMatch = trimmed.match(
      /^import\s+([A-Za-z$_][\w$]*)\s*(?:,|\s+from\s)/
    );
    if (defaultMatch?.[1]) {
      imported.add(defaultMatch[1]);
    }

    // Namespace import: import * as Foo from 'module'
    const namespaceMatch = trimmed.match(
      /^import\s+\*\s+as\s+([A-Za-z$_][\w$]*)\s+from/
    );
    if (namespaceMatch?.[1]) {
      imported.add(namespaceMatch[1]);
    }

    // Named imports: import { Foo, Bar as Baz } from 'module'
    const namedMatch = trimmed.match(/import\s+{([^}]+)}\s+from/);
    if (namedMatch?.[1]) {
      const parts = namedMatch[1].split(',');
      for (const part of parts) {
        const item = part.trim();
        if (!item) continue;
        const segments = item.split(/\s+as\s+/);
        const name = segments[1] ?? segments[0];
        if (name) {
          imported.add(name.trim());
        }
      }
    }
  }
  return imported;
}

/**
 * Insert import statement after existing imports in code.
 * Finds the position after the last import statement and inserts the new import.
 *
 * @example
 * const code = `
 * import React from 'react';
 *
 * export default function App() {}
 * `;
 * const result = insertAfterImports(code, "import { Aside } from '@astrojs/starlight/components';");
 * // Inserts after the React import
 */
export function insertAfterImports(code: string, importLine: string): string {
  if (!code || typeof code !== 'string') {
    return importLine;
  }
  const lines = code.split(/\r?\n/);
  let idx = 0;
  while (idx < lines.length) {
    const trimmed = lines[idx]?.trim() ?? '';
    if (!trimmed) {
      idx += 1;
      continue;
    }
    if (trimmed.startsWith('//') || trimmed.startsWith('/*')) {
      idx += 1;
      continue;
    }
    if (trimmed.startsWith('import ')) {
      idx += 1;
      continue;
    }
    break;
  }

  // MDX requires a blank line between imports and markdown content.
  // Only add blank line if the next line is markdown content, not JavaScript.
  const nextLine = lines[idx]?.trim() ?? '';
  const isJavaScript = nextLine.startsWith('export ') ||
    nextLine.startsWith('const ') ||
    nextLine.startsWith('let ') ||
    nextLine.startsWith('var ') ||
    nextLine.startsWith('function ') ||
    nextLine.startsWith('class ') ||
    nextLine.startsWith('import ') ||
    nextLine.startsWith('//') ||
    nextLine.startsWith('/*');

  if (nextLine && !isJavaScript) {
    // There's markdown content after the insertion point; add blank line after the import
    lines.splice(idx, 0, importLine, '');
  } else {
    lines.splice(idx, 0, importLine);
  }
  return lines.join('\n');
}
