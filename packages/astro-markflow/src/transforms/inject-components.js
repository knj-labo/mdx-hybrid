/**
 * Component import injection transformations
 * @module transforms/inject-components
 */

import { collectImportedNames, insertAfterImports } from '../utils/imports.js';
import { resolveStarlightConfig } from '../utils/config.js';

/**
 * Default Astro component names
 * @deprecated Use registry.getComponentsByModule('astro/components') instead
 */
export const ASTRO_COMPONENTS = ["Code", "Prism"];

/**
 * Default Astro components module path
 * @deprecated Use registry from markflow/registry instead
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
 * @param {import('markflow/registry').ComponentRegistry} [registry] - Optional component registry
 * @returns {string} - Code with Starlight imports
 *
 * @example
 * const code = `<Aside>Note</Aside>`;
 * const result = injectStarlightComponents(code, true);
 * // Adds: import { Aside } from '@astrojs/starlight/components';
 *
 * // With registry (preferred):
 * const result = injectStarlightComponents(code, true, registry);
 * // Gets defaults from registry.getComponentsByModule('@astrojs/starlight/components')
 */
export function injectStarlightComponents(code, config, registry) {
  const resolved = resolveStarlightConfig(config, registry);
  if (!resolved) return code;

  return injectComponentImports(code, resolved.components, resolved.moduleId);
}

/**
 * Inject Astro component imports based on usage.
 * Checks for Code/Prism component usage and adds imports.
 *
 * @param {string} code - JSX code to process
 * @param {import('markflow/registry').ComponentRegistry} [registry] - Optional component registry
 * @returns {string} - Code with Astro imports
 *
 * @example
 * const code = `<Code lang="js">const x = 1;</Code>`;
 * const result = injectAstroComponents(code);
 * // Adds: import { Code } from 'astro/components';
 *
 * // With registry (preferred):
 * const result = injectAstroComponents(code, registry);
 * // Gets component list from registry.getComponentsByModule('astro/components')
 */
export function injectAstroComponents(code, registry) {
  // Get components from registry if available, otherwise use deprecated constants
  let components = ASTRO_COMPONENTS;
  let moduleId = ASTRO_COMPONENTS_MODULE;

  if (registry) {
    const astroComponents = registry.getComponentsByModule(ASTRO_COMPONENTS_MODULE);
    if (astroComponents.length > 0) {
      components = astroComponents.map((c) => c.name);
      moduleId = astroComponents[0].modulePath;
    }
  }

  return injectComponentImports(code, components, moduleId);
}

/**
 * Inject component imports from registry based on usage.
 * Scans code for component usage and injects missing imports
 * using information from the registry.
 *
 * @param {string} code - JSX code to process
 * @param {import('markflow/registry').ComponentRegistry} registry - Component registry
 * @returns {string} - Code with injected imports
 *
 * @example
 * const code = `<Aside>Note</Aside><Code lang="js">x</Code>`;
 * const result = injectComponentImportsFromRegistry(code, registry);
 * // Adds imports for both Aside and Code from their respective modules
 */
export function injectComponentImportsFromRegistry(code, registry) {
  if (!code || typeof code !== 'string' || !registry) {
    return code;
  }

  const scanTarget = stripHeadingsMeta(code);
  const imported = collectImportedNames(code);
  const allComponents = registry.getAllComponents();

  // Find used components that are missing imports
  const missingByModule = new Map();

  for (const comp of allComponents) {
    if (new RegExp(`<${comp.name}\\b`).test(scanTarget) && !imported.has(comp.name)) {
      const modulePath = comp.modulePath;
      if (!missingByModule.has(modulePath)) {
        missingByModule.set(modulePath, []);
      }
      missingByModule.get(modulePath).push(comp);
    }
  }

  if (missingByModule.size === 0) {
    return code;
  }

  // Generate import statements grouped by module
  let result = code;
  for (const [modulePath, components] of missingByModule) {
    // Check if all components use named exports
    const allNamed = components.every((c) => c.exportType === 'named');
    if (allNamed) {
      const names = components.map((c) => c.name).join(', ');
      const importLine = `import { ${names} } from '${modulePath}';`;
      result = insertAfterImports(result, importLine);
    } else {
      // Individual default imports for each component
      for (const comp of components) {
        const importLine = `import ${comp.name} from '${modulePath}/${comp.name}.astro';`;
        result = insertAfterImports(result, importLine);
      }
    }
  }

  return result;
}
