/**
 * Astro integration for Markflow - high-performance MDX compiler.
 * @module astro-markflow
 */

import { markflowPlugin } from './vite-plugin.js';
import { mergePresets } from './presets/index.js';

/**
 * Astro integration for Markflow.
 * @param {import('../index').MarkflowOptions} [options]
 * @returns {import('astro').AstroIntegration}
 */
export default function markflow(options = {}) {
  // Handle presets if provided
  let resolvedOptions = { ...options };

  if (Array.isArray(options.presets) && options.presets.length > 0) {
    const presetConfig = mergePresets(options.presets);

    // Apply preset config (user options override preset defaults)
    resolvedOptions = {
      libraries: options.libraries ?? presetConfig.libraries,
      starlightComponents: options.starlightComponents ?? presetConfig.starlightComponents,
      expressiveCode: options.expressiveCode ?? presetConfig.expressiveCode,
      ...options,
    };

    // Remove presets from final options (not needed by vite plugin)
    delete resolvedOptions.presets;
  }

  return {
    name: 'astro-markflow',
    hooks: {
      'astro:config:setup': ({ updateConfig }) => {
        updateConfig({
          vite: {
            plugins: [markflowPlugin(resolvedOptions)],
          },
        });
      },
    },
  };
}

// Re-export presets for convenience
export { starlightPreset, expressiveCodePreset, astroPreset, mergePresets } from './presets/index.js';
