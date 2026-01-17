/**
 * Context-aware transform wrappers for pipeline composition
 * @module transforms
 */

import { rewriteExpressiveCodeBlocks, injectExpressiveCodeComponent } from './expressive-code.js';
import { injectAstroComponents, injectStarlightComponents } from './inject-components.js';
import { rewriteAstroSetHtml } from './shiki.js';

/**
 * Transform that rewrites <pre><code> blocks to ExpressiveCode components.
 * Only runs if expressiveCode is configured.
 *
 * @param {import('../types.js').TransformContext} ctx - Transform context
 * @returns {import('../types.js').TransformContext} Updated context
 */
export function transformExpressiveCode(ctx) {
  if (!ctx.config.expressiveCode) {
    return ctx;
  }
  const { code, changed } = rewriteExpressiveCodeBlocks(
    ctx.code,
    ctx.config.expressiveCode.component
  );
  if (changed) {
    return {
      ...ctx,
      code: injectExpressiveCodeComponent(code, ctx.config.expressiveCode)
    };
  }
  return { ...ctx, code };
}

/**
 * Transform that injects Astro component imports (Code, Prism).
 *
 * @param {import('../types.js').TransformContext} ctx - Transform context
 * @returns {import('../types.js').TransformContext} Updated context
 */
export function transformInjectAstroComponents(ctx) {
  return {
    ...ctx,
    code: injectAstroComponents(ctx.code)
  };
}

/**
 * Transform that injects Starlight component imports.
 * Only runs if starlightComponents is configured.
 *
 * @param {import('../types.js').TransformContext} ctx - Transform context
 * @returns {import('../types.js').TransformContext} Updated context
 */
export function transformInjectStarlightComponents(ctx) {
  if (!ctx.config.starlightComponents) {
    return ctx;
  }
  return {
    ...ctx,
    code: injectStarlightComponents(ctx.code, ctx.config.starlightComponents)
  };
}

/**
 * Transform that applies Shiki syntax highlighting.
 * Only runs if shiki highlighter is available.
 *
 * @param {import('../types.js').TransformContext} ctx - Transform context
 * @returns {Promise<import('../types.js').TransformContext>} Updated context
 */
export async function transformShikiHighlight(ctx) {
  if (!ctx.config.shiki) {
    return ctx;
  }
  const code = await rewriteAstroSetHtml(ctx.code, ctx.config.shiki);
  return { ...ctx, code };
}
