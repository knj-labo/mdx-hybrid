/**
 * Context-aware transform wrappers for pipeline composition
 * @module transforms
 */

import {
  rewriteExpressiveCodeBlocks,
  rewriteSetHtmlCodeBlocks,
  injectExpressiveCodeComponent,
} from './expressive-code.js';
import {
  injectAstroComponents,
  injectStarlightComponents,
  injectComponentImportsFromRegistry,
} from './inject-components.js';
import { rewriteAstroSetHtml } from './shiki.js';
import type { TransformContext } from '../types.js';
import { normalizeSteps } from './normalize-steps.js';
import { normalizeFileTree } from './normalize-filetree.js';

/**
 * Transform that rewrites <pre><code> blocks to ExpressiveCode components.
 * Also handles code blocks inside set:html JSON strings (component slots).
 * Only runs if expressiveCode is configured.
 */
export function transformExpressiveCode(ctx: TransformContext): TransformContext {
  if (!ctx.config.expressiveCode || !ctx.code) {
    return ctx;
  }

  const componentName = ctx.config.expressiveCode.component;

  // First, rewrite loose <pre><code> blocks
  let { code, changed } = rewriteExpressiveCodeBlocks(ctx.code, componentName);

  // Then, rewrite code blocks inside set:html JSON strings
  const setHtmlResult = rewriteSetHtmlCodeBlocks(code, componentName);
  code = setHtmlResult.code;
  changed = changed || setHtmlResult.changed;

  if (changed) {
    return {
      ...ctx,
      code: injectExpressiveCodeComponent(code, ctx.config.expressiveCode),
    };
  }
  return { ...ctx, code };
}

/**
 * Transform that applies Shiki syntax highlighting.
 * Only runs if shiki highlighter is available.
 */
export async function transformShikiHighlight(
  ctx: TransformContext
): Promise<TransformContext> {
  if (!ctx.config.shiki || !ctx.code) {
    return ctx;
  }
  const code = await rewriteAstroSetHtml(ctx.code, ctx.config.shiki);
  return { ...ctx, code };
}

/**
 * Transform that injects component imports from the registry.
 * Unified replacement for transformInjectAstroComponents and transformInjectStarlightComponents.
 * Uses the registry from context to find all component modules and inject missing imports.
 */
export function transformInjectComponentsFromRegistry(ctx: TransformContext): TransformContext {
  if (!ctx.code || !ctx.registry) {
    return ctx;
  }
  return {
    ...ctx,
    code: injectComponentImportsFromRegistry(ctx.code, ctx.registry),
  };
}

// Re-export from sub-modules
export {
  rewriteExpressiveCodeBlocks,
  rewriteSetHtmlCodeBlocks,
  injectExpressiveCodeComponent,
} from './expressive-code.js';
export {
  injectAstroComponents,
  injectStarlightComponents,
  injectComponentImports,
  injectComponentImportsFromRegistry,
} from './inject-components.js';
export { rewriteAstroSetHtml, highlightHtmlBlocks } from './shiki.js';
export { blocksToJsx } from './blocks-to-jsx.js';
export { normalizeSteps } from './normalize-steps.js';
export { normalizeFileTree } from './normalize-filetree.js';
