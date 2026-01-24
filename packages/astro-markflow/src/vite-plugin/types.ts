/**
 * Type definitions for the Markflow Vite plugin
 * @module vite-plugin/types
 */

import type { DefaultTreeAdapterMap } from 'parse5';
import type { ComponentLibrary } from 'markflow/registry';
import type { MarkflowPlugin, MdxImportHandlingOptions } from '../types.js';

// Parse5 DOM types
export type DocumentFragment = DefaultTreeAdapterMap['documentFragment'];
export type Node = DefaultTreeAdapterMap['node'];
export type Element = DefaultTreeAdapterMap['element'];
export type TextNode = DefaultTreeAdapterMap['textNode'];

/**
 * Native NAPI binding interface for Markflow compiler.
 */
export interface MarkflowBinding {
  createCompiler?: (config: Record<string, unknown>) => MarkflowCompiler;
  MarkflowCompiler?: new (config: Record<string, unknown>) => MarkflowCompiler;
  compileBatch: (
    inputs: Array<{ id: string; source: string; filepath: string }>,
    options: { continueOnError: boolean; config: Record<string, unknown> }
  ) => BatchCompileResult;
  parseBlocks: (
    source: string,
    options: { enable_directives: boolean }
  ) => ParseBlocksResult;
  parseFrontmatter: (source: string) => { frontmatter: Record<string, unknown> };
}

/**
 * Compiler instance for single-file compilation.
 */
export interface MarkflowCompiler {
  compile: (
    source: string,
    filename: string,
    options: { file?: string; url?: string }
  ) => CompileResult;
}

/**
 * Result from single-file compilation.
 */
export interface CompileResult {
  code: string;
  map?: unknown;
  frontmatter_json?: string;
  headings?: Array<{ depth: number; slug: string; text: string }>;
  imports?: Array<{ path: string }>;
  diagnostics?: {
    warnings?: Array<{ line: number; message: string }>;
  };
}

/**
 * Result from batch compilation.
 */
export interface BatchCompileResult {
  results: Array<{
    id: string;
    result?: {
      html: string;
      frontmatterJson?: string;
      headings?: Array<{ depth: number; slug: string; text: string }>;
      hoistedImports?: string[];
      hasUserDefaultExport?: boolean;
    };
  }>;
  stats: {
    succeeded: number;
    total: number;
    processingTimeMs: number;
  };
}

/**
 * Result from parsing blocks.
 */
export interface ParseBlocksResult {
  blocks: Array<{
    type: 'html' | 'component';
    content?: string;
    name?: string;
    props?: Record<string, unknown>;
    slotHtml?: string;
  }>;
  headings: Array<{ depth: number; slug: string; text: string }>;
}

/**
 * Plugin options for the Markflow Vite plugin.
 */
export interface MarkflowPluginOptions {
  include?: (id: string) => boolean;
  libraries?: ComponentLibrary[];
  starlightComponents?: boolean | { enabled?: boolean; components?: string[]; module?: string };
  expressiveCode?: boolean | { enabled?: boolean; component?: string; module?: string };
  compiler?: {
    jsx?: {
      code_sample_components?: string[];
    };
  };
  plugins?: MarkflowPlugin[];
  binding?: MarkflowBinding;
  mdx?: MdxImportHandlingOptions;
}

/**
 * Default file extensions to compile.
 */
export const DEFAULT_EXTENSIONS = new Set(['.md', '.mdx']);
