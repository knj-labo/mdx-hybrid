/**
 * Component import injection transformations
 * @module transforms/inject-components
 */

import { collectImportedNames, insertAfterImports } from '../utils/imports.js';
import { resolveStarlightConfig } from '../utils/config.js';

/**
 * Default Astro component names
 */
export const ASTRO_COMPONENTS = ["Code", "Prism"];

/**
 * Default Astro components module path
 */
export const ASTRO_COMPONENTS_MODULE = "astro/components";

/**
 * Strip heading metadata exports from code.
 * Removes `export const headings` and `export function getHeadings` statements.
 *
 * @param {string} code - Code to process
 * @returns {string} - Code without heading exports
 */
function stripHeadingsMeta(code) {
  return code
    .replace(/export const headings\s*=\s*\[[\s\S]*?\];\r?\n/g, "")
    .replace(/export function getHeadings\(\)\s*\{[\s\S]*?\}\r?\n/g, "");
}

/**
 * Generic component import injection.
 * Scans code for component usage and injects missing imports.
 *
 * @param {string} code - JSX code to process
 * @param {string[]} components - Component names to check for
 * @param {string} moduleId - Module to import from
 * @returns {string} - Code with injected imports
 *
 * @example
 * const code = `
 * export default function Content() {
 *   return <Aside>Hello</Aside>;
 * }
 * `;
 * const result = injectComponentImports(code, ['Aside'], '@astrojs/starlight/components');
 * // Adds: import { Aside } from '@astrojs/starlight/components';
 */
export function injectComponentImports(code, components, moduleId) {
  if (!code || typeof code !== 'string') {
    return code;
  }
  const scanTarget = stripHeadingsMeta(code);
  const used = components.filter((name) =>
    new RegExp(`<${name}\\b`).test(scanTarget),
  );
  if (used.length === 0) return code;

  const imported = collectImportedNames(code);
  const missing = used.filter((name) => !imported.has(name));
  if (missing.length === 0) return code;

  const importLine = `import { ${missing.join(", ")} } from '${moduleId}';`;
  return insertAfterImports(code, importLine);
}

/**
 * Inject Starlight component imports based on usage.
 * Normalizes config and delegates to injectComponentImports.
 *
 * @param {string} code - JSX code to process
 * @param {boolean|object} config - Starlight configuration
 * @returns {string} - Code with Starlight imports
 *
 * @example
 * const code = `<Aside>Note</Aside>`;
 * const result = injectStarlightComponents(code, true);
 * // Adds: import { Aside } from '@astrojs/starlight/components';
 */
export function injectStarlightComponents(code, config) {
  const resolved = resolveStarlightConfig(config);
  if (!resolved) return code;

  return injectComponentImports(code, resolved.components, resolved.moduleId);
}

/**
 * Inject Astro component imports based on usage.
 * Checks for Code/Prism component usage and adds imports.
 *
 * @param {string} code - JSX code to process
 * @returns {string} - Code with Astro imports
 *
 * @example
 * const code = `<Code lang="js">const x = 1;</Code>`;
 * const result = injectAstroComponents(code);
 * // Adds: import { Code } from 'astro/components';
 */
export function injectAstroComponents(code) {
  return injectComponentImports(code, ASTRO_COMPONENTS, ASTRO_COMPONENTS_MODULE);
}
