/**
 * Pipeline orchestrator for composing Markflow transforms
 * @module pipeline/orchestrator
 */

import { pipe } from './pipe.js';
import {
  transformExpressiveCode,
  transformInjectComponentsFromRegistry,
  transformShikiHighlight,
} from '../transforms/index.js';

/**
 * Default context values for transforms.
 * @type {Partial<import('./types.js').TransformContext>}
 */
const DEFAULT_CONTEXT = {
  code: '',
  source: '',
  filename: '',
  frontmatter: {},
  headings: [],
  registry: undefined,
  config: {
    expressiveCode: null,
    starlightComponents: false,
    shiki: null,
  },
};

/**
 * Creates a TransformContext with default values.
 * Useful for standalone pipeline usage outside of Vite.
 *
 * @param {Partial<import('./types.js').TransformContext>} [overrides] - Values to override defaults
 * @returns {import('./types.js').TransformContext} Complete transform context
 *
 * @example
 * const ctx = createContext({
 *   code: compiledJsx,
 *   source: markdownSource,
 *   filename: '/path/to/file.md',
 *   frontmatter: { title: 'Hello' },
 *   headings: [{ depth: 1, text: 'Hello' }],
 * });
 */
export function createContext(overrides = {}) {
  return {
    ...DEFAULT_CONTEXT,
    ...overrides,
    config: {
      ...DEFAULT_CONTEXT.config,
      ...(overrides.config || {}),
    },
  };
}

/**
 * Creates the standard Markflow transform pipeline with hook support.
 * This is the same pipeline used by the Vite plugin internally.
 *
 * Hook execution order:
 * 1. afterParse hooks (user transforms after parsing)
 * 2. ExpressiveCode rewriting (built-in)
 * 3. beforeInject hooks (user transforms before injection)
 * 4. Component injection from registry (built-in)
 * 5. Shiki highlighting (built-in)
 * 6. beforeOutput hooks (user transforms before output)
 *
 * @param {import('./types.js').PipelineOptions} [options] - Hook arrays to include in pipeline
 * @returns {import('./types.js').Transform} Composed transform function
 *
 * @example
 * // Create pipeline with custom hooks
 * const pipeline = createPipeline({
 *   afterParse: [myCustomTransform],
 *   beforeOutput: [addMetadataComments],
 * });
 *
 * // Use the pipeline
 * const ctx = createContext({ code, source, filename });
 * const result = await pipeline(ctx);
 */
export function createPipeline(options = {}) {
  const { afterParse = [], beforeInject = [], beforeOutput = [] } = options;

  return pipe(
    // User hooks: afterParse
    ...afterParse,

    // Built-in: ExpressiveCode rewriting
    transformExpressiveCode,

    // User hooks: beforeInject
    ...beforeInject,

    // Built-in: Component injection (unified, registry-driven)
    transformInjectComponentsFromRegistry,

    // Built-in: Shiki highlighting
    transformShikiHighlight,

    // User hooks: beforeOutput
    ...beforeOutput,
  );
}

/**
 * Creates a custom pipeline with only the specified transforms.
 * Unlike createPipeline, this doesn't include any built-in transforms.
 * Useful for testing or when you want full control over the pipeline.
 *
 * @param {...import('./types.js').Transform} transforms - Transforms to compose
 * @returns {import('./types.js').Transform} Composed transform function
 *
 * @example
 * // Create a minimal pipeline for testing
 * const testPipeline = createCustomPipeline(
 *   transformExpressiveCode,
 *   myCustomTransform,
 * );
 *
 * // Use it standalone
 * const result = await testPipeline(ctx);
 */
export function createCustomPipeline(...transforms) {
  return pipe(...transforms);
}
