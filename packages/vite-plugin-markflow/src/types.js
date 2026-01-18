/**
 * Type definitions for Markflow plugin system
 * @module types
 */

/**
 * Transform context passed through the pipeline.
 * Contains the current code state and metadata needed by transforms.
 *
 * @typedef {Object} TransformContext
 * @property {string} code - Current JSX code being transformed
 * @property {string} source - Original markdown source
 * @property {string} filename - Source file path
 * @property {Object} frontmatter - Parsed frontmatter object
 * @property {Array} headings - Extracted headings from the document
 * @property {import('markflow/registry').ComponentRegistry} registry - Component registry for import resolution
 * @property {TransformConfig} config - Plugin configuration for transforms
 */

/**
 * Configuration available to transforms
 *
 * @typedef {Object} TransformConfig
 * @property {ExpressiveCodeConfig|null} expressiveCode - ExpressiveCode configuration or null if disabled
 * @property {boolean|Object} starlightComponents - Starlight components configuration
 * @property {Function|null} shiki - Shiki highlighter function or null if disabled
 */

/**
 * ExpressiveCode configuration
 *
 * @typedef {Object} ExpressiveCodeConfig
 * @property {string} component - Component name (e.g., "Code" or "ExpressiveCode")
 * @property {string} moduleId - Module to import from
 */

/**
 * A Markflow plugin that can hook into the transform pipeline.
 *
 * @typedef {Object} MarkflowPlugin
 * @property {string} name - Plugin identifier for debugging and ordering
 * @property {'pre'|'post'} [enforce] - Execution order: 'pre' runs before built-in transforms, 'post' runs after
 * @property {(ctx: TransformContext) => TransformContext|Promise<TransformContext>} [afterParse] - Hook called after markdown is parsed to JSX, before any transforms
 * @property {(ctx: TransformContext) => TransformContext|Promise<TransformContext>} [beforeInject] - Hook called before component injection transforms
 * @property {(ctx: TransformContext) => TransformContext|Promise<TransformContext>} [beforeOutput] - Hook called after all transforms, before esbuild
 * @property {(source: string, filename: string) => string} [preprocess] - Hook to preprocess raw markdown source before parsing
 */

/**
 * Collected hooks from plugins, organized by hook type
 *
 * @typedef {Object} PluginHooks
 * @property {Array<(ctx: TransformContext) => TransformContext|Promise<TransformContext>>} afterParse
 * @property {Array<(ctx: TransformContext) => TransformContext|Promise<TransformContext>>} beforeInject
 * @property {Array<(ctx: TransformContext) => TransformContext|Promise<TransformContext>>} beforeOutput
 * @property {Array<(source: string, filename: string) => string>} preprocess
 */

// Export empty object to make this a module
export {};
