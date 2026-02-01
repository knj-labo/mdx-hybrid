/**
 * Pipeline orchestrator for composing Markflow transforms
 * @module pipeline/orchestrator
 */

import { pipe } from './pipe.js';
import type { TransformContext, TransformConfig, Transform, PipelineOptions } from './types.js';
import {
  transformExpressiveCode,
  transformInjectComponentsFromRegistry,
  transformShikiHighlight,
} from '../transforms/index.js';
import { normalizeSteps } from '../transforms/normalize-steps.js';

/**
 * Default context values for transforms.
 */
const DEFAULT_CONTEXT: TransformContext = {
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

const PIPELINE_PROFILE =
  typeof process !== 'undefined' && process.env?.MARKFLOW_PIPELINE_PROFILE === '1';
const PIPELINE_PROFILE_VERBOSE =
  typeof process !== 'undefined' && process.env?.MARKFLOW_PIPELINE_PROFILE_VERBOSE === '1';

type PipelineProfileStats = {
  totalMs: number;
  count: number;
  maxMs: number;
};

const pipelineProfileStats = new Map<string, PipelineProfileStats>();
let pipelineProfileHookRegistered = false;
const now = (): number =>
  typeof performance !== 'undefined' && typeof performance.now === 'function'
    ? performance.now()
    : Date.now();

function registerPipelineProfileExitHook(): void {
  if (pipelineProfileHookRegistered || !PIPELINE_PROFILE) return;
  pipelineProfileHookRegistered = true;

  if (typeof process === 'undefined' || typeof process.on !== 'function') {
    return;
  }

  process.on('exit', () => {
    if (!pipelineProfileStats.size) return;
    const entries = [...pipelineProfileStats.entries()]
      .sort((a, b) => b[1].totalMs - a[1].totalMs);
    const totalMs = entries.reduce((sum, [, stats]) => sum + stats.totalMs, 0);
    console.log('[markflow:pipeline] summary (total ms)');
    for (const [label, stats] of entries) {
      const avgMs = stats.totalMs / stats.count;
      const pct = totalMs > 0 ? (stats.totalMs / totalMs) * 100 : 0;
      console.log(
        `[markflow:pipeline] ${label} total=${stats.totalMs.toFixed(2)}ms ` +
        `avg=${avgMs.toFixed(2)}ms max=${stats.maxMs.toFixed(2)}ms ` +
        `n=${stats.count} ${pct.toFixed(1)}%`
      );
    }
  });
}

function recordPipelineProfile(label: string, filename: string, durationMs: number): void {
  const stats = pipelineProfileStats.get(label) ?? {
    totalMs: 0,
    count: 0,
    maxMs: 0,
  };
  stats.totalMs += durationMs;
  stats.count += 1;
  stats.maxMs = Math.max(stats.maxMs, durationMs);
  pipelineProfileStats.set(label, stats);

  if (PIPELINE_PROFILE_VERBOSE) {
    const basename = filename.split(/[\\/]/).pop() ?? filename;
    const suffix = basename ? ` (${basename})` : '';
    console.log(`[markflow:pipeline] ${label} ${durationMs.toFixed(2)}ms${suffix}`);
  }
}

function withPipelineTiming(label: string, transform: Transform): Transform {
  if (!PIPELINE_PROFILE) return transform;
  registerPipelineProfileExitHook();
  return async (ctx: TransformContext): Promise<TransformContext> => {
    const start = now();
    try {
      return await transform(ctx);
    } finally {
      const durationMs = now() - start;
      recordPipelineProfile(label, ctx.filename, durationMs);
    }
  };
}

function labelHook(group: string, index: number, transform: Transform): Transform {
  const name = transform.name ? `:${transform.name}` : '';
  return withPipelineTiming(`${group}[${index}]${name}`, transform);
}

function labelBuiltIn(name: string, transform: Transform): Transform {
  return withPipelineTiming(`builtin:${name}`, transform);
}

function labelCustom(index: number, transform: Transform): Transform {
  const name = transform.name ? `:${transform.name}` : '';
  return withPipelineTiming(`custom[${index}]${name}`, transform);
}

const transformNormalizeSteps: Transform = (ctx) => {
  if (!ctx.code) return ctx;
  const { code, changed } = normalizeSteps(ctx.code);
  if (!changed) return ctx;
  return { ...ctx, code };
};

/**
 * Creates a TransformContext with default values.
 * Useful for standalone pipeline usage outside of Vite.
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
export function createContext(
  overrides: Partial<TransformContext> = {}
): TransformContext {
  return {
    ...DEFAULT_CONTEXT,
    ...overrides,
    config: {
      ...DEFAULT_CONTEXT.config,
      ...(overrides.config || {}),
    } as TransformConfig,
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
export function createPipeline(options: PipelineOptions = {}): Transform {
  const { afterParse = [], beforeInject = [], beforeOutput = [] } = options;

  return pipe<TransformContext>(
    // User hooks: afterParse
    ...afterParse.map((hook, index) => labelHook('afterParse', index, hook)),

    // Built-in: ExpressiveCode rewriting
    labelBuiltIn('expressiveCode', transformExpressiveCode),

    // User hooks: beforeInject
    ...beforeInject.map((hook, index) => labelHook('beforeInject', index, hook)),

    // Built-in: Component injection (unified, registry-driven)
    labelBuiltIn('injectComponents', transformInjectComponentsFromRegistry),

    // Built-in: Shiki highlighting
    labelBuiltIn('shikiHighlight', transformShikiHighlight),

    // Built-in: Normalize Steps to single <ol>
    labelBuiltIn('normalizeSteps', transformNormalizeSteps),

    // User hooks: beforeOutput
    ...beforeOutput.map((hook, index) => labelHook('beforeOutput', index, hook))
  );
}

/**
 * Creates a custom pipeline with only the specified transforms.
 * Unlike createPipeline, this doesn't include any built-in transforms.
 * Useful for testing or when you want full control over the pipeline.
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
export function createCustomPipeline(...transforms: Transform[]): Transform {
  return pipe<TransformContext>(
    ...transforms.map((transform, index) => labelCustom(index, transform))
  );
}
