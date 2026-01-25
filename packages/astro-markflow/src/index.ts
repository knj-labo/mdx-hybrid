/**
 * Astro integration for Markflow - high-performance MDX compiler.
 * @module astro-markflow
 */

import type { AstroIntegration } from 'astro';
import type { ComponentLibrary } from 'markflow/registry';
import { markflowPlugin } from './vite-plugin.js';
import { mergePresets, STARLIGHT_DEFAULT_ALLOW_IMPORTS, type PresetConfig } from './presets/index.js';
import type { MarkflowPlugin, MdxImportHandlingOptions } from './types.js';

/**
 * Options for the Markflow integration.
 */
export interface MarkflowOptions {
  /**
   * File filter function. Defaults to .md and .mdx files.
   */
  include?: (id: string) => boolean;

  /**
   * Component libraries to register.
   */
  libraries?: ComponentLibrary[];

  /**
   * Presets to apply. Presets are merged in order.
   */
  presets?: PresetConfig[];

  /**
   * Enable Starlight component injection.
   */
  starlightComponents?: boolean | {
    enabled: boolean;
    importSource?: string;
  };

  /**
   * Enable ExpressiveCode block rewriting.
   */
  expressiveCode?: boolean | {
    enabled: boolean;
    componentName?: string;
    importSource?: string;
  };

  /**
   * Compiler configuration.
   */
  compiler?: {
    jsx?: {
      code_sample_components?: string[];
    };
  };

  /**
   * Markflow plugins for transform hooks.
   */
  plugins?: MarkflowPlugin[];

  /**
   * MDX import handling configuration.
   * Controls which imports are allowed vs trigger fallback to @mdx-js/mdx.
   */
  mdx?: MdxImportHandlingOptions;
}

/**
 * Astro integration for Markflow.
 *
 * @example
 * ```js
 * // astro.config.mjs
 * import { defineConfig } from 'astro/config';
 * import markflow from 'astro-markflow';
 *
 * export default defineConfig({
 *   integrations: [markflow()],
 * });
 * ```
 *
 * @example
 * ```js
 * // With presets
 * import markflow from 'astro-markflow';
 * import { starlightPreset } from 'astro-markflow/presets';
 *
 * export default defineConfig({
 *   integrations: [
 *     markflow({
 *       presets: [starlightPreset()],
 *     })
 *   ],
 * });
 * ```
 */
export default function markflow(options: MarkflowOptions = {}): AstroIntegration {
  // Handle presets if provided
  let resolvedOptions = { ...options };

  if (Array.isArray(options.presets) && options.presets.length > 0) {
    const presetConfig = mergePresets(options.presets);

    // Apply preset config (user options override preset defaults)
    resolvedOptions = {
      libraries: options.libraries ?? presetConfig.libraries,
      starlightComponents: options.starlightComponents ?? presetConfig.starlightComponents,
      expressiveCode: options.expressiveCode ?? presetConfig.expressiveCode,
      mdx: options.mdx ?? presetConfig.mdx,
      ...options,
    };

    // Remove presets from final options (not needed by vite plugin)
    delete (resolvedOptions as Record<string, unknown>).presets;
  }

  // Auto-apply Starlight default allowImports when starlightComponents is enabled
  // This ensures imports like @astrojs/starlight/components don't trigger fallback
  const hasStarlightComponents = resolvedOptions.starlightComponents === true ||
    (typeof resolvedOptions.starlightComponents === 'object' && resolvedOptions.starlightComponents.enabled);
  const hasAllowImports = resolvedOptions.mdx?.allowImports && resolvedOptions.mdx.allowImports.length > 0;

  if (hasStarlightComponents && !hasAllowImports) {
    resolvedOptions.mdx = {
      ...resolvedOptions.mdx,
      allowImports: [...STARLIGHT_DEFAULT_ALLOW_IMPORTS],
      ignoreCodeFences: resolvedOptions.mdx?.ignoreCodeFences ?? true,
    };
  }

  return {
    name: 'astro-markflow',
    hooks: {
      'astro:config:setup': ({ updateConfig }) => {
        updateConfig({
          vite: {
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            plugins: [markflowPlugin(resolvedOptions) as any],
          },
        });
      },
    },
  };
}

// Re-export presets for convenience
export { starlightPreset, expressiveCodePreset, astroPreset, mergePresets } from './presets/index.js';
export type { PresetConfig } from './presets/index.js';
export type { MarkflowPlugin, TransformContext, PluginHooks, MdxImportHandlingOptions } from './types.js';
