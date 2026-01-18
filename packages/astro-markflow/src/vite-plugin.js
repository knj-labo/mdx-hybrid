import { readFile } from "node:fs/promises";
import path from "node:path";
import { transformWithEsbuild } from "vite";
import { parseFragment, serialize } from "parse5";
import { codeToHtml, createCssVariablesTheme } from "shiki";
import { createRegistry, starlightLibrary, astroLibrary, expressiveCodeLibrary } from "markflow/registry";
import { pipe, when } from "./pipeline/pipe.js";
import {
  transformExpressiveCode,
  transformInjectComponentsFromRegistry,
  transformShikiHighlight,
} from "./transforms/index.js";
import { blocksToJsx } from "./transforms/blocks-to-jsx.js";
import { resolveExpressiveCodeConfig } from "./utils/config.js";

const DEFAULT_EXTENSIONS = new Set([".md", ".mdx"]);

/**
 * Resolves library configuration from options.
 * Supports both new `libraries` API and legacy `starlightComponents` option.
 *
 * @param {object} options - Plugin options
 * @returns {{ libraries: Array, registry: import('markflow/registry').ComponentRegistry }}
 */
export function resolveLibraries(options) {
  // New API: explicit libraries array
  if (Array.isArray(options.libraries)) {
    const registry = createRegistry(options.libraries);
    return { libraries: options.libraries, registry };
  }

  // Legacy API: derive libraries from starlightComponents option
  const libraries = [astroLibrary];

  if (options.starlightComponents) {
    libraries.push(starlightLibrary);
  }

  if (options.expressiveCode) {
    libraries.push(expressiveCodeLibrary);
  }

  const registry = createRegistry(libraries);
  return { libraries, registry };
}
let bindingPromise;
const VIRTUAL_PREFIX = "\0markflow:";
const DEBUG_BINDING = process.env.MARKFLOW_DEBUG_BINDING === "1";
const ENABLE_SHIKI = process.env.MARKFLOW_SHIKI === "1";
const IS_MDAST = process.env.MARKFLOW_PIPELINE === "mdast";

const logBindingSource = (source) => {
  if (!DEBUG_BINDING) return;
  console.info(`[markflow] binding source: ${source}`);
  const nativePath = process.env.NAPI_RS_NATIVE_LIBRARY_PATH;
  if (nativePath) {
    console.info(`[markflow] NAPI_RS_NATIVE_LIBRARY_PATH=${nativePath}`);
  } else {
    console.info("[markflow] NAPI_RS_NATIVE_LIBRARY_PATH is not set");
  }
};

async function loadMarkflowBinding() {
  if (!bindingPromise) {
    bindingPromise = (async () => {
      let source = "markflow-napi";
      try {
        const binding = await import("markflow-napi");
        logBindingSource(source);
        return binding;
      } catch (error) {
        if (error?.code !== "ERR_MODULE_NOT_FOUND") {
          throw error;
        }
        const fallbackUrl = new URL(
          "../../../crates/napi/index.js",
          import.meta.url,
        );
        source = fallbackUrl.href;
        const binding = await import(/* @vite-ignore */ fallbackUrl.href).catch(
          (fallbackError) => {
            console.error(
              "[markflow] Failed to load fallback NAPI binding",
              fallbackError,
            );
            throw fallbackError;
          },
        );
        logBindingSource(source);
        return binding;
      }
    })();
  }
  return bindingPromise;
}

const stripQuery = (id) => {
  if (!id) return id;
  const queryIndex = id.indexOf("?");
  return queryIndex >= 0 ? id.slice(0, queryIndex) : id;
};

const normalizePath = (value) => value.split(path.sep).join("/");

function deriveAstroUrl(filePath, rootDir) {
  if (!filePath) return undefined;
  const normalizedFile = normalizePath(filePath);
  const root = rootDir ?? process.cwd();
  const pagesDir = normalizePath(path.join(root, "src", "pages"));
  if (!normalizedFile.startsWith(pagesDir)) {
    return undefined;
  }
  let relative = normalizedFile.slice(pagesDir.length);
  if (relative.startsWith("/")) {
    relative = relative.slice(1);
  }
  if (!relative) {
    return "/";
  }
  if (relative.endsWith(".md") || relative.endsWith(".mdx")) {
    relative = relative.replace(/\.mdx?$/, "");
  }
  if (relative === "" || relative === "index") {
    return "/";
  }
  if (relative.endsWith("/index")) {
    relative = relative.slice(0, -"/index".length);
  }
  return `/${relative}`;
}

function deriveFileOptions(id, rootDir) {
  const sourcePath = stripQuery(id);
  let absolutePath = sourcePath;
  if (rootDir && !path.isAbsolute(sourcePath)) {
    absolutePath = path.resolve(rootDir, sourcePath);
  }
  const url = deriveAstroUrl(absolutePath, rootDir);
  const options = { file: absolutePath };
  if (url) {
    options.url = url;
  }
  return options;
}

const shouldCompile = (id) =>
  DEFAULT_EXTENSIONS.has(path.extname(stripQuery(id)));

/**
 * Collects hooks from an array of plugins, organizing them by hook type.
 * @param {import('./types.js').MarkflowPlugin[]} plugins - Array of plugins
 * @returns {import('./types.js').PluginHooks} Collected hooks
 */
function collectHooks(plugins) {
  const hooks = {
    afterParse: [],
    beforeInject: [],
    beforeOutput: [],
    preprocess: [],
  };

  // Sort plugins: 'pre' first, then undefined, then 'post'
  const sorted = [...plugins].sort((a, b) => {
    const order = { pre: 0, undefined: 1, post: 2 };
    const aOrder = order[a.enforce] ?? 1;
    const bOrder = order[b.enforce] ?? 1;
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
 *
 * Options:
 * - starlightComponents: true | { module?: string, components?: string[] }
 *   Auto-imports Starlight components when their JSX tags appear in output.
 * - plugins: Array of MarkflowPlugin objects to extend transform behavior
 */
export function markflowPlugin(userOptions = {}) {
  let compiler;
  let resolvedConfig;
  const sourceLookup = new Map();
  const compilationCache = new Map(); // filename -> compiled result (build mode)
  const fallbackFiles = new Set();
  const fallbackReasons = new Map(); // file -> reason
  const processedFiles = new Set();
  let totalProcessingTimeMs = 0;

  const providedBinding = userOptions.binding ?? null;

  // Collect hooks from plugins
  /** @type {import('./types.js').MarkflowPlugin[]} */
  const plugins = userOptions.plugins ?? [];
  const hooks = collectHooks(plugins);

  // Build compiler options with default code_sample_components
  const compilerOptions = {
    ...(userOptions.compiler ?? {}),
    jsx: {
      ...(userOptions.compiler?.jsx ?? {}),
      code_sample_components:
        userOptions.compiler?.jsx?.code_sample_components ?? ["Code", "Prism"],
    },
  };

  const include = userOptions.include ?? shouldCompile;
  const starlightComponents = userOptions.starlightComponents ?? false;
  const expressiveCode = resolveExpressiveCodeConfig(
    userOptions.expressiveCode ?? false,
  );

  // Resolve libraries and create registry
  const { registry } = resolveLibraries(userOptions);

  const unwrapVirtual = (value) =>
    value && value.startsWith(VIRTUAL_PREFIX)
      ? value.slice(VIRTUAL_PREFIX.length)
      : value;

  let shikiReady;

  const getShiki = () => {
    if (!ENABLE_SHIKI || !IS_MDAST) return null;
    if (!shikiReady) {
      shikiReady = createShikiHighlighter();
    }
    return shikiReady;
  };

  return {
    name: "vite-plugin-markflow",
    enforce: "pre",
    async configResolved(config) {
      resolvedConfig = config;
      if (config.esbuild == null) {
        config.esbuild = {
          jsx: "automatic",
          jsxImportSource: "astro",
        };
      } else if (config.esbuild !== false) {
        if (config.esbuild.jsx == null) {
          config.esbuild.jsx = "automatic";
        }
        if (config.esbuild.jsxImportSource == null) {
          config.esbuild.jsxImportSource = "astro";
        }
      }
      const binding = providedBinding ?? (await loadMarkflowBinding());
      if (providedBinding) {
        logBindingSource("provided");
      }
      const createCompiler = binding.createCompiler
        ? binding.createCompiler
        : (cfg) => new binding.MarkflowCompiler(cfg);
      compiler = createCompiler(compilerOptions);
    },
    async buildStart() {
      // Only batch compile in build mode (not dev/serve)
      if (resolvedConfig.command !== 'build') return;

      // Find all MD/MDX files
      const { glob } = await import('glob');
      const files = await glob('**/*.{md,mdx}', {
        cwd: resolvedConfig.root,
        ignore: ['node_modules/**', 'dist/**'],
        absolute: true,
      });

      if (files.length === 0) return;

      // Read all files and prepare batch inputs
      const inputs = await Promise.all(
        files.map(async (file) => ({
          id: file,
          source: await readFile(file, 'utf8'),
          filepath: file,
        }))
      );

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

      console.info(`[markflow] Batch compiled ${batchResult.stats.succeeded}/${batchResult.stats.total} files in ${batchResult.stats.processingTimeMs.toFixed(0)}ms`);
    },
    async resolveId(sourceId, importer) {
      if (sourceId.startsWith(VIRTUAL_PREFIX)) {
        return sourceId;
      }
      const normalizedImporter = stripQuery(unwrapVirtual(importer));
      const normalizedSource = unwrapVirtual(sourceId);
      const cleanId = stripQuery(normalizedSource);
      if (!include(cleanId)) {
        if (
          importer?.startsWith(VIRTUAL_PREFIX) &&
          normalizedImporter &&
          !path.isAbsolute(sourceId) &&
          sourceId.startsWith(".")
        ) {
          return path.resolve(path.dirname(normalizedImporter), sourceId);
        }
        return null;
      }
      const resolved = await this.resolve(cleanId, normalizedImporter, {
        skipSelf: true,
      });
      const fallback = () => {
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
          ? stripQuery(unwrapVirtual(resolved.id))
          : fallback();

      // Check if file was already marked for fallback (runtime error)
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
      if (!compiler) {
        throw new Error("Markflow compiler has not been initialized");
      }
      const filename =
        sourceLookup.get(id) ??
        stripQuery(id.slice(VIRTUAL_PREFIX.length).replace(/\.markflow\.jsx$/, ""));
      try {
        const source = await readFile(filename, "utf8");

        // Early detection of problematic patterns - skip to fallback
        if (hasProblematicMdxPatterns(source)) {
          this.warn(`[markflow] Skipping ${filename}: contains patterns incompatible with markdown-rs`);
          fallbackFiles.add(filename);
          fallbackReasons.set(filename, 'Detected problematic MDX patterns');
          return createFallbackModule(filename);
        }

        const startTime = performance.now();
        let result;
        let frontmatter = {};
        let headings = [];

        // Check cache first (populated in build mode by buildStart)
        const cached = compilationCache.get(filename);
        // Skip cache for files with user imports or JSX components that need runtime rendering
        // The batch compiler outputs JSX components (like <Aside {...}>) that can't be used with set:html
        // Also skip cache for all .mdx files since they typically have imports/components
        const isMdx = filename.endsWith('.mdx');
        const hasUserImports = cached?.hoistedImports?.length > 0;
        const hasUserDefaultExport = cached?.hasUserDefaultExport === true;
        // Detect JSX patterns: spread syntax and uppercase components
        const hasJsxComponents = cached?.html && /\{\.\.\.|\<[A-Z]/.test(cached.html);
        if (cached && !hasUserImports && !hasUserDefaultExport && !hasJsxComponents && !isMdx) {
          // compileBatch returns CompileIrResult with 'html' field (raw HTML)
          // We need to wrap it in a JSX module structure
          if (cached.frontmatterJson) {
            try {
              frontmatter = JSON.parse(cached.frontmatterJson);
            } catch {
              frontmatter = {};
            }
          }
          headings = cached.headings || [];

          // Wrap HTML in JSX module (batch compilation produces self-contained HTML)
          const jsxCode = wrapHtmlInJsxModule(cached.html, frontmatter, headings);
          result = {
            code: jsxCode,
            map: null,
            frontmatter_json: cached.frontmatterJson,
            headings,
            imports: [],
          };
        } else if (IS_MDAST) {
          // Use parseBlocks() for mdast pipeline
          const binding = await loadMarkflowBinding();
          const parseResult = binding.parseBlocks(source, {
            enable_directives: true,
          });
          headings = parseResult.headings;
          const frontmatterResult = binding.parseFrontmatter(source);
          frontmatter = frontmatterResult.frontmatter || {};

          result = {
            code: blocksToJsx(parseResult.blocks, frontmatter, headings, registry),
            map: null,
            frontmatter_json: JSON.stringify(frontmatter),
            headings,
            imports: [],
          };
        } else {
          // Use original compiler for multipass pipeline (dev mode or cache miss)
          const fileOptions = deriveFileOptions(filename, resolvedConfig?.root);
          result = compiler.compile(source, filename, fileOptions);
          // Extract frontmatter and headings from compiler result
          if (result.frontmatter_json) {
            try {
              frontmatter = JSON.parse(result.frontmatter_json);
            } catch {
              frontmatter = {};
            }
          }
          headings = result.headings || [];
        }
        const endTime = performance.now();
        totalProcessingTimeMs += endTime - startTime;
        processedFiles.add(filename);

        // Defensive check: ensure result.code is defined
        if (result.code == null || typeof result.code !== 'string') {
          throw new Error(`Compiler returned undefined or invalid code for ${filename}`);
        }

        // Log warnings from Rust diagnostics
        if (result.diagnostics?.warnings?.length > 0) {
          for (const warning of result.diagnostics.warnings) {
            this.warn(
              `[markflow] ${filename}:${warning.line}: ${warning.message}`,
            );
          }
        }

        // Build transform context
        /** @type {import('./types.js').TransformContext} */
        const ctx = {
          code: result.code,
          source,
          filename,
          frontmatter,
          headings,
          registry,
          config: {
            expressiveCode,
            starlightComponents,
            shiki: getShiki(),
          },
        };

        // Run transform pipeline with hooks
        const transformPipeline = pipe(
          // User hooks: afterParse
          ...hooks.afterParse,

          // Built-in: ExpressiveCode rewriting
          transformExpressiveCode,

          // User hooks: beforeInject
          ...hooks.beforeInject,

          // Built-in: Component injection (unified, registry-driven)
          transformInjectComponentsFromRegistry,

          // Built-in: Shiki highlighting
          transformShikiHighlight,

          // User hooks: beforeOutput
          ...hooks.beforeOutput,
        );

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
          loader: "jsx",
          jsx: "transform",
          jsxFactory: "_jsx",
          jsxFragment: "_Fragment",
        });
        return {
          code: esbuildResult.code,
          map: esbuildResult.map ?? result.map ?? undefined,
          meta: {
            vite: {
              jsx: true,
            },
          },
        };
      } catch (error) {
        const message = error?.message || String(error);
        const shouldFallback =
          message.includes("Vite module runner has been closed") ||
          message.includes("Markdown parser error") ||
          message.includes("Markdown parse error") ||
          message.includes("Transform failed") ||
          message.includes("Compiler returned undefined") ||
          message.includes("Cannot read properties of undefined") ||
          message.includes("Cannot read properties of null");
        if (shouldFallback) {
          fallbackFiles.add(filename);
          fallbackReasons.set(filename, message);
          this.warn(
            `[markflow] Falling back to Astro MDX for ${filename}: ${message}`,
          );

          // Invalidate the virtual module from the cache so subsequent requests
          // go through resolveId again (which will now return null for this file)
          if (resolvedConfig?.server?.moduleGraph) {
            const mod = resolvedConfig.server.moduleGraph.getModuleById(id);
            if (mod) {
              resolvedConfig.server.moduleGraph.invalidateModule(mod);
            }
          }

          return createFallbackModule(filename);
        }
        throw new Error(`[markflow] Compile failed for ${filename}: ${message}`);
      }
    },
    async buildEnd() {
      if (process.env.MARKFLOW_STATS !== "1") return;

      const totalFiles = processedFiles.size + fallbackFiles.size;

      const stats = {
        timestamp: new Date().toISOString(),
        totalFiles,
        processedByMarkflow: processedFiles.size,
        handledByAstro: fallbackFiles.size,
        handledByAstroRate:
          totalFiles > 0
            ? `${((fallbackFiles.size / totalFiles) * 100).toFixed(2)}%`
            : "0%",
        // Pre-validation skips removed - was causing false positives
        preValidationSkips: {
          count: 0,
          files: [],
        },
        // Runtime fallbacks: errors discovered during compilation in load()
        runtimeFallbacks: {
          count: fallbackFiles.size,
          files: Array.from(fallbackFiles).map((file) => ({
            file: file.replace(resolvedConfig?.root ?? "", ""),
            reason: fallbackReasons.get(file) ?? "unknown",
          })),
        },
        // Legacy fields for backwards compatibility
        fallbacks: fallbackFiles.size,
        fallbackRate:
          totalFiles > 0
            ? `${((fallbackFiles.size / totalFiles) * 100).toFixed(2)}%`
            : "0%",
        fallbackFiles: Array.from(fallbackFiles).map((file) => ({
          file: file.replace(resolvedConfig?.root ?? "", ""),
          reason: fallbackReasons.get(file) ?? "unknown",
        })),
        performance: {
          totalProcessingTimeMs: Math.round(totalProcessingTimeMs * 100) / 100,
          averageFileTimeMs:
            processedFiles.size > 0
              ? Math.round((totalProcessingTimeMs / processedFiles.size) * 100) / 100
              : 0,
        },
      };

      const { writeFile } = await import("node:fs/promises");
      const outputPath = path.join(resolvedConfig?.root ?? ".", "markflow-stats.json");
      await writeFile(outputPath, JSON.stringify(stats, null, 2));
      console.info(`[markflow] Stats written to ${outputPath}`);
    },
  };
}

/**
 * Wrap raw HTML from batch compilation in a JSX module structure.
 * This creates the same output format as the single-file compiler.
 * Only used for files without user imports (pure HTML content).
 */
function wrapHtmlInJsxModule(html, frontmatter, headings) {
  const frontmatterJson = JSON.stringify(frontmatter);
  const headingsJson = JSON.stringify(headings);

  // Wrap HTML in a Fragment (using set:html for raw HTML)
  // The HTML from batch compilation is already rendered and self-contained
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

/**
 * Detect MDX patterns that markdown-rs cannot parse correctly.
 * These files should fall back to Astro MDX immediately.
 */
function hasProblematicMdxPatterns(source) {
  // Pattern 1: Code fences inside JSX blocks (TabItem, Fragment with slot)
  // The ``` inside JSX confuses the parser
  const hasCodeFenceInJsx = /<(TabItem|Fragment[^>]*slot=)[^>]*>[\s\S]*?```[\s\S]*?<\//.test(source);

  // Pattern 2: Nested interactive components (MultipleChoice > Box)
  // markdown-rs struggles with nested JSX containing markdown
  const hasNestedInteractive = /<MultipleChoice[\s\S]*?<Box/.test(source);

  // Pattern 3: Steps/FileTree with complex nesting
  const hasComplexSteps = /<Steps[\s\S]*?<details[\s\S]*?<\/Steps>/.test(source);

  return hasCodeFenceInJsx || hasNestedInteractive || hasComplexSteps;
}

function createFallbackModule(filename) {
  return {
    code: `export { default } from ${JSON.stringify(
      filename,
    )};\nexport * from ${JSON.stringify(filename)};`,
  };
}

function createShikiHighlighter() {
  const theme = createCssVariablesTheme({
    name: "astro-code",
    variablePrefix: "--astro-code-",
  });
  const cache = new Map();
  return async (code, lang) => {
    const key = `${lang || "text"}`;
    let cached = cache.get(key);
    if (!cached) {
      cached = { lang: lang || "text" };
      cache.set(key, cached);
    }
    const html = await codeToHtml(code, {
      lang: cached.lang,
      theme,
    });
    return html.replace(/<pre class="([^"]*)"/, (match, classes) => {
      const normalized = classes
        .split(/\s+/)
        .filter((value) => value && value !== "shiki")
        .join(" ");
      const next = normalized ? `astro-code ${normalized}` : "astro-code";
      return `<pre class="${next}"`;
    });
  };
}

async function rewriteAstroSetHtml(code, highlight) {
  const marker = "<Fragment set:html={";
  const idx = code.indexOf(marker);
  if (idx === -1) return code;
  const start = idx + marker.length;
  const end = code.indexOf("} />", start);
  if (end === -1) return code;

  let literal = code.slice(start, end).trim();
  if (!literal) return code;

  let html;
  try {
    html = JSON.parse(literal);
  } catch {
    return code;
  }

  const rewritten = await highlightHtmlBlocks(html, highlight);
  const encoded = JSON.stringify(rewritten);
  return `${code.slice(0, start)}${encoded}${code.slice(end)}`;
}

async function highlightHtmlBlocks(html, highlight) {
  const fragment = parseFragment(html);
  const tasks = [];

  walk(fragment, (node) => {
    if (node.nodeName !== "pre") return;
    const codeNode = (node.childNodes || []).find((child) => child.nodeName === "code");
    if (!codeNode) return;

    const codeText = getText(codeNode).trimEnd();
    if (!codeText) return;
    const classAttr = getAttr(codeNode, "class") || "";
    const lang = classAttr
      .split(/\s+/)
      .find((value) => value.startsWith("language-"))
      ?.slice("language-".length);

    tasks.push(
      highlight(codeText, lang).then((shikiHtml) => {
        const highlighted = parseFragment(shikiHtml);
        const pre = (highlighted.childNodes || []).find((child) => child.nodeName === "pre");
        if (pre) {
          node.nodeName = pre.nodeName;
          node.tagName = pre.tagName;
          node.attrs = pre.attrs;
          node.childNodes = pre.childNodes;
        }
      }),
    );
  });

  if (tasks.length > 0) {
    await Promise.all(tasks);
  }
  return serialize(fragment);
}

function walk(node, visit) {
  visit(node);
  if (!node.childNodes) return;
  for (const child of node.childNodes) {
    walk(child, visit);
  }
}

function getAttr(node, name) {
  const attrs = node.attrs || [];
  const found = attrs.find((attr) => attr.name === name);
  return found ? found.value : null;
}

function getText(node) {
  if (!node.childNodes) return "";
  let text = "";
  for (const child of node.childNodes) {
    if (child.nodeName === "#text") {
      text += child.value || "";
    } else {
      text += getText(child);
    }
  }
  return text;
}
