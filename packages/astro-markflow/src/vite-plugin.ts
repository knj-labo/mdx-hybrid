/**
 * Markflow Vite plugin for MDX compilation.
 * @module vite-plugin
 */

import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { transformWithEsbuild, type ResolvedConfig, type Plugin } from 'vite';
import type { SourceMapInput } from 'rollup';
import { compile as compileMdx } from '@mdx-js/mdx';
import remarkGfm from 'remark-gfm';
import remarkDirective from 'remark-directive';
import { parseFragment, serialize } from 'parse5';
import { codeToHtml, createCssVariablesTheme } from 'shiki';
import type { DefaultTreeAdapterMap } from 'parse5';
import {
  createRegistry,
  starlightLibrary,
  astroLibrary,
  expressiveCodeLibrary,
  type ComponentLibrary,
  type Registry,
} from 'markflow/registry';
import { createPipeline } from './pipeline/index.js';
import { blocksToJsx } from './transforms/blocks-to-jsx.js';
import { resolveExpressiveCodeConfig, type ExpressiveCodeConfig } from './utils/config.js';
import { stripFrontmatter } from './utils/frontmatter.js';
import { hasProblematicMdxPatterns } from './utils/mdx-detection.js';
import { collectImportedNames, insertAfterImports } from './utils/imports.js';
import type { MarkflowPlugin, MdxImportHandlingOptions, PluginHooks, TransformContext } from './types.js';

type DocumentFragment = DefaultTreeAdapterMap['documentFragment'];
type Node = DefaultTreeAdapterMap['node'];
type Element = DefaultTreeAdapterMap['element'];
type TextNode = DefaultTreeAdapterMap['textNode'];

// Type for NAPI binding
interface MarkflowBinding {
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

interface MarkflowCompiler {
  compile: (
    source: string,
    filename: string,
    options: { file?: string; url?: string }
  ) => CompileResult;
}

interface CompileResult {
  code: string;
  map?: unknown;
  frontmatter_json?: string;
  headings?: Array<{ depth: number; slug: string; text: string }>;
  imports?: Array<{ path: string }>;
  diagnostics?: {
    warnings?: Array<{ line: number; message: string }>;
  };
}

interface BatchCompileResult {
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

interface ParseBlocksResult {
  blocks: Array<{
    type: 'html' | 'component';
    content?: string;
    name?: string;
    props?: Record<string, unknown>;
    slotHtml?: string;
  }>;
  headings: Array<{ depth: number; slug: string; text: string }>;
}

interface MarkflowPluginOptions {
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

const DEFAULT_EXTENSIONS = new Set(['.md', '.mdx']);

/**
 * Resolves library configuration from options.
 * Supports both new `libraries` API and legacy `starlightComponents` option.
 */
export function resolveLibraries(options: MarkflowPluginOptions): {
  libraries: ComponentLibrary[];
  registry: Registry;
} {
  // New API: explicit libraries array
  if (Array.isArray(options.libraries)) {
    const registry = createRegistry(options.libraries);
    return { libraries: options.libraries, registry };
  }

  // Legacy API: derive libraries from starlightComponents option
  const libraries: ComponentLibrary[] = [astroLibrary];

  if (options.starlightComponents) {
    libraries.push(starlightLibrary);
  }

  if (options.expressiveCode) {
    libraries.push(expressiveCodeLibrary);
  }

  const registry = createRegistry(libraries);
  return { libraries, registry };
}

let bindingPromise: Promise<MarkflowBinding> | undefined;
const VIRTUAL_PREFIX = '\0markflow:';
const DEBUG_BINDING = process.env.MARKFLOW_DEBUG_BINDING === '1';
const ENABLE_SHIKI = process.env.MARKFLOW_SHIKI === '1';
const IS_MDAST = process.env.MARKFLOW_PIPELINE === 'mdast';
const require = createRequire(import.meta.url);

const logBindingSource = (source: string): void => {
  if (!DEBUG_BINDING) return;
  console.info(`[markflow] binding source: ${source}`);
  const nativePath = process.env.NAPI_RS_NATIVE_LIBRARY_PATH;
  if (nativePath) {
    console.info(`[markflow] NAPI_RS_NATIVE_LIBRARY_PATH=${nativePath}`);
  } else {
    console.info('[markflow] NAPI_RS_NATIVE_LIBRARY_PATH is not set');
  }
};

async function loadMarkflowBinding(): Promise<MarkflowBinding> {
  if (!bindingPromise) {
    bindingPromise = (async () => {
      // Load native binding directly via require() on the .node binary to bypass Vite SSR runner.
      const require = createRequire(import.meta.url);
      const pkgRoot = path.dirname(require.resolve('markflow-napi/package.json'));

      const guessBinaryName = () => {
        const triplet = `${process.platform}-${process.arch}`;
        return [
          `markflow.${triplet}.node`,
          `markflow-${triplet}.node`,
          `markflow.${process.platform}-${process.arch}.node`,
        ];
      };

      const findBinaryPath = (): string => {
        const candidates = guessBinaryName().map((name) =>
          path.resolve(pkgRoot, name)
        );
        for (const candidate of candidates) {
          if (require('node:fs').existsSync(candidate)) {
            return candidate;
          }
        }
        // Fallback: first .node in package root
        const entries = require('node:fs').readdirSync(pkgRoot);
        const nodeFile = entries.find((f: string) => f.endsWith('.node'));
        if (nodeFile) {
          return path.resolve(pkgRoot, nodeFile);
        }
        throw new Error('markflow-napi native binary not found');
      };

      const binaryPath = findBinaryPath();
      const binding = require(binaryPath) as MarkflowBinding;
      logBindingSource(binaryPath);
      return binding;
    })();
  }
  return bindingPromise;
}

const stripQuery = (id: string): string => {
  if (!id) return id;
  const queryIndex = id.indexOf('?');
  return queryIndex >= 0 ? id.slice(0, queryIndex) : id;
};

const normalizePath = (value: string): string => value.split(path.sep).join('/');

function deriveAstroUrl(filePath: string, rootDir?: string): string | undefined {
  if (!filePath) return undefined;
  const normalizedFile = normalizePath(filePath);
  const root = rootDir ?? process.cwd();
  const pagesDir = normalizePath(path.join(root, 'src', 'pages'));
  if (!normalizedFile.startsWith(pagesDir)) {
    return undefined;
  }
  let relative = normalizedFile.slice(pagesDir.length);
  if (relative.startsWith('/')) {
    relative = relative.slice(1);
  }
  if (!relative) {
    return '/';
  }
  if (relative.endsWith('.md') || relative.endsWith('.mdx')) {
    relative = relative.replace(/\.mdx?$/, '');
  }
  if (relative === '' || relative === 'index') {
    return '/';
  }
  if (relative.endsWith('/index')) {
    relative = relative.slice(0, -'/index'.length);
  }
  return `/${relative}`;
}

function deriveFileOptions(
  id: string,
  rootDir?: string
): { file: string; url?: string } {
  const sourcePath = stripQuery(id);
  let absolutePath = sourcePath;
  if (rootDir && !path.isAbsolute(sourcePath)) {
    absolutePath = path.resolve(rootDir, sourcePath);
  }
  const url = deriveAstroUrl(absolutePath, rootDir);
  const options: { file: string; url?: string } = { file: absolutePath };
  if (url) {
    options.url = url;
  }
  return options;
}

const shouldCompile = (id: string): boolean =>
  DEFAULT_EXTENSIONS.has(path.extname(stripQuery(id)));

/**
 * Collects hooks from an array of plugins, organizing them by hook type.
 */
function collectHooks(plugins: MarkflowPlugin[]): PluginHooks {
  const hooks: PluginHooks = {
    afterParse: [],
    beforeInject: [],
    beforeOutput: [],
    preprocess: [],
  };

  // Sort plugins: 'pre' first, then undefined, then 'post'
  const sorted = [...plugins].sort((a, b) => {
    const order: Record<string, number> = { pre: 0, undefined: 1, post: 2 };
    const aOrder = order[a.enforce ?? 'undefined'] ?? 1;
    const bOrder = order[b.enforce ?? 'undefined'] ?? 1;
    return aOrder - bOrder;
  });

  for (const plugin of sorted) {
    if (plugin.afterParse) hooks.afterParse.push(plugin.afterParse);
    if (plugin.beforeInject) hooks.beforeInject.push(plugin.beforeInject);
    if (plugin.beforeOutput) hooks.beforeOutput.push(plugin.beforeOutput);
    if (plugin.preprocess) hooks.preprocess.push(plugin.preprocess);
  }

  return hooks;
}

/**
 * Creates the Markflow Vite plugin that intercepts `.md`/`.mdx` files
 * before `@astrojs/mdx` runs.
 */
export function markflowPlugin(userOptions: MarkflowPluginOptions = {}): Plugin {
  let compiler: MarkflowCompiler | undefined;
  let resolvedConfig: ResolvedConfig | undefined;
  const sourceLookup = new Map<string, string>();
  type CachedCompileResult = NonNullable<BatchCompileResult['results'][number]['result']> & {
    originalSource?: string;
    processedSource?: string;
  };
  const originalSourceCache = new Map<string, string>();   // Raw markdown before preprocess hooks
  const processedSourceCache = new Map<string, string>();  // Preprocessed markdown fed to compiler
  const compilationCache = new Map<string, CachedCompileResult>();
  const fallbackFiles = new Set<string>();
  const fallbackReasons = new Map<string, string>();
  const processedFiles = new Set<string>();
  let totalProcessingTimeMs = 0;

  const providedBinding = userOptions.binding ?? null;

  // Collect hooks from plugins
  const plugins = userOptions.plugins ?? [];
  const hooks = collectHooks(plugins);

  // Build compiler options with default code_sample_components
  const compilerOptions = {
    ...(userOptions.compiler ?? {}),
    jsx: {
      ...(userOptions.compiler?.jsx ?? {}),
      code_sample_components:
        userOptions.compiler?.jsx?.code_sample_components ?? ['Code', 'Prism'],
    },
  };

  const include = userOptions.include ?? shouldCompile;
  const starlightComponents = userOptions.starlightComponents ?? false;
  const expressiveCode = resolveExpressiveCodeConfig(
    userOptions.expressiveCode ?? false
  );

  // Resolve libraries and create registry
  const { registry } = resolveLibraries(userOptions);

  // Track whether Starlight is configured for gating default directive handling
  const hasStarlightConfigured = Boolean(userOptions.starlightComponents) ||
    (Array.isArray(userOptions.libraries) &&
     userOptions.libraries.some(lib => lib === starlightLibrary));

  // MDX import handling options
  const mdxOptions = userOptions.mdx;

  const unwrapVirtual = (value: string | undefined): string | undefined =>
    value && value.startsWith(VIRTUAL_PREFIX)
      ? value.slice(VIRTUAL_PREFIX.length)
      : value;

  let shikiReady: Promise<(code: string, lang?: string) => Promise<string>> | undefined;

  const getShiki = (): Promise<(code: string, lang?: string) => Promise<string>> | null => {
    if (!ENABLE_SHIKI || !IS_MDAST) return null;
    if (!shikiReady) {
      shikiReady = createShikiHighlighter();
    }
    return shikiReady;
  };

  // Lazy compiler initialization to avoid Vite module runner timing issues
  const getCompiler = async (): Promise<MarkflowCompiler> => {
    if (!compiler) {
      const binding = providedBinding ?? (await loadMarkflowBinding());
      if (providedBinding) {
        logBindingSource('provided');
      }
      const createCompiler = binding.createCompiler
        ? binding.createCompiler
        : (cfg: Record<string, unknown>) => new binding.MarkflowCompiler!(cfg);
      compiler = createCompiler(compilerOptions);
    }
    return compiler;
  };

  return {
    name: 'vite-plugin-markflow',
    enforce: 'pre',

    configResolved(config) {
      resolvedConfig = config;
      if (config.esbuild == null) {
        (config as { esbuild: object }).esbuild = {
          jsx: 'automatic',
          jsxImportSource: 'astro',
        };
      } else if (config.esbuild !== false) {
        const esbuildConfig = config.esbuild as Record<string, unknown>;
        if (esbuildConfig.jsx == null) {
          esbuildConfig.jsx = 'automatic';
        }
        if (esbuildConfig.jsxImportSource == null) {
          esbuildConfig.jsxImportSource = 'astro';
        }
      }
      // Ensure native binding is treated as external to avoid Vite SSR runner involvement
      const optimizeDeps = (config as Record<string, any>).optimizeDeps ?? {};
      const exclude: string[] = optimizeDeps.exclude ?? [];
      if (!exclude.includes('markflow-napi')) {
        exclude.push('markflow-napi');
      }
      optimizeDeps.exclude = exclude;
      (config as Record<string, any>).optimizeDeps = optimizeDeps;

      const ssr = (config as Record<string, any>).ssr ?? {};
      const ssrExternal: string[] = ssr.external ?? [];
      if (!ssrExternal.includes('markflow-napi')) {
        ssrExternal.push('markflow-napi');
      }
      ssr.external = ssrExternal;
      (config as Record<string, any>).ssr = ssr;
      // Note: Binding/compiler initialization deferred to buildStart/load hooks
      // to avoid Vite module runner timing issues with async imports
    },

    async buildStart() {
      // Only batch compile in build mode (not dev/serve)
      if (resolvedConfig?.command !== 'build') return;

      // Find all MD/MDX files (use CJS require to avoid Vite's module runner)
      const { glob } = require('glob') as {
        glob: (
          pattern: string,
          options: { cwd: string; ignore: string[]; absolute: boolean }
        ) => Promise<string[]>;
      };
      const files = await glob('**/*.{md,mdx}', {
        cwd: resolvedConfig.root,
        ignore: ['node_modules/**', 'dist/**'],
        absolute: true,
      });

      if (files.length === 0) return;

      // Read all files in parallel and prepare batch inputs
      const inputsOrNull = await Promise.all(
        files.map(async (file) => {
          const rawSource = await readFile(file, 'utf8');
          let processedSource = rawSource;

          // Apply preprocess hooks (same as load hook does)
          for (const preprocessHook of hooks.preprocess) {
            processedSource = preprocessHook(processedSource, file);
          }

          // Pre-detect problematic patterns - these files will be handled by Astro's MDX plugin
          if (hasProblematicMdxPatterns(processedSource, mdxOptions)) {
            fallbackFiles.add(file);
            fallbackReasons.set(file, 'Pre-detected problematic MDX patterns');
            return null;
          }

          originalSourceCache.set(file, rawSource);       // For TransformContext.source
          processedSourceCache.set(file, processedSource); // For potential reuse in cache fast path
          return { id: file, source: processedSource, filepath: file };
        })
      );

      const inputs = inputsOrNull.filter(
        (i): i is NonNullable<typeof i> => i !== null
      );

      if (fallbackFiles.size > 0) {
        console.info(
          `[markflow] Pre-detected ${fallbackFiles.size} files with patterns incompatible with markdown-rs (delegating to Astro MDX)`
        );
      }

      if (inputs.length === 0) return;

      try {
        // Batch compile with parallel processing
        const binding = providedBinding ?? (await loadMarkflowBinding());
        const batchResult = binding.compileBatch(inputs, {
          continueOnError: true,
          config: compilerOptions,
        });

        // Cache successful results
        for (const result of batchResult.results) {
          if (result.result) {
            compilationCache.set(result.id, {
              ...result.result,
              originalSource: originalSourceCache.get(result.id),
              processedSource: processedSourceCache.get(result.id),
            });
          }
        }

        console.info(
          `[markflow] Batch compiled ${batchResult.stats.succeeded}/${batchResult.stats.total} files in ${batchResult.stats.processingTimeMs.toFixed(0)}ms`
        );
      } catch (err) {
        this.warn(
          `[markflow] Batch compile skipped due to binding load failure: ${err}`
        );
      }
    },

    async resolveId(sourceId, importer) {
      if (sourceId.startsWith(VIRTUAL_PREFIX)) {
        return sourceId;
      }
      const normalizedImporter = stripQuery(unwrapVirtual(importer) ?? '');
      const normalizedSource = unwrapVirtual(sourceId) ?? sourceId;
      const cleanId = stripQuery(normalizedSource);
      if (!include(cleanId)) {
        if (
          importer?.startsWith(VIRTUAL_PREFIX) &&
          normalizedImporter &&
          !path.isAbsolute(sourceId) &&
          sourceId.startsWith('.')
        ) {
          return path.resolve(path.dirname(normalizedImporter), sourceId);
        }
        return null;
      }
      const resolved = await this.resolve(cleanId, normalizedImporter, {
        skipSelf: true,
      });
      const fallback = (): string => {
        if (path.isAbsolute(cleanId)) {
          return cleanId;
        }
        if (normalizedImporter) {
          return path.resolve(path.dirname(normalizedImporter), cleanId);
        }
        return cleanId;
      };
      const resolvedId =
        resolved && resolved.id
          ? stripQuery(unwrapVirtual(resolved.id) ?? resolved.id)
          : fallback();

      // Pre-detected fallback files should be handled by Astro's MDX plugin
      // which has proper remark-directive support and user-configured plugins
      if (fallbackFiles.has(resolvedId)) {
        return null;
      }

      // Dev mode pre-detection: check if file needs fallback before returning virtualId
      // This ensures dev mode delegates problematic files to Astro MDX just like build mode does
      if (resolvedConfig?.command !== 'build') {
        try {
          const source = await readFile(resolvedId, 'utf8');
          let processedSource = source;
          for (const preprocessHook of hooks.preprocess) {
            processedSource = preprocessHook(processedSource, resolvedId);
          }
          if (hasProblematicMdxPatterns(processedSource, mdxOptions)) {
            fallbackFiles.add(resolvedId);
            fallbackReasons.set(resolvedId, 'Pre-detected problematic MDX patterns (dev mode)');
            return null; // Delegate to Astro's MDX plugin
          }
        } catch {
          // File read failed, let normal path handle it
        }
      }

      const virtualId = `${VIRTUAL_PREFIX}${resolvedId}.markflow.jsx`;
      sourceLookup.set(virtualId, resolvedId);
      return virtualId;
    },

    async load(id) {
      if (!id.startsWith(VIRTUAL_PREFIX)) {
        return null;
      }
      const filename =
        sourceLookup.get(id) ??
        stripQuery(id.slice(VIRTUAL_PREFIX.length).replace(/\.markflow\.jsx$/, ''));

      try {
        // Check cache FIRST, before any file I/O (populated in build mode by buildStart)
        const cached = compilationCache.get(filename);
        const isMdx = filename.endsWith('.mdx');

        if (cached && !isMdx) {
          const hasUserImports = (cached.hoistedImports?.length ?? 0) > 0;
          const hasUserDefaultExport = cached.hasUserDefaultExport === true;
          const hasJsxComponents = cached.html && /\{\.\.\.|\<[A-Z]/.test(cached.html);

          if (!hasUserImports && !hasUserDefaultExport && !hasJsxComponents) {
            // FAST PATH: Use cached result without file I/O
            const startTime = performance.now();
            let frontmatter: Record<string, unknown> = {};
            if (cached.frontmatterJson) {
              try {
                frontmatter = JSON.parse(cached.frontmatterJson) as Record<string, unknown>;
              } catch {
                frontmatter = {};
              }
            }
            const headings = cached.headings || [];

            const jsxCode = wrapHtmlInJsxModule(cached.html, frontmatter, headings, filename);
            const result: CompileResult = {
              code: jsxCode,
              map: null,
              frontmatter_json: cached.frontmatterJson,
              headings,
              imports: [],
            };

            const endTime = performance.now();
            totalProcessingTimeMs += endTime - startTime;
            processedFiles.add(filename);

            const shikiHighlighter = getShiki();
            const normalizedStarlightComponents:
              | boolean
              | { components?: string[]; module?: string } =
              typeof starlightComponents === 'object' && starlightComponents !== null
                ? { components: starlightComponents.components, module: starlightComponents.module }
                : Boolean(starlightComponents);
            const sourceForHooks =
              originalSourceCache.get(filename) ??
              cached.originalSource ??
              processedSourceCache.get(filename) ??
              cached.processedSource ??
              (await readFile(filename, 'utf8'));
            const ctx: TransformContext = {
              code: result.code,
              source: sourceForHooks, // Preserve markdown for user hooks
              filename,
              frontmatter,
              headings,
              registry,
              config: {
                expressiveCode,
                starlightComponents: normalizedStarlightComponents,
                shiki: shikiHighlighter ? await shikiHighlighter : null,
              },
            };

            const transformPipeline = createPipeline({
              afterParse: hooks.afterParse,
              beforeInject: hooks.beforeInject,
              beforeOutput: hooks.beforeOutput,
            });

            const transformed = await transformPipeline(ctx);
            result.code = transformed.code;

            const esbuildResult = await transformWithEsbuild(result.code, id, {
              loader: 'jsx',
              jsx: 'transform',
              jsxFactory: '_jsx',
              jsxFragment: '_Fragment',
            });

            return {
              code: esbuildResult.code,
              map: esbuildResult.map ?? result.map ?? undefined,
            };
          }
        }

        // Lazy initialize compiler on first use (only needed for cache miss path)
        const currentCompiler = await getCompiler();

        // Only read file if cache miss
        const source = await readFile(filename, 'utf8');
        originalSourceCache.set(filename, source);

        // Apply preprocess hooks
        let processedSource = source;
        for (const preprocessHook of hooks.preprocess) {
          processedSource = preprocessHook(processedSource, filename);
        }
        processedSourceCache.set(filename, processedSource);

        // Early detection of problematic patterns - skip to fallback
        // Note: Pre-detected files from buildStart are handled by resolveId returning null
        // This catches files that weren't pre-detected (e.g., preprocess hooks revealed the pattern)
        if (hasProblematicMdxPatterns(processedSource, mdxOptions)) {
          this.warn(
            `[markflow] Skipping ${filename}: contains patterns incompatible with markdown-rs`
          );
          fallbackFiles.add(filename);
          fallbackReasons.set(filename, 'Detected problematic MDX patterns');
          // Use @mdx-js/mdx as fallback compiler for runtime-detected files
          return compileFallbackModule(filename, processedSource, id, registry, hasStarlightConfigured);
        }

        const startTime = performance.now();
        let result: CompileResult;
        let frontmatter: Record<string, unknown> = {};
        let headings: Array<{ depth: number; slug: string; text: string }> = [];

        if (IS_MDAST) {
          const binding = await loadMarkflowBinding();

          // Strip frontmatter before passing to parseBlocks
          // Otherwise the mdast pipeline renders YAML as regular text
          const contentSource = stripFrontmatter(processedSource);

          const parseResult = binding.parseBlocks(contentSource, {
            enable_directives: true,
          });
          headings = parseResult.headings;

          // Extract frontmatter from original source (before stripping)
          const frontmatterResult = binding.parseFrontmatter(processedSource);
          frontmatter = frontmatterResult.frontmatter || {};

          result = {
            code: blocksToJsx(parseResult.blocks, frontmatter, headings, registry, filename),
            map: null,
            frontmatter_json: JSON.stringify(frontmatter),
            headings,
            imports: [],
          };
        } else {
          const fileOptions = deriveFileOptions(filename, resolvedConfig?.root);
          result = currentCompiler.compile(processedSource, filename, fileOptions);
          if (result.frontmatter_json) {
            try {
              frontmatter = JSON.parse(result.frontmatter_json) as Record<string, unknown>;
            } catch {
              frontmatter = {};
            }
          }
          headings = result.headings || [];
        }

        const endTime = performance.now();
        totalProcessingTimeMs += endTime - startTime;
        processedFiles.add(filename);

        if (result.code == null || typeof result.code !== 'string') {
          throw new Error(`Compiler returned undefined or invalid code for ${filename}`);
        }

        if (result.diagnostics?.warnings?.length) {
          for (const warning of result.diagnostics.warnings) {
            this.warn(`[markflow] ${filename}:${warning.line}: ${warning.message}`);
          }
        }

        const shikiHighlighter = getShiki();
        // Normalize starlightComponents to match TransformConfig type
        const normalizedStarlightComponents: boolean | { components?: string[]; module?: string } =
          typeof starlightComponents === 'object' && starlightComponents !== null
            ? { components: starlightComponents.components, module: starlightComponents.module }
            : Boolean(starlightComponents);
        const ctx: TransformContext = {
          code: result.code,
          source,
          filename,
          frontmatter,
          headings,
          registry,
          config: {
            expressiveCode,
            starlightComponents: normalizedStarlightComponents,
            shiki: shikiHighlighter ? await shikiHighlighter : null,
          },
        };

        const transformPipeline = createPipeline({
          afterParse: hooks.afterParse,
          beforeInject: hooks.beforeInject,
          beforeOutput: hooks.beforeOutput,
        });

        const transformed = await transformPipeline(ctx);
        result.code = transformed.code;

        if (Array.isArray(result?.imports)) {
          for (const dep of result.imports) {
            if (dep?.path) {
              this.addWatchFile(dep.path);
            }
          }
        }

        const esbuildResult = await transformWithEsbuild(result.code, id, {
          loader: 'jsx',
          jsx: 'transform',
          jsxFactory: '_jsx',
          jsxFragment: '_Fragment',
        });

        return {
          code: esbuildResult.code,
          map: esbuildResult.map ?? result.map ?? undefined,
        };
      } catch (error) {
        const message = (error as Error)?.message || String(error);
        const shouldFallback =
          message.includes('Vite module runner has been closed') ||
          message.includes('Markdown parser error') ||
          message.includes('Markdown parse error') ||
          message.includes('Transform failed') ||
          message.includes('Compiler returned undefined') ||
          message.includes('Cannot read properties of undefined') ||
          message.includes('Cannot read properties of null');

        if (shouldFallback) {
          fallbackFiles.add(filename);
          fallbackReasons.set(filename, message);
          this.warn(`[markflow] Falling back to @mdx-js/mdx for ${filename}: ${message}`);

          // Try to invalidate the module in dev server mode
          const config = resolvedConfig as unknown as {
            server?: {
              moduleGraph?: {
                getModuleById: (id: string) => object | null;
                invalidateModule: (mod: object) => void;
              };
            };
          };
          if (config?.server?.moduleGraph) {
            const mod = config.server.moduleGraph.getModuleById(id);
            if (mod) {
              config.server.moduleGraph.invalidateModule(mod);
            }
          }

          // Re-read and process the file for fallback compilation
          const fallbackSource = await readFile(filename, 'utf8');
          let processedFallbackSource = fallbackSource;
          for (const preprocessHook of hooks.preprocess) {
            processedFallbackSource = preprocessHook(processedFallbackSource, filename);
          }
          return compileFallbackModule(filename, processedFallbackSource, id, registry, hasStarlightConfigured);
        }
        throw new Error(`[markflow] Compile failed for ${filename}: ${message}`);
      }
    },

    async buildEnd() {
      if (process.env.MARKFLOW_STATS !== '1') return;

      const totalFiles = processedFiles.size + fallbackFiles.size;

      const stats = {
        timestamp: new Date().toISOString(),
        totalFiles,
        processedByMarkflow: processedFiles.size,
        handledByAstro: fallbackFiles.size,
        handledByAstroRate:
          totalFiles > 0
            ? `${((fallbackFiles.size / totalFiles) * 100).toFixed(2)}%`
            : '0%',
        preValidationSkips: {
          count: 0,
          files: [] as string[],
        },
        runtimeFallbacks: {
          count: fallbackFiles.size,
          files: Array.from(fallbackFiles).map((file) => ({
            file: file.replace(resolvedConfig?.root ?? '', ''),
            reason: fallbackReasons.get(file) ?? 'unknown',
          })),
        },
        fallbacks: fallbackFiles.size,
        fallbackRate:
          totalFiles > 0
            ? `${((fallbackFiles.size / totalFiles) * 100).toFixed(2)}%`
            : '0%',
        fallbackFiles: Array.from(fallbackFiles).map((file) => ({
          file: file.replace(resolvedConfig?.root ?? '', ''),
          reason: fallbackReasons.get(file) ?? 'unknown',
        })),
        performance: {
          totalProcessingTimeMs: Math.round(totalProcessingTimeMs * 100) / 100,
          averageFileTimeMs:
            processedFiles.size > 0
              ? Math.round((totalProcessingTimeMs / processedFiles.size) * 100) / 100
              : 0,
        },
      };

      const { writeFile } = await import('node:fs/promises');
      const outputPath = path.join(resolvedConfig?.root ?? '.', 'markflow-stats.json');
      await writeFile(outputPath, JSON.stringify(stats, null, 2));
      console.info(`[markflow] Stats written to ${outputPath}`);
    },
  };
}

/**
 * Wrap raw HTML from batch compilation in a JSX module structure.
 */
function wrapHtmlInJsxModule(
  html: string,
  frontmatter: Record<string, unknown>,
  headings: Array<{ depth: number; slug: string; text: string }>,
  filename: string
): string {
  const frontmatterJson = JSON.stringify(frontmatter);
  const headingsJson = JSON.stringify(headings);

  return `import { createComponent, renderJSX } from 'astro/runtime/server/index.js';
import { Fragment, jsx as _jsx } from 'astro/jsx-runtime';

export const frontmatter = ${frontmatterJson};
export function getHeadings() { return ${headingsJson}; }
function _Content() {
  return (
    <Fragment set:html={${JSON.stringify(html)}} />
  );
}
const MarkflowContent = createComponent(
  (result, props, _slots) => renderJSX(result, _jsx(_Content, { ...props })),
  ${JSON.stringify(filename)}
);
export const Content = MarkflowContent;
export default MarkflowContent;
`;
}

async function compileFallbackModule(
  filename: string,
  source: string,
  virtualId: string,
  registry: Registry | null,
  hasStarlightConfigured: boolean
): Promise<{ code: string; map?: SourceMapInput }> {
  let frontmatter: Record<string, unknown> = {};
  try {
    const binding = await loadMarkflowBinding();
    const frontmatterResult = binding.parseFrontmatter(source);
    frontmatter = frontmatterResult.frontmatter || {};
  } catch {
    frontmatter = {};
  }

  let sourceWithoutFrontmatter = stripFrontmatter(source);
  const directiveResult = rewriteFallbackDirectives(sourceWithoutFrontmatter, registry, hasStarlightConfigured);
  if (directiveResult.changed) {
    sourceWithoutFrontmatter = injectFallbackImports(
      directiveResult.code,
      directiveResult.usedComponents,
      registry,
      hasStarlightConfigured
    );
  }
  // Use @mdx-js/mdx to compile files that markflow can't handle
  // (e.g., files with import/export statements)
  // Include remark-gfm for GFM features (tables, strikethrough, task lists)
  // and remark-directive to handle unconverted ::: directives gracefully
  const compiled = await compileMdx(sourceWithoutFrontmatter, {
    jsxImportSource: 'astro',
    remarkPlugins: [remarkGfm, remarkDirective],
    // Don't use providerImportSource as it requires @mdx-js/react
    // which may not be installed
  });

  // The compiled output is a VFile, get the string value
  const mdxCode = String(compiled);

  // Normalize MDX default export so we can wrap with Astro createComponent
  const mdxWithoutDefault = mdxCode
    .replace(/export default function MDXContent/g, 'function MDXContent')
    .replace(/export default MDXContent\s*;/g, '')
    .replace(/export\s*\{\s*MDXContent\s+as\s+default\s*\};?/g, '');

  // Wrap in Astro-compatible module format
  // @mdx-js/mdx outputs ESM with `export default function MDXContent(...)`
  // We need to add Content, frontmatter and getHeadings exports for Astro compatibility
  // Note: MDXContent is the default export function from @mdx-js/mdx
  const wrappedCode = `
import { createComponent, renderJSX } from 'astro/runtime/server/index.js';
import { Fragment } from 'astro/jsx-runtime';
${mdxWithoutDefault}

// Re-export for Astro compatibility
// Wrap MDXContent so it renders as an Astro component factory
const MarkflowContent = createComponent(
  (result, props, _slots) =>
    renderJSX(
      result,
      MDXContent({
        ...(props ?? {}),
        // Ensure Astro's Fragment is available for <Fragment slot="..."> usage in MDX.
        components: { ...(props?.components ?? {}), Fragment },
      })
    ),
  ${JSON.stringify(filename)}
);
export { MDXContent };
export const Content = MarkflowContent;
export const file = ${JSON.stringify(filename)};
export const url = undefined;
export function getHeadings() { return []; }
export const frontmatter = ${JSON.stringify(frontmatter)};
export default MarkflowContent;
`;

  // Transform JSX through esbuild (same as the main compilation path)
  const esbuildResult = await transformWithEsbuild(wrappedCode, virtualId, {
    loader: 'jsx',
    jsx: 'transform',
    jsxFactory: '_jsx',
    jsxFragment: '_Fragment',
  });

  return {
    code: esbuildResult.code,
    map: esbuildResult.map as SourceMapInput | undefined,
  };
}

type DirectiveOpening = {
  name: string;
  bracketTitle: string | null;
  rawAttrs: string;
  prefix: string;  // Leading whitespace and blockquote markers (e.g., "  ", "> ", "  > > ")
  componentName: string;
};

// Default Starlight directives for fallback when registry is empty
const DEFAULT_STARLIGHT_DIRECTIVES: Record<string, {
  component: string;
  injectProps?: Record<string, { source: string; value?: string }>;
}> = {
  note: { component: 'Aside', injectProps: { type: { source: 'directive_name' } } },
  tip: { component: 'Aside', injectProps: { type: { source: 'directive_name' } } },
  caution: { component: 'Aside', injectProps: { type: { source: 'directive_name' } } },
  danger: { component: 'Aside', injectProps: { type: { source: 'directive_name' } } },
  warning: { component: 'Aside', injectProps: { type: { source: 'directive_name' } } },
  info: { component: 'Aside', injectProps: { type: { source: 'directive_name' } } },
};

function rewriteFallbackDirectives(
  source: string,
  registry: Registry | null,
  hasStarlightConfigured: boolean
): { code: string; usedComponents: Set<string>; changed: boolean } {
  if (!source) {
    return { code: source, usedComponents: new Set(), changed: false };
  }

  // Get directives from registry, fall back to defaults
  const registryDirectives = registry?.getSupportedDirectives().map((name) => name.toLowerCase()) ?? [];
  const supportedSet = new Set(registryDirectives);

  // Add default Starlight directives only if registry is empty AND Starlight is configured
  const useDefaultDirectives = supportedSet.size === 0 && hasStarlightConfigured;
  if (useDefaultDirectives) {
    for (const dir of Object.keys(DEFAULT_STARLIGHT_DIRECTIVES)) {
      supportedSet.add(dir);
    }
  }

  const lines = source.split(/\r?\n/);
  const output: string[] = [];
  const stack: DirectiveOpening[] = [];
  const usedComponents = new Set<string>();
  let changed = false;
  let inFence = false;
  let fenceChar: string | null = null;

  for (const line of lines) {
    // Extract prefix (whitespace + blockquote markers) like we do for directives
    const prefixMatch = line.match(/^(\s*(?:>\s*)*)/);
    const prefix = prefixMatch?.[1] ?? '';
    const afterPrefix = line.slice(prefix.length);

    // Check for code fence after stripping prefix (handles blockquoted code fences)
    const fenceMatch = afterPrefix.match(/^([`~]{3,})/);
    if (fenceMatch) {
      const char = fenceMatch[1]?.[0] ?? null;
      if (!inFence) {
        inFence = true;
        fenceChar = char;
      } else if (char && fenceChar === char) {
        inFence = false;
        fenceChar = null;
      }
      output.push(line);
      continue;
    }

    if (inFence) {
      output.push(line);
      continue;
    }

    const opening = parseOpeningDirective(afterPrefix, supportedSet, prefix);
    if (opening) {
      // Try registry first, then fall back to defaults
      const mapping = registry?.getDirectiveMapping(opening.name)
        ?? (useDefaultDirectives ? DEFAULT_STARLIGHT_DIRECTIVES[opening.name] : null);
      if (!mapping) {
        output.push(line);
        continue;
      }

      const componentName = mapping.component;
      const props: string[] = ['data-mf-source="directive"'];
      if (mapping.injectProps) {
        for (const [propKey, propSource] of Object.entries(mapping.injectProps)) {
          if (propSource.source === 'directive_name') {
            props.push(`${propKey}="${escapeAttributeValue(opening.name)}"`);
          } else if (propSource.source === 'bracket_title' && opening.bracketTitle) {
            props.push(`${propKey}="${escapeAttributeValue(opening.bracketTitle)}"`);
          } else if (propSource.source === 'literal' && propSource.value) {
            props.push(`${propKey}="${escapeAttributeValue(propSource.value)}"`);
          }
        }
      }

      if (opening.bracketTitle) {
        props.push(`title="${escapeAttributeValue(opening.bracketTitle)}"`);
      }
      if (opening.rawAttrs) {
        props.push(opening.rawAttrs);
      }

      const propsStr = props.length > 0 ? ` ${props.join(' ')}` : '';
      output.push(`${opening.prefix}<${componentName}${propsStr}>`);
      stack.push({ ...opening, componentName });
      usedComponents.add(componentName);
      changed = true;
      continue;
    }

    const closer = parseDirectiveCloser(afterPrefix, prefix);
    if (closer && stack.length > 0) {
      const opened = stack.pop();
      if (opened) {
        output.push(`${opened.prefix}</${opened.componentName}>`);
        changed = true;
        continue;
      }
    }

    output.push(line);
  }

  while (stack.length > 0) {
    const opened = stack.pop();
    if (opened) {
      output.push(`${opened.prefix}</${opened.componentName}>`);
    }
  }

  return { code: output.join('\n'), usedComponents, changed };
}

function injectFallbackImports(
  source: string,
  usedComponents: Set<string>,
  registry: Registry | null,
  hasStarlightConfigured: boolean
): string {
  if (!source || usedComponents.size === 0) {
    return source;
  }

  const imported = collectImportedNames(source);
  const importLines: string[] = [];

  for (const componentName of usedComponents) {
    if (imported.has(componentName)) {
      continue;
    }
    const def = registry?.getComponent(componentName);
    if (def) {
      if (def.exportType === 'named') {
        importLines.push(`import { ${componentName} } from '${def.modulePath}';`);
      } else {
        importLines.push(`import ${componentName} from '${def.modulePath}/${componentName}.astro';`);
      }
    } else if (componentName === 'Aside' && hasStarlightConfigured) {
      // Fallback for Starlight Aside component when using default directives
      // Only inject if Starlight is actually configured to avoid module-not-found errors
      importLines.push(`import { Aside } from '@astrojs/starlight/components';`);
    }
  }

  if (importLines.length === 0) {
    return source;
  }

  return insertAfterImports(source, importLines.join('\n'));
}

function parseOpeningDirective(
  afterPrefix: string,
  supported: Set<string>,
  prefix: string
): { name: string; bracketTitle: string | null; rawAttrs: string; prefix: string } | null {
  // Content is already after the prefix; check for directive start
  if (!afterPrefix.startsWith(':::')) {
    return null;
  }

  let rest = afterPrefix.slice(3);
  let name = '';
  while (rest.length > 0 && /[A-Za-z]/.test(rest[0] ?? '')) {
    name += (rest[0] ?? '').toLowerCase();
    rest = rest.slice(1);
  }

  if (!name || !supported.has(name)) {
    return null;
  }

  let bracketTitle: string | null = null;
  if (rest.startsWith('[')) {
    rest = rest.slice(1);
    let title = '';
    while (rest.length > 0) {
      const ch = rest[0] ?? '';
      rest = rest.slice(1);
      if (ch === ']') {
        bracketTitle = title;
        break;
      }
      title += ch;
    }
  }

  const rawAttrs = normalizeDirectiveAttrs(rest.trim(), Boolean(bracketTitle));
  return { name, bracketTitle, rawAttrs, prefix };
}

function normalizeDirectiveAttrs(attrs: string, hasBracketTitle: boolean): string {
  if (!attrs) {
    return '';
  }

  // Strip outer braces from remark-directive syntax: {key="value"} → key="value"
  let normalized = attrs.trim();
  if (normalized.startsWith('{') && normalized.endsWith('}')) {
    normalized = normalized.slice(1, -1).trim();
  }

  const tokens = normalized.split(/\s+/).filter(Boolean);
  const cleaned: string[] = [];
  for (const tok of tokens) {
    const key = tok.split('=')[0]?.trim() ?? '';
    if (!key) continue;
    const lower = key.toLowerCase();
    if (lower === 'type') continue;
    if (hasBracketTitle && lower === 'title') continue;
    cleaned.push(tok);
  }
  return cleaned.join(' ');
}

function parseDirectiveCloser(afterPrefix: string, prefix: string): { prefix: string } | null {
  // Check if the content after prefix is exactly `:::`
  if (afterPrefix.trim() === ':::') {
    return { prefix };
  }
  return null;
}

function escapeAttributeValue(value: string): string {
  return value.replace(/&/g, '&amp;').replace(/"/g, '&quot;');
}

async function createShikiHighlighter(): Promise<(code: string, lang?: string) => Promise<string>> {
  const theme = createCssVariablesTheme({
    name: 'astro-code',
    variablePrefix: '--astro-code-',
  });
  const cache = new Map<string, { lang: string }>();

  return async (code: string, lang?: string): Promise<string> => {
    const key = `${lang || 'text'}`;
    let cached = cache.get(key);
    if (!cached) {
      cached = { lang: lang || 'text' };
      cache.set(key, cached);
    }
    const html = await codeToHtml(code, {
      lang: cached.lang,
      theme,
    });
    return html.replace(/<pre class="([^"]*)"/, (_match, classes: string) => {
      const normalized = classes
        .split(/\s+/)
        .filter((value) => value && value !== 'shiki')
        .join(' ');
      const next = normalized ? `astro-code ${normalized}` : 'astro-code';
      return `<pre class="${next}"`;
    });
  };
}
