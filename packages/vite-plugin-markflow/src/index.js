import { readFile } from "node:fs/promises";
import path from "node:path";
import { transformWithEsbuild } from "vite";
import { parseFragment, serialize } from "parse5";
import { codeToHtml, createCssVariablesTheme } from "shiki";
import { createRegistry, starlightLibrary, astroLibrary } from "markflow/registry";
import { pipe, when } from "./pipeline/pipe.js";
import {
  transformExpressiveCode,
  transformInjectAstroComponents,
  transformInjectStarlightComponents,
  transformShikiHighlight,
} from "./transforms/index.js";

const DEFAULT_EXTENSIONS = new Set([".md", ".mdx"]);

// Create default registry with Starlight and Astro components
const defaultRegistry = createRegistry([starlightLibrary, astroLibrary]);

// Derive component lists from registry for backward compatibility
const STARLIGHT_COMPONENTS_MODULE = "@astrojs/starlight/components";
const ASTRO_COMPONENTS_MODULE = "astro/components";
const EXPRESSIVE_CODE_COMPONENT = "ExpressiveCode";
const EXPRESSIVE_CODE_MODULE = "astro-expressive-code/components";
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
  const fallbackFiles = new Set();
  const fallbackReasons = new Map(); // file -> reason
  const processedFiles = new Set();
  const compilationCache = new Map(); // filename -> batch compiled result
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

      console.info(
        `[markflow] Batch compiled ${batchResult.stats.succeeded}/${batchResult.stats.total} files in ${batchResult.stats.processingTimeMs.toFixed(0)}ms`
      );
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
        const fileOptions = deriveFileOptions(filename, resolvedConfig?.root);

        const startTime = performance.now();
        let result;
        let frontmatter = {};
        let headings = [];

        // Check cache first (populated in build mode by buildStart)
        const cached = compilationCache.get(filename);
        if (cached) {
          // compileBatch returns CompileIrResult with 'html' field (raw HTML)
          // We need to wrap it in a JSX module structure
          if (cached.frontmatter_json) {
            try {
              frontmatter = JSON.parse(cached.frontmatter_json);
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
            frontmatter_json: cached.frontmatter_json,
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
            code: blocksToJsx(parseResult.blocks, frontmatter, headings),
            map: null,
            frontmatter_json: JSON.stringify(frontmatter),
            headings,
            imports: [],
          };
        } else {
          // Use original compiler for multipass pipeline (dev mode or cache miss)
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

          // Built-in: Component injection
          transformInjectAstroComponents,
          transformInjectStarlightComponents,

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
          message.includes("Transform failed");
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

function injectStarlightComponents(code, config) {
  const resolved = resolveStarlightConfig(config);
  if (!resolved) return code;

  return injectComponentImports(code, resolved.components, resolved.moduleId);
}

function injectAstroComponents(code) {
  // Get Astro components from registry
  const astroComponents = defaultRegistry.getAllComponents()
    .filter(c => c.modulePath === ASTRO_COMPONENTS_MODULE)
    .map(c => c.name);
  return injectComponentImports(code, astroComponents, ASTRO_COMPONENTS_MODULE);
}

function injectExpressiveCodeComponent(code, config) {
  const importName = config.component;
  const imported = collectImportedNames(code);
  if (imported.has(importName)) {
    return code;
  }
  const importLine =
    importName === "Code"
      ? `import { Code } from '${config.moduleId}';`
      : `import { Code as ${importName} } from '${config.moduleId}';`;
  return insertAfterImports(code, importLine);
}

function injectComponentImports(code, components, moduleId) {
  const scanTarget = stripHeadingsMeta(code);
  const used = components.filter((name) =>
    new RegExp(`<${name}\\b`).test(scanTarget),
  );
  if (used.length === 0) return code;

  const imported = collectImportedNames(code);
  const missing = used.filter((name) => !imported.has(name));
  if (missing.length === 0) return code;

  const importLine = `import { ${missing.join(", ")} } from '${moduleId}';`;
  return insertAfterImports(code, importLine);
}

function stripHeadingsMeta(code) {
  return code
    .replace(/export const headings\s*=\s*\[[\s\S]*?\];\r?\n/g, "")
    .replace(/export function getHeadings\(\)\s*\{[\s\S]*?\}\r?\n/g, "");
}

/**
 * Wrap raw HTML from batch compilation in a JSX module structure.
 * This creates the same output format as the single-file compiler.
 *
 * Note: We don't extract imports from the source because batch compilation
 * produces self-contained HTML where all components are already rendered.
 * Asset imports (like CSS with ?raw) are not supported in batch mode.
 */
function wrapHtmlInJsxModule(html, frontmatter, headings) {
  const frontmatterJson = JSON.stringify(frontmatter);
  const headingsJson = JSON.stringify(headings);

  // Wrap HTML in a Fragment (using set:html for raw HTML)
  return `export const frontmatter = ${frontmatterJson};
export function getHeadings() { return ${headingsJson}; }
export default function MarkflowContent() {
  return (
    <Fragment set:html={${JSON.stringify(html)}} />
  );
}
`;
}

function blocksToJsx(blocks, frontmatter = {}, headings = [], registry = defaultRegistry) {
  const fragments = [];
  const componentImports = new Map(); // component name -> module path

  // Get supported directives from registry
  const supportedDirectives = registry.getSupportedDirectives();

  for (const block of blocks) {
    if (block.type === "html") {
      fragments.push(block.content);
    } else if (block.type === "component") {
      // Handle directive components using registry
      const isDirective = supportedDirectives.includes(block.name);
      let componentName = block.name;
      let effectiveProps = block.props;

      if (isDirective) {
        const mapping = registry.getDirectiveMapping(block.name);
        if (mapping) {
          componentName = mapping.component;
          // Apply injected props from mapping
          if (mapping.injectProps) {
            const injectedProps = {};
            for (const [propKey, propSource] of Object.entries(mapping.injectProps)) {
              if (propSource.source === 'directive_name') {
                injectedProps[propKey] = { type: "literal", value: block.name };
              } else if (propSource.source === 'literal' && propSource.value) {
                injectedProps[propKey] = { type: "literal", value: propSource.value };
              }
            }
            effectiveProps = { ...block.props, ...injectedProps };
          }
        }
      }

      // Skip Fragment - it's a built-in Astro component
      if (componentName !== "Fragment") {
        const componentDef = registry.getComponent(componentName);
        const modulePath = componentDef?.modulePath ?? '@astrojs/starlight/components';
        componentImports.set(componentName, modulePath);
      }

      const propsStr = effectiveProps
        ? Object.entries(effectiveProps)
            .map(([key, value]) => {
              // Handle PropValue enum from Rust: { type: "literal"|"expression", value: string }
              if (typeof value === "object" && value !== null && "type" in value && "value" in value) {
                if (value.type === "literal") {
                  return `${key}="${String(value.value).replace(/"/g, '\\"')}"`;
                } else if (value.type === "expression") {
                  return `${key}={${value.value}}`;
                }
              }
              if (typeof value === "string") {
                return `${key}="${value.replace(/"/g, '\\"')}"`;
              }
              return `${key}={${JSON.stringify(value)}}`;
            })
            .join(" ")
        : "";
      const openTag = propsStr ? `<${componentName} ${propsStr}>` : `<${componentName}>`;
      fragments.push(`${openTag}${block.slotHtml || ""}</${componentName}>`);
    }
  }

  // Generate imports grouped by module path
  const importsByModule = new Map();
  for (const [name, modulePath] of componentImports) {
    if (!importsByModule.has(modulePath)) {
      importsByModule.set(modulePath, []);
    }
    importsByModule.get(modulePath).push(name);
  }

  const componentImportLines = Array.from(importsByModule.entries())
    .map(([modulePath, names]) => {
      // Check if components use named exports
      const useNamed = names.every(name => {
        const def = registry.getComponent(name);
        return def?.exportType === 'named';
      });
      if (useNamed) {
        return `import { ${names.join(', ')} } from '${modulePath}';`;
      }
      // Default to individual default imports (Starlight .astro files)
      return names.map(name => `import ${name} from '${modulePath}/${name}.astro';`).join('\n');
    })
    .join("\n");

  const frontmatterJson = JSON.stringify(frontmatter);
  const headingsJson = JSON.stringify(headings);
  const jsxContent = fragments.join("\n");

  return `${componentImportLines}
export const frontmatter = ${frontmatterJson};
export function getHeadings() { return ${headingsJson}; }
export default function MarkflowContent() {
  return (
    <>
${jsxContent}
    </>
  );
}
`;
}

function resolveExpressiveCodeConfig(config) {
  if (!config) return null;
  if (config === true) {
    return {
      component: EXPRESSIVE_CODE_COMPONENT,
      moduleId: EXPRESSIVE_CODE_MODULE,
    };
  }
  if (typeof config === "object") {
    const component =
      typeof config.component === "string" && config.component.length > 0
        ? config.component
        : EXPRESSIVE_CODE_COMPONENT;
    const moduleId =
      typeof config.module === "string" && config.module.length > 0
        ? config.module
        : EXPRESSIVE_CODE_MODULE;
    return { component, moduleId };
  }
  return null;
}

function rewriteExpressiveCodeBlocks(code, componentName) {
  // Match <pre> with optional attributes (class="astro-code" tabindex="0" etc.)
  const pattern =
    /<pre[^>]*><code(?: class="language-([^"]+)")?>([\s\S]*?)<\/code><\/pre>/g;
  let changed = false;
  const next = code.replace(pattern, (match, lang, raw) => {
    changed = true;
    const decoded = decodeHtmlEntities(raw);
    const props = [`code={${JSON.stringify(decoded)}}`];
    if (lang) {
      props.push(`lang="${lang}"`);
    }
    return `<${componentName} ${props.join(" ")} />`;
  });
  return { code: next, changed };
}

function decodeHtmlEntities(value) {
  if (!value || !value.includes("&")) return value;
  return value
    .replace(/&#x([0-9a-fA-F]+);/g, (_, hex) =>
      String.fromCodePoint(Number.parseInt(hex, 16)),
    )
    .replace(/&#([0-9]+);/g, (_, num) =>
      String.fromCodePoint(Number.parseInt(num, 10)),
    )
    .replace(/&quot;/g, "\"")
    .replace(/&#39;/g, "'")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&amp;/g, "&");
}

function createFallbackModule(filename) {
  return {
    code: `export { default } from ${JSON.stringify(
      filename,
    )};\nexport * from ${JSON.stringify(filename)};`,
  };
}

function resolveStarlightConfig(config, registry = defaultRegistry) {
  if (!config) return null;

  // Get Starlight components from registry
  const starlightComponents = registry.getAllComponents()
    .filter(c => c.modulePath === '@astrojs/starlight/components')
    .map(c => c.name);
  const defaultModuleId = '@astrojs/starlight/components';

  if (config === true) {
    return {
      components: starlightComponents,
      moduleId: defaultModuleId,
    };
  }
  if (typeof config === "object") {
    const components = Array.isArray(config.components)
      ? config.components
      : starlightComponents;
    const moduleId =
      typeof config.module === "string" && config.module.length > 0
        ? config.module
        : defaultModuleId;
    return { components, moduleId };
  }
  return null;
}

function collectImportedNames(code) {
  const imported = new Set();
  const lines = code.split(/\r?\n/);
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed.startsWith("import ") || trimmed.startsWith("import(")) {
      continue;
    }

    const defaultMatch = trimmed.match(
      /^import\s+([A-Za-z$_][\w$]*)\s*(?:,|\s+from\s)/,
    );
    if (defaultMatch) {
      imported.add(defaultMatch[1]);
    }

    const namespaceMatch = trimmed.match(
      /^import\s+\*\s+as\s+([A-Za-z$_][\w$]*)\s+from/,
    );
    if (namespaceMatch) {
      imported.add(namespaceMatch[1]);
    }

    const namedMatch = trimmed.match(/import\s+{([^}]+)}\s+from/);
    if (namedMatch) {
      const parts = namedMatch[1].split(",");
      for (const part of parts) {
        const item = part.trim();
        if (!item) continue;
        const [name, alias] = item.split(/\s+as\s+/);
        imported.add((alias || name).trim());
      }
    }
  }
  return imported;
}

function insertAfterImports(code, importLine) {
  const lines = code.split(/\r?\n/);
  let idx = 0;
  while (idx < lines.length) {
    const trimmed = lines[idx].trim();
    if (!trimmed) {
      idx += 1;
      continue;
    }
    if (trimmed.startsWith("//") || trimmed.startsWith("/*")) {
      idx += 1;
      continue;
    }
    if (trimmed.startsWith("import ")) {
      idx += 1;
      continue;
    }
    break;
  }
  lines.splice(idx, 0, importLine);
  return lines.join("\n");
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
