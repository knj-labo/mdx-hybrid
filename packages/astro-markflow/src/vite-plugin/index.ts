/**
 * Vite plugin modules for Markflow
 * @module vite-plugin
 */

// Re-export types
export type {
  MarkflowBinding,
  MarkflowCompiler,
  CompileResult,
  BatchCompileResult,
  ParseBlocksResult,
  MarkflowPluginOptions,
  DocumentFragment,
  Node,
  Element,
  TextNode,
} from './types.js';

export { DEFAULT_EXTENSIONS } from './types.js';

// Re-export binding loader
export { loadMarkflowBinding, resetBindingPromise, ENABLE_SHIKI, IS_MDAST } from './binding-loader.js';

// Re-export JSX module utilities
export { wrapHtmlInJsxModule, compileFallbackModule } from './jsx-module.js';

// Re-export directive rewriter
export { rewriteFallbackDirectives, injectFallbackImports } from './directive-rewriter.js';
