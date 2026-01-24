/**
 * Shared constants for astro-markflow
 * @module constants
 */

/**
 * Virtual module prefix for Vite module resolution.
 * The null byte prefix ensures these are treated as virtual modules.
 */
export const VIRTUAL_MODULE_PREFIX = '\0markflow:';

/**
 * File extension for compiled markflow JSX output.
 */
export const OUTPUT_EXTENSION = '.markflow.jsx';

/**
 * esbuild configuration for JSX transformation.
 * Used to transform JSX syntax into function calls compatible with Astro's runtime.
 */
export const ESBUILD_JSX_CONFIG = {
  loader: 'jsx' as const,
  jsx: 'transform' as const,
  jsxFactory: '_jsx',
  jsxFragment: '_Fragment',
} as const;

/**
 * Shiki syntax highlighting theme configuration.
 * Uses CSS variables for theming compatibility with Astro.
 */
export const SHIKI_THEME = {
  /** Theme name used by shiki */
  name: 'astro-code',
  /** CSS variable prefix for syntax highlighting colors */
  variablePrefix: '--astro-code-',
  /** CSS class name added to highlighted code blocks */
  className: 'astro-code',
} as const;

/**
 * Default glob patterns to ignore when scanning for markdown files.
 */
export const DEFAULT_IGNORE_PATTERNS = ['node_modules/**', 'dist/**'] as const;
