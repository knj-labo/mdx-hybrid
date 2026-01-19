/**
 * Context-aware transform wrappers for pipeline composition
 * @module transforms
 */

import { rewriteExpressiveCodeBlocks, injectExpressiveCodeComponent } from './expressive-code.js';
import {
  injectAstroComponents,
  injectStarlightComponents,
  injectComponentImportsFromRegistry,
} from './inject-components.js';
import { rewriteAstroSetHtml } from './shiki.js';
import type { TransformContext } from '../types.js';
import { normalizeSteps } from './normalize-steps.js';

/**
 * Transform that rewrites <pre><code> blocks to ExpressiveCode components.
 * Only runs if expressiveCode is configured.
 */
export function transformExpressiveCode(ctx: TransformContext): TransformContext {
  if (!ctx.config.expressiveCode || !ctx.code) {
    return ctx;
  }
  const { code, changed } = rewriteExpressiveCodeBlocks(
    ctx.code,
    ctx.config.expressiveCode.component
  );
  if (changed) {
    return {
      ...ctx,
      code: injectExpressiveCodeComponent(code, ctx.config.expressiveCode),
    };
  }
  return { ...ctx, code };
}

/**
 * Transform that injects Astro component imports (Code, Prism).
 */
export function transformInjectAstroComponents(ctx: TransformContext): TransformContext {
  if (!ctx.code) {
    return ctx;
  }
  return {
    ...ctx,
    code: injectAstroComponents(ctx.code),
  };
}

/**
 * Transform that injects Starlight component imports.
 * Only runs if starlightComponents is configured.
 */
export function transformInjectStarlightComponents(ctx: TransformContext): TransformContext {
  if (!ctx.config.starlightComponents || !ctx.code) {
    return ctx;
  }
  return {
    ...ctx,
    code: injectStarlightComponents(ctx.code, ctx.config.starlightComponents),
  };
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
export { rewriteExpressiveCodeBlocks, injectExpressiveCodeComponent } from './expressive-code.js';
export {
  injectAstroComponents,
  injectStarlightComponents,
  injectComponentImports,
  injectComponentImportsFromRegistry,
} from './inject-components.js';
export { rewriteAstroSetHtml, highlightHtmlBlocks } from './shiki.js';
export { blocksToJsx } from './blocks-to-jsx.js';
export { normalizeSteps } from './normalize-steps.js';
