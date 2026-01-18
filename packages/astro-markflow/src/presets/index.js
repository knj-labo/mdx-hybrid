/**
 * Astro Markflow presets for common UI libraries.
 * @module presets
 */

import { starlightLibrary, expressiveCodeLibrary, astroLibrary } from 'markflow/registry';

/**
 * @typedef {Object} PresetConfig
 * @property {Array} libraries - Component libraries to register
 * @property {boolean|Object} [expressiveCode] - ExpressiveCode configuration
 * @property {boolean|Object} [starlightComponents] - Starlight components configuration
 */

/**
 * Creates a Starlight preset with Starlight components and optionally ExpressiveCode.
 *
 * @param {Object} [options]
 * @param {boolean} [options.expressiveCode=true] - Enable ExpressiveCode integration
 * @returns {PresetConfig} Preset configuration
 *
 * @example
 * import markflow from 'astro-markflow';
 * import { starlightPreset } from 'astro-markflow/presets';
 *
 * export default defineConfig({
 *   integrations: [
 *     markflow({
 *       presets: [starlightPreset()],
 *     })
 *   ]
 * });
 */
export function starlightPreset(options = {}) {
  const { expressiveCode = true } = options;

  const libraries = [astroLibrary, starlightLibrary];
  if (expressiveCode) {
    libraries.push(expressiveCodeLibrary);
  }

  return {
    libraries,
    starlightComponents: true,
    expressiveCode: expressiveCode ? { enabled: true } : false,
  };
}

/**
 * Creates an ExpressiveCode preset for syntax highlighting.
 *
 * @param {Object} [options]
 * @param {string} [options.componentName='Code'] - Component name to use
 * @param {string} [options.importSource='astro-expressive-code/components'] - Import source
 * @returns {PresetConfig} Preset configuration
 *
 * @example
 * import markflow from 'astro-markflow';
 * import { expressiveCodePreset } from 'astro-markflow/presets';
 *
 * export default defineConfig({
 *   integrations: [
 *     markflow({
 *       presets: [expressiveCodePreset()],
 *     })
 *   ]
 * });
 */
export function expressiveCodePreset(options = {}) {
  const { componentName = 'Code', importSource = 'astro-expressive-code/components' } = options;

  return {
    libraries: [astroLibrary, expressiveCodeLibrary],
    expressiveCode: {
      enabled: true,
      componentName,
      importSource,
    },
  };
}

/**
 * Creates a base Astro preset with core components.
 *
 * @returns {PresetConfig} Preset configuration
 *
 * @example
 * import markflow from 'astro-markflow';
 * import { astroPreset } from 'astro-markflow/presets';
 *
 * export default defineConfig({
 *   integrations: [
 *     markflow({
 *       presets: [astroPreset()],
 *     })
 *   ]
 * });
 */
export function astroPreset() {
  return {
    libraries: [astroLibrary],
  };
}

/**
 * Merges multiple presets into a single configuration.
 * Later presets override earlier ones for conflicting options.
 *
 * @param {PresetConfig[]} presets - Array of presets to merge
 * @returns {PresetConfig} Merged configuration
 */
export function mergePresets(presets) {
  if (!presets || presets.length === 0) {
    return { libraries: [astroLibrary] };
  }

  const merged = {
    libraries: [],
    expressiveCode: false,
    starlightComponents: false,
  };

  const libraryIds = new Set();

  for (const preset of presets) {
    // Merge libraries (deduplicate by id)
    if (Array.isArray(preset.libraries)) {
      for (const lib of preset.libraries) {
        if (!libraryIds.has(lib.id)) {
          libraryIds.add(lib.id);
          merged.libraries.push(lib);
        }
      }
    }

    // Merge expressiveCode (last wins)
    if (preset.expressiveCode !== undefined) {
      merged.expressiveCode = preset.expressiveCode;
    }

    // Merge starlightComponents (last wins)
    if (preset.starlightComponents !== undefined) {
      merged.starlightComponents = preset.starlightComponents;
    }
  }

  // Ensure at least astroLibrary is present
  if (merged.libraries.length === 0) {
    merged.libraries.push(astroLibrary);
  }

  return merged;
}
