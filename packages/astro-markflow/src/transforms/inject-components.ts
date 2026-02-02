/**
 * Component import injection transformations
 * @module transforms/inject-components
 */

import type { Registry } from 'markflow/registry';
import { astroLibrary } from 'markflow/registry';
import { collectImportedNames, insertAfterImports } from '../utils/imports.js';
import { resolveStarlightConfig, type StarlightUserConfig } from '../utils/config.js';


/** Combined regex to strip set:html content and headings metadata in a single pass */
const RE_SCAN_NOISE = /set:html=\{"(?:[^"\\]|\\.)*"\}|export const headings[\s\S]*?];\r?\n?|export function getHeadings\(\) \{(?:[^\r\n]+\r?\n|[\s\S]*?\r?\n\}\r?\n?)/g;

/** Strip set:html content and headings metadata in a single pass to avoid false component matches */
function stripScanNoise(code: string): string {
  return code.replace(RE_SCAN_NOISE, (match) =>
    match.startsWith('set:html=') ? 'set:html={""}' : ''
  );
}

/** Cache compiled regex per registry instance (registry is immutable after init) */
const registryRegexCache = new WeakMap<Registry, RegExp>();

function getComponentPattern(registry: Registry): RegExp | null {
  let cached = registryRegexCache.get(registry);
  if (cached) return cached;
  const all = registry.getAllComponents();
  if (all.length === 0) return null;
  cached = new RegExp(`<(${all.map(c => c.name).join('|')})\\b`, 'g');
  registryRegexCache.set(registry, cached);
  return cached;
}

/**
 * Generic component import injection.
 * Scans code for component usage and injects missing imports.
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
export function injectComponentImports(
  code: string,
  components: string[],
  moduleId: string
): string {
  if (!code || typeof code !== 'string' || components.length === 0) {
    return code;
  }
  const scanTarget = stripScanNoise(code);

  // PERF: Use single combined regex instead of per-component regex
  // This reduces from O(n) regex compilations to O(1)
  const combinedPattern = new RegExp(`<(${components.join('|')})\\b`, 'g');
  const matches = scanTarget.match(combinedPattern);
  if (!matches) return code;

  // Extract unique component names from matches
  const usedSet = new Set<string>();
  for (const match of matches) {
    const name = match.slice(1); // Remove leading '<'
    usedSet.add(name);
  }
  const used = components.filter((name) => usedSet.has(name));
  if (used.length === 0) return code;

  const imported = collectImportedNames(code, { skipCodeFences: true });
  const missing = used.filter((name) => !imported.has(name));
  if (missing.length === 0) return code;

  const importLine = `import { ${missing.join(', ')} } from '${moduleId}';`;
  return insertAfterImports(code, importLine);
}

/**
 * Inject Starlight component imports based on usage.
 * Normalizes config and delegates to injectComponentImports.
 *
 * @example
 * const code = `<Aside>Note</Aside>`;
 * const result = injectStarlightComponents(code, true);
 * // Adds: import { Aside } from '@astrojs/starlight/components';
 */
export function injectStarlightComponents(
  code: string,
  config: boolean | StarlightUserConfig,
  registry?: Registry
): string {
  const resolved = resolveStarlightConfig(config, registry);
  if (!resolved) return code;

  return injectComponentImports(code, resolved.components, resolved.moduleId);
}

/**
 * Inject Astro component imports based on usage.
 * Checks for Code/Prism component usage and adds imports.
 *
 * @example
 * const code = `<Code lang="js">const x = 1;</Code>`;
 * const result = injectAstroComponents(code);
 * // Adds: import { Code } from 'astro/components';
 */
export function injectAstroComponents(code: string, registry?: Registry): string {
  // Get components from registry if available, otherwise use library preset
  let components = astroLibrary.components.map((c) => c.name);
  let moduleId = astroLibrary.defaultModulePath;

  if (registry) {
    const astroComponents = registry.getComponentsByModule(astroLibrary.defaultModulePath);
    if (astroComponents.length > 0 && astroComponents[0]) {
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
 * @example
 * const code = `<Aside>Note</Aside><Code lang="js">x</Code>`;
 * const result = injectComponentImportsFromRegistry(code, registry);
 * // Adds imports for both Aside and Code from their respective modules
 */
export function injectComponentImportsFromRegistry(
  code: string,
  registry: Registry
): string {
  if (!code || typeof code !== 'string' || !registry) {
    return code;
  }

  const combinedPattern = getComponentPattern(registry);
  if (!combinedPattern) return code;

  const scanTarget = stripScanNoise(code);
  const imported = collectImportedNames(code, { skipCodeFences: true });

  combinedPattern.lastIndex = 0;
  const matches = scanTarget.match(combinedPattern);
  if (!matches) return code;

  // Extract unique component names from matches
  const usedNames = new Set<string>();
  for (const match of matches) {
    const name = match.slice(1); // Remove leading '<'
    usedNames.add(name);
  }

  // Find used components that are missing imports
  const missingByModule = new Map<string, Array<{ name: string; exportType: string }>>();

  const allComponents = registry.getAllComponents();
  for (const comp of allComponents) {
    if (usedNames.has(comp.name) && !imported.has(comp.name)) {
      const modulePath = comp.modulePath;
      if (!missingByModule.has(modulePath)) {
        missingByModule.set(modulePath, []);
      }
      missingByModule.get(modulePath)!.push({ name: comp.name, exportType: comp.exportType });
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
