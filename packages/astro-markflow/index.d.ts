import type { AstroIntegration } from 'astro';
import type { ComponentLibrary } from 'markflow/registry';

export interface PresetConfig {
  /**
   * Component libraries to register.
   */
  libraries?: ComponentLibrary[];

  /**
   * ExpressiveCode configuration.
   */
  expressiveCode?: boolean | {
    enabled: boolean;
    componentName?: string;
    importSource?: string;
  };

  /**
   * Starlight components configuration.
   */
  starlightComponents?: boolean | {
    enabled: boolean;
    importSource?: string;
  };
}

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
}

export interface MarkflowPlugin {
  /**
   * Plugin name for debugging.
   */
  name: string;

  /**
   * Execution order: 'pre' runs before built-in transforms, 'post' runs after.
   */
  enforce?: 'pre' | 'post';

  /**
   * Hook called after markdown is parsed to JSX.
   */
  afterParse?: (ctx: TransformContext) => TransformContext | Promise<TransformContext>;

  /**
   * Hook called before component injection.
   */
  beforeInject?: (ctx: TransformContext) => TransformContext | Promise<TransformContext>;

  /**
   * Hook called after all transforms, before esbuild.
   */
  beforeOutput?: (ctx: TransformContext) => TransformContext | Promise<TransformContext>;

  /**
   * Hook to preprocess raw markdown source.
   */
  preprocess?: (source: string, filename: string) => string;
}

export interface TransformContext {
  code: string;
  source: string;
  filename: string;
  frontmatter: Record<string, unknown>;
  headings: Array<{ depth: number; slug: string; text: string }>;
  registry: unknown;
  config: {
    expressiveCode: unknown;
    starlightComponents: unknown;
    shiki: unknown;
  };
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
export default function markflow(options?: MarkflowOptions): AstroIntegration;

/**
 * Creates a Starlight preset with Starlight components and optionally ExpressiveCode.
 */
export function starlightPreset(options?: { expressiveCode?: boolean }): PresetConfig;

/**
 * Creates an ExpressiveCode preset for syntax highlighting.
 */
export function expressiveCodePreset(options?: { componentName?: string; importSource?: string }): PresetConfig;

/**
 * Creates a base Astro preset with core components.
 */
export function astroPreset(): PresetConfig;

/**
 * Merges multiple presets into a single configuration.
 */
export function mergePresets(presets: PresetConfig[]): PresetConfig;
