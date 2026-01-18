/**
 * Type definitions for Markflow transform pipeline
 * @module pipeline/types
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
 * @property {import('markflow/registry').ComponentRegistry} [registry] - Component registry for import resolution
 * @property {TransformConfig} config - Plugin configuration for transforms
 */

/**
 * Configuration available to transforms
 *
 * @typedef {Object} TransformConfig
 * @property {ExpressiveCodeConfig|null} [expressiveCode] - ExpressiveCode configuration or null if disabled
 * @property {boolean|Object} [starlightComponents] - Starlight components configuration
 * @property {Function|null} [shiki] - Shiki highlighter function or null if disabled
 */

/**
 * ExpressiveCode configuration
 *
 * @typedef {Object} ExpressiveCodeConfig
 * @property {string} component - Component name (e.g., "Code" or "ExpressiveCode")
 * @property {string} moduleId - Module to import from
 */

/**
 * A transform function that takes a context and returns a modified context.
 * Can be synchronous or asynchronous.
 *
 * @typedef {(ctx: TransformContext) => TransformContext | Promise<TransformContext>} Transform
 */

/**
 * Options for creating a standard Markflow pipeline with hooks.
 *
 * @typedef {Object} PipelineOptions
 * @property {Transform[]} [afterParse] - Hooks to run after parsing, before built-in transforms
 * @property {Transform[]} [beforeInject] - Hooks to run before component injection
 * @property {Transform[]} [beforeOutput] - Hooks to run after all transforms, before output
 */

// Export empty object to make this a module
export {};
