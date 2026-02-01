#!/usr/bin/env node
/**
 * Pipeline stage benchmark for Markflow.
 *
 * Measures each stage of the **production** pipeline across all .mdx files
 * from the withastro-docs fixture.
 *
 * The production path is: compileBatch → wrapHtmlInJsxModule → transforms → esbuild.
 *
 * Usage:
 *   bun scripts/bench-pipeline.mjs [--en-only] [--warmup=N]
 *
 * Requires:
 *   - N-API binding built: pnpm --dir crates/napi run build:napi
 *   - astro-markflow built: pnpm --dir packages/astro-markflow run build
 *     (not strictly needed since bun handles TS imports)
 */

import { performance } from 'node:perf_hooks';
import { readFileSync, statSync } from 'node:fs';
import { resolve, dirname, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { glob } from 'node:fs/promises';

// N-API binding (JS, works in both node and bun)
import { compileBatch, parseFrontmatter } from '../crates/napi/index.js';

// astro-markflow transforms (TS — requires bun or tsx)
import {
  transformExpressiveCode,
  transformShikiHighlight,
  transformInjectComponentsFromRegistry,
} from '../packages/astro-markflow/src/transforms/index.ts';
import { normalizeSteps } from '../packages/astro-markflow/src/transforms/normalize-steps.ts';
import { wrapHtmlInJsxModule } from '../packages/astro-markflow/src/vite-plugin/jsx-module.ts';
import { createShikiHighlighter } from '../packages/astro-markflow/src/vite-plugin/shiki-highlighter.ts';
import { createRegistry, starlightLibrary, expressiveCodeLibrary, astroLibrary } from 'markflow/registry';

// esbuild
import esbuild from 'esbuild';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '..');
const DOCS_DIR = resolve(ROOT, 'fixtures/integration/withastro-docs/repo/src/content/docs');

// ---------------------------------------------------------------------------
// CLI args
// ---------------------------------------------------------------------------
const args = process.argv.slice(2);
const enOnly = args.includes('--en-only');
const warmupCount = (() => {
  const flag = args.find((a) => a.startsWith('--warmup='));
  return flag ? parseInt(flag.split('=')[1], 10) : 3;
})();

// ---------------------------------------------------------------------------
// Collect files
// ---------------------------------------------------------------------------
async function collectFiles() {
  const pattern = enOnly ? 'en/**/*.mdx' : '**/*.mdx';
  const files = [];
  for await (const entry of glob(pattern, { cwd: DOCS_DIR })) {
    files.push(resolve(DOCS_DIR, entry));
  }
  files.sort();
  return files;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
async function main() {
  const files = await collectFiles();
  if (files.length === 0) {
    console.error('No .mdx files found in', DOCS_DIR);
    process.exit(1);
  }

  // Read all files
  const sources = files.map((f) => ({
    filepath: f,
    source: readFileSync(f, 'utf8'),
  }));
  const totalBytes = sources.reduce((sum, s) => sum + Buffer.byteLength(s.source), 0);

  console.log(`\n=== Pipeline Stage Benchmark (Production Path) ===`);
  console.log(`Files: ${files.length} (${(totalBytes / 1024 / 1024).toFixed(1)} MB)`);
  if (enOnly) console.log('(English docs only)');
  console.log();

  // Setup registry (Starlight preset)
  const registry = createRegistry([starlightLibrary, expressiveCodeLibrary, astroLibrary]);
  const expressiveCode = { component: 'Code', importSource: 'astro-expressive-code/components', enabled: true };

  // Initialize shiki (warm up)
  console.log('Initializing Shiki...');
  const shiki = await createShikiHighlighter();
  // Warmup shiki with a few calls
  for (let i = 0; i < warmupCount; i++) {
    await shiki('const x = 1;', 'js');
  }
  console.log('Shiki ready.\n');

  // ---------------------------------------------------------------------------
  // Stage 1: Rust compileBatch (md → HTML string)
  // ---------------------------------------------------------------------------
  const inputs = sources.map(({ filepath, source }) => ({
    id: filepath,
    source,
    filepath,
  }));

  const rustStart = performance.now();
  const batchResult = compileBatch(inputs, {
    continueOnError: true,
    config: { enable_directives: true },
  });
  const rustMs = performance.now() - rustStart;

  // Collect successful results
  const compiled = [];
  for (const r of batchResult.results) {
    if (!r.result) continue;
    const fm = r.result.frontmatterJson ? JSON.parse(r.result.frontmatterJson) : {};
    compiled.push({
      filepath: r.id,
      source: sources.find((s) => s.filepath === r.id)?.source ?? '',
      html: r.result.html,
      frontmatter: fm,
      headings: r.result.headings || [],
      hoistedExports: r.result.hoistedExports,
      hasUserDefaultExport: r.result.hasUserDefaultExport,
    });
  }

  // ---------------------------------------------------------------------------
  // Stage 2: wrapHtmlInJsxModule
  // ---------------------------------------------------------------------------
  const wrapStart = performance.now();
  const jsxResults = compiled.map((r) => {
    const code = wrapHtmlInJsxModule(r.html, r.frontmatter, r.headings, r.filepath, {
      hoistedExports: r.hoistedExports,
      hasUserDefaultExport: r.hasUserDefaultExport,
    });
    return { ...r, code };
  });
  const wrapMs = performance.now() - wrapStart;

  // ---------------------------------------------------------------------------
  // Stage 3a: ExpressiveCode rewriting
  // ---------------------------------------------------------------------------
  const ecStart = performance.now();
  const ecResults = jsxResults.map((r) => {
    const ctx = {
      code: r.code,
      source: r.source,
      filename: r.filepath,
      frontmatter: r.frontmatter,
      headings: r.headings,
      registry,
      config: {
        expressiveCode,
        starlightComponents: false,
        shiki: null,
      },
    };
    return { ...r, ctx: transformExpressiveCode(ctx) };
  });
  const ecMs = performance.now() - ecStart;

  // ---------------------------------------------------------------------------
  // Stage 3b: Component injection
  // ---------------------------------------------------------------------------
  const ciStart = performance.now();
  const ciResults = ecResults.map((r) => {
    return { ...r, ctx: transformInjectComponentsFromRegistry(r.ctx) };
  });
  const ciMs = performance.now() - ciStart;

  // ---------------------------------------------------------------------------
  // Stage 3c: Shiki highlighting
  // ---------------------------------------------------------------------------
  const shikiStart = performance.now();
  const shikiResults = [];
  for (const r of ciResults) {
    const ctx = { ...r.ctx, config: { ...r.ctx.config, shiki } };
    const result = await transformShikiHighlight(ctx);
    shikiResults.push({ ...r, ctx: result });
  }
  const shikiMs = performance.now() - shikiStart;

  // ---------------------------------------------------------------------------
  // Stage 3d: normalizeSteps
  // ---------------------------------------------------------------------------
  const nsStart = performance.now();
  const nsResults = shikiResults.map((r) => {
    if (!r.ctx.code) return r;
    const { code, changed } = normalizeSteps(r.ctx.code);
    if (changed) {
      return { ...r, ctx: { ...r.ctx, code } };
    }
    return r;
  });
  const nsMs = performance.now() - nsStart;

  // ---------------------------------------------------------------------------
  // Stage 4: esbuild JSX → JS
  // ---------------------------------------------------------------------------
  const esStart = performance.now();
  let esbuildErrors = 0;
  const esbuildResults = await Promise.all(
    nsResults.map((r) =>
      esbuild.transform(r.ctx.code, {
        loader: 'jsx',
        jsx: 'transform',
        jsxFactory: '_jsx',
        jsxFragment: '_Fragment',
        sourcefile: r.filepath,
      }).catch(() => {
        esbuildErrors++;
        return null;
      })
    )
  );
  const esMs = performance.now() - esStart;
  void esbuildResults;

  // ---------------------------------------------------------------------------
  // Results
  // ---------------------------------------------------------------------------
  const totalMs = rustMs + wrapMs + ecMs + ciMs + shikiMs + nsMs + esMs;
  const n = compiled.length;

  const stages = [
    ['Rust compileBatch', rustMs],
    ['wrapHtmlInJsxModule', wrapMs],
    ['ExpressiveCode', ecMs],
    ['Component injection', ciMs],
    ['Shiki highlight', shikiMs],
    ['normalizeSteps', nsMs],
    ['esbuild JSX→JS', esMs],
  ];

  const header = 'Stage                      Total (ms)   Avg (ms/file)   % of total';
  const sep = '─'.repeat(header.length);

  console.log(header);
  console.log(sep);

  for (const [name, ms] of stages) {
    const pct = ((ms / totalMs) * 100).toFixed(0);
    console.log(
      `${String(name).padEnd(27)}${ms.toFixed(0).padStart(9)}   ${(ms / n).toFixed(2).padStart(13)}   ${pct.padStart(8)}%`
    );
  }

  console.log(sep);
  console.log(
    `${'Total pipeline'.padEnd(27)}${totalMs.toFixed(0).padStart(9)}   ${(totalMs / n).toFixed(2).padStart(13)}   ${'100'.padStart(8)}%`
  );
  if (esbuildErrors > 0) {
    console.log(`\n(${esbuildErrors} files had esbuild errors — skipped in JSX→JS stage)`);
  }
  console.log(`(${batchResult.stats.total - batchResult.stats.succeeded} files failed compilation — skipped)`);
  console.log();
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
