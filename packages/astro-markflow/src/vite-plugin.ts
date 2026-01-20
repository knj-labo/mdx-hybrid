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
import { hasProblematicMdxPatterns } from './utils/mdx-detection.js';
import type { MarkflowPlugin, PluginHooks, TransformContext } from './types.js';

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
  const compilationCache = new Map<string, NonNullable<BatchCompileResult['results'][number]['result']>>();
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

      // Read all files and prepare batch inputs
      const inputs: Array<{ id: string; source: string; filepath: string }> = [];
      for (const file of files) {
        let source = await readFile(file, 'utf8');

        // Apply preprocess hooks (same as load hook does)
        for (const preprocessHook of hooks.preprocess) {
          source = preprocessHook(source, file);
        }

        // Pre-detect problematic patterns - these files will be handled by Astro's MDX plugin
        if (hasProblematicMdxPatterns(source)) {
          fallbackFiles.add(file);
          fallbackReasons.set(file, 'Pre-detected problematic MDX patterns');
        } else {
          inputs.push({ id: file, source, filepath: file });
        }
      }

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
            compilationCache.set(result.id, result.result);
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

      // If file was pre-detected as needing fallback, let other plugins (like @astrojs/mdx) handle it
      if (fallbackFiles.has(resolvedId)) {
        return null;
      }

      const virtualId = `${VIRTUAL_PREFIX}${resolvedId}.markflow.jsx`;
      sourceLookup.set(virtualId, resolvedId);
      return virtualId;
    },

    async load(id) {
      if (!id.startsWith(VIRTUAL_PREFIX)) {
        return null;
      }
      // Lazy initialize compiler on first use
      const currentCompiler = await getCompiler();
      const filename =
        sourceLookup.get(id) ??
        stripQuery(id.slice(VIRTUAL_PREFIX.length).replace(/\.markflow\.jsx$/, ''));

      try {
        const source = await readFile(filename, 'utf8');

        // Apply preprocess hooks
        let processedSource = source;
        for (const preprocessHook of hooks.preprocess) {
          processedSource = preprocessHook(processedSource, filename);
        }

        // Early detection of problematic patterns - skip to fallback
        // Note: Pre-detected files from buildStart are handled by resolveId returning null
        // This catches files that weren't pre-detected (e.g., preprocess hooks revealed the pattern)
        if (hasProblematicMdxPatterns(processedSource)) {
          this.warn(
            `[markflow] Skipping ${filename}: contains patterns incompatible with markdown-rs`
          );
          fallbackFiles.add(filename);
          fallbackReasons.set(filename, 'Detected problematic MDX patterns');
          // Use @mdx-js/mdx as fallback compiler for runtime-detected files
          return compileFallbackModule(filename, processedSource, id);
        }

        const startTime = performance.now();
        let result: CompileResult;
        let frontmatter: Record<string, unknown> = {};
        let headings: Array<{ depth: number; slug: string; text: string }> = [];

        // Check cache first (populated in build mode by buildStart)
        const cached = compilationCache.get(filename);
        const isMdx = filename.endsWith('.mdx');
        const hasUserImports = (cached?.hoistedImports?.length ?? 0) > 0;
        const hasUserDefaultExport = cached?.hasUserDefaultExport === true;
        const hasJsxComponents = cached?.html && /\{\.\.\.|\<[A-Z]/.test(cached.html);

        if (cached && !hasUserImports && !hasUserDefaultExport && !hasJsxComponents && !isMdx) {
          if (cached.frontmatterJson) {
            try {
              frontmatter = JSON.parse(cached.frontmatterJson) as Record<string, unknown>;
            } catch {
              frontmatter = {};
            }
          }
          headings = cached.headings || [];

          const jsxCode = wrapHtmlInJsxModule(cached.html, frontmatter, headings);
          result = {
            code: jsxCode,
            map: null,
            frontmatter_json: cached.frontmatterJson,
            headings,
            imports: [],
          };
        } else if (IS_MDAST) {
          const binding = await loadMarkflowBinding();
          const parseResult = binding.parseBlocks(processedSource, {
            enable_directives: true,
          });
          headings = parseResult.headings;
          const frontmatterResult = binding.parseFrontmatter(processedSource);
          frontmatter = frontmatterResult.frontmatter || {};

          result = {
            code: blocksToJsx(parseResult.blocks, frontmatter, headings, registry),
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
          return compileFallbackModule(filename, processedFallbackSource, id);
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
  headings: Array<{ depth: number; slug: string; text: string }>
): string {
  const frontmatterJson = JSON.stringify(frontmatter);
  const headingsJson = JSON.stringify(headings);

  return `import { Fragment, jsx as _jsx } from 'astro/jsx-runtime';
const _Fragment = Fragment;

export const frontmatter = ${frontmatterJson};
export function getHeadings() { return ${headingsJson}; }
export default function MarkflowContent() {
  return (
    <Fragment set:html={${JSON.stringify(html)}} />
  );
}
`;
}

async function compileFallbackModule(
  filename: string,
  source: string,
  virtualId: string
): Promise<{ code: string; map?: SourceMapInput }> {
  // Use @mdx-js/mdx to compile files that markflow can't handle
  // (e.g., files with import/export statements)
  const compiled = await compileMdx(source, {
    jsxImportSource: 'astro',
    // Don't use providerImportSource as it requires @mdx-js/react
    // which may not be installed
  });

  // The compiled output is a VFile, get the string value
  const mdxCode = String(compiled);

  // Wrap in Astro-compatible module format
  // @mdx-js/mdx outputs ESM with `export default function MDXContent(...)`
  // We need to add Content, frontmatter and getHeadings exports for Astro compatibility
  // Note: MDXContent is the default export function from @mdx-js/mdx
  const wrappedCode = `
${mdxCode}

// Re-export for Astro compatibility
// MDXContent is defined by @mdx-js/mdx as the default export
export const Content = MDXContent;
export const file = ${JSON.stringify(filename)};
export const url = undefined;
export function getHeadings() { return []; }
export const frontmatter = {};
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
