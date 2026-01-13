import type { AstroIntegration } from 'astro';

export interface MarkflowOptions {
  /**
   * File filter function. Defaults to .md and .mdx files.
   */
  include?: (id: string) => boolean;

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
 */
export default function markflow(options?: MarkflowOptions): AstroIntegration;
