/**
 * Import statement manipulation utilities
 * @module utils/imports
 */

/**
 * Collect all imported names from JavaScript/JSX code.
 * Handles default imports, namespace imports, and named imports.
 *
 * @param {string} code - JavaScript/JSX code to analyze
 * @returns {Set<string>} - Set of imported identifiers
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
export function collectImportedNames(code) {
  const imported = new Set();
  if (!code || typeof code !== 'string') {
    return imported;
  }
  const lines = code.split(/\r?\n/);
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed.startsWith("import ") || trimmed.startsWith("import(")) {
      continue;
    }

    // Default import: import Foo from 'module'
    const defaultMatch = trimmed.match(
      /^import\s+([A-Za-z$_][\w$]*)\s*(?:,|\s+from\s)/,
    );
    if (defaultMatch) {
      imported.add(defaultMatch[1]);
    }

    // Namespace import: import * as Foo from 'module'
    const namespaceMatch = trimmed.match(
      /^import\s+\*\s+as\s+([A-Za-z$_][\w$]*)\s+from/,
    );
    if (namespaceMatch) {
      imported.add(namespaceMatch[1]);
    }

    // Named imports: import { Foo, Bar as Baz } from 'module'
    const namedMatch = trimmed.match(/import\s+{([^}]+)}\s+from/);
    if (namedMatch) {
      const parts = namedMatch[1].split(",");
      for (const part of parts) {
        const item = part.trim();
        if (!item) continue;
        const [name, alias] = item.split(/\s+as\s+/);
        imported.add((alias || name).trim());
      }
    }
  }
  return imported;
}

/**
 * Insert import statement after existing imports in code.
 * Finds the position after the last import statement and inserts the new import.
 *
 * @param {string} code - Code to modify
 * @param {string} importLine - Import statement to add (should include semicolon)
 * @returns {string} - Modified code with new import
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
export function insertAfterImports(code, importLine) {
  if (!code || typeof code !== 'string') {
    return importLine;
  }
  const lines = code.split(/\r?\n/);
  let idx = 0;
  while (idx < lines.length) {
    const trimmed = lines[idx].trim();
    if (!trimmed) {
      idx += 1;
      continue;
    }
    if (trimmed.startsWith("//") || trimmed.startsWith("/*")) {
      idx += 1;
      continue;
    }
    if (trimmed.startsWith("import ")) {
      idx += 1;
      continue;
    }
    break;
  }
  lines.splice(idx, 0, importLine);
  return lines.join("\n");
}
