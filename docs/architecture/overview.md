# Architecture Overview

> This document provides a high-level overview of Markflow's architecture.

## System Architecture

Markflow is a streaming Markdown/MDX engine built with Rust, designed for high-performance processing in web frameworks.

### Core Components

1. **Core Parse Layer** (`crates/core`)
   - Markdown-rs based parser entrypoints
   - Frontmatter extraction and slug utilities
   - Parse pipeline hooks (text/AST transforms)

2. **Astro Engine** (`crates/astro`)
   - MDAST renderer and block model
   - Directive/components registry and transforms
   - Astro-focused code generation helpers

3. **NAPI Bindings** (`crates/napi`)
   - Node.js interface via NAPI-RS

4. **WASM Bindings** (`crates/wasm`)
   - WebAssembly interface
   - Browser and edge runtime support

## Data Flow (Astro path)

```
Markdown Input
     ↓
  Parser (markdown-rs → MDAST)
     ↓
  Astro Engine (render blocks)
     ↓
  HTML Output

## Core Split (2026-02)

The parsing surface lives in `crates/core`, while Astro/Starlight-specific rendering and
rewrites live in `crates/astro`. Bindings depend on both to keep the core reusable and
the framework concerns isolated.

## SSG / Route Improvements (Notes)

These are design notes to improve SSG throughput and route stability without changing
the public API surface.

- **Observed bottleneck (non-Vite SSG)**:
  - SSG route generation accounts for ~96% of non-Vite time.
  - `/docs` accounts for ~98% of route generation time.
- **Impact-first strategy**: prioritize changes that reduce `/docs` route generation
  work before optimizing general compilation paths. Cache-based optimizations are
  intentionally out of scope for now.
- **Stable route IDs**: normalize paths once (posix, `index` fallback) and reuse the
  same normalization in loader + integration to avoid duplicate routes across OSes.
- **Single filesystem scan**: avoid double globbing; use existing store entries as the
  source of truth and only touch files that changed.
- **SSG memory bound**: cap parallel file reads/transforms to prevent spikes when a
  large collection is loaded.
- **Deterministic output**: ensure block rendering and imports are ordered deterministically
  so CI diffs remain stable.
- **Rustization candidates (optional)**:
  - Move `/docs` scan + frontmatter parse into Rust to reduce JS I/O
    overhead, keep Astro `parseData` and `store.set` in JS.

### SSG Action Plan (Concrete)

1. **Measure first**
   - Capture per-phase timings for `/docs` route generation (scan, read, frontmatter, schema parse).
   - Keep a top-N slowest files list to identify outliers.

2. **Single scan**
   - Replace double globbing with a single file list derived from `store.entries()`.

3. **Bounded parallelism**
   - Limit concurrent file reads/parses to avoid memory/IO spikes in large collections.

4. **Rust helper (optional)**
   - Provide a small Rust helper that returns ordered file lists + frontmatter payloads
     for a root + pattern. Keep Astro schema parsing in JS.

### Vite Performance Notes (Suspects)

- **Heavy transforms**: Shiki + ExpressiveCode are the largest CPU cost in the Vite pipeline.
- **HTML re-parsing**: parse5 serialize/parse in `highlightHtmlBlocks` adds overhead, especially for large docs.
- **Fallback path**: falling back to `@mdx-js/mdx` doubles work (detect, then recompile).
- **Repeated work**: preprocessing + compile + esbuild for each file.
- **Binding load**: native binding resolution and warmup can add startup cost.

### Why Vite Can Be Slower Than astro/mdx (Hypotheses)

- **Extra pipeline stages**: Markflow runs additional transforms (registry injection, ExpressiveCode, Shiki)
  on top of the MDX compilation that astro/mdx already does.
- **JS ↔ native bridge**: NAPI binding load + JSON serialization for blocks/headings adds overhead that
  the pure JS MDX pipeline doesn’t pay.
- **HTML AST roundtrip**: parse5-based highlight passes parse+serialize large HTML strings, which is
  costlier than mdx/rehype’s in-memory HAST path.
- **Fallback duplication**: problematic MDX patterns trigger fallback to `@mdx-js/mdx`, effectively
  running two compilation paths for the same file.
- **Dev server costs**: Vite adds module graph updates, HMR bookkeeping, and sourcemap generation;
  heavy per-file transforms amplify this overhead.

### Vite/MDX Measurement Plan (Memo)

Capture per-file timings and counters to separate **transform cost** from **Vite overhead**:

- **Timing buckets**: preprocess, native compile, fallback compile, esbuild, pipeline transforms
  (ExpressiveCode, Shiki, registry injection), parse5 roundtrip.
- **Counters**: fallback rate, number of code blocks,
  JSX blocks, raw HTML blocks, and component injections.
- **Warmup costs**: native binding load time, Shiki initialization time.
- **Build phases**: first load vs HMR re-run, and build vs dev server.

Use existing env flags (`MARKFLOW_DEBUG_TIMING`, `MARKFLOW_LOAD_PROFILE`) and extend with a
per-phase histogram if needed.

### Impact-First Optimization Ideas (Memo)

Ranked by expected impact; prioritize `/docs` and heavy transform paths.

**Very High**
- **Avoid fallback duplication**: detect problematic MDX before invoking NAPI; if fallback is needed,
  skip the native compile entirely (single-path compile).
- **Shiki/ExpressiveCode fast-mode**: disable or defer in dev by default; enable on demand or on build.
- **Remove parse5 roundtrip**: highlight code blocks at the block/AST level instead of HTML parse+serialize.
- **Single scan for `/docs`**: eliminate double globbing and redundant I/O.

**High**
- **Block-level code highlighting**: run Shiki on RenderBlock::Code (or equivalent JS blocks),
  then render once; avoid HTML re-parsing.
- **Bounded parallelism**: cap concurrent file transforms; prevents IO/memory spikes and tail latency.
- **Batch compile on build**: use a single native batch call to reduce per-file NAPI overhead.

**Medium**
- **Reduce JSON bridging**: return structured blocks/headings without extra stringify/parse steps.
- **Defer component injection**: skip registry injection when no matching components exist.
- **Simplify preprocess**: short-circuit early if no JSX/directives/code fences detected.

**Low/Speculative**
- **Rust helper for scan**: move filesystem scan + frontmatter into Rust.

### Vite Hot Path: Concrete Plan (Impact-First)

1. **Single-path compile (no duplicate fallback)**
   - Run MDX-problem detection up front.
   - If fallback is needed, **skip native compile** entirely and go straight to `@mdx-js/mdx`.

2. **Block-level highlighting (remove parse5)**
   - Highlight `RenderBlock::Code` before converting to HTML/JSX.
   - Eliminate parse5 parse/serialize for large HTML strings.

3. **Feature gating**
   - If no code fences, skip Shiki/ExpressiveCode entirely.
   - In dev, default to “fast mode” (no Shiki/ExpressiveCode) unless explicitly enabled.

4. **Batch compile in build**
   - Use a single native batch call for build to reduce per-file NAPI overhead.
   - Emit per-file results with shared warmup.

### Parse5 Removal: Block-Level Shiki (Design Sketch)

**Goal**: remove `parse5` HTML roundtrips in `highlightHtmlBlocks` / `highlightJsxCodeBlocks`
by highlighting code blocks *before* they become large HTML strings.

**Current flow (expensive)**  
`blocks → blocksToJsx → HTML/JSX → parse5 → serialize → output`

**Proposed flow (cheaper)**  
`blocks → highlightBlocks(shiki) → blocksToJsx → output`

**Key idea**
- Traverse `RenderBlock::Code` *recursively* (including slot children).
- If `ExpressiveCode` is enabled, **skip Shiki entirely** (single-path).
- If Shiki is enabled and ExpressiveCode is disabled:
  - Convert code blocks to **pre-highlighted HTML** once:
    - `shiki(code, lang) → "<pre class=...>…</pre>"`
  - Replace the block with `RenderBlock::Html { content: shikiHtml }`
    (or attach `highlightedHtml` to keep metadata).

**Why this removes parse5**
- No need to parse/serialize HTML because highlighting happens at the block level
  before any HTML/JSX concatenation.

**Edge cases**
- Nested code blocks inside component slots: handled by recursive traversal.
- Empty code blocks: skip highlight to avoid churn.
- CJK / entity handling: `set:html` already bypasses entity mangling.

**Suggested config switch**
- `shiki.mode = "blocks" | "html"` (default `"blocks"` once validated).

**Expected impact**
- Removes the heaviest string→AST→string cost in large docs.

#### Block-Level Highlighting (Concrete)

**New transform**: `highlightBlocks(blocks, shiki, options?) -> blocks`

- **Input**: `RenderBlock[]` (from Rust), `shiki(code, lang) -> html`
- **Output**: `RenderBlock[]` with code blocks replaced by HTML blocks.
- **Rule**:
  - If ExpressiveCode is enabled: **skip** highlightBlocks entirely.
  - If Shiki disabled: **skip**.
  - If no code blocks: **skip**.

**Algorithm (recursive)**:
1. Walk blocks depth-first.
2. For each `RenderBlock::Code { code, lang, meta }`:
   - If `code` empty → keep as-is.
   - Else call `shiki(code, lang)` to get `<pre class="shiki">…</pre>`.
   - Replace with `RenderBlock::Html { content: shikiHtml }`.
3. For `RenderBlock::Component`, recurse into `slot_children`.

**Why this works**
- `blocksToJsx()` already renders `Html` blocks via `set:html`.
- Shiki output is plain HTML, so it is safe to embed without parse5.
- Slot content is handled by the same recursion, so nested code blocks are covered.

**Changes to pipeline (conceptual)**
```
parse -> blocks -> highlightBlocks -> blocksToJsx -> (expressiveCode? then only if enabled)
```

**Notes**
- ExpressiveCode and Shiki should be **mutually exclusive** in the pipeline.
- This avoids both `highlightHtmlBlocks()` and `highlightJsxCodeBlocks()`.

#### ExpressiveCode vs Shiki (Priority Rules)

1. **ExpressiveCode wins** when enabled.
   - Shiki is **skipped entirely**.
2. **Shiki runs** only when ExpressiveCode is disabled.
3. **Neither runs** if no code blocks are present.
4. **Dev default**: keep ExpressiveCode off unless explicitly enabled (fast path).

#### blocksToJsx Rules (Shiki-first Assumption)

- `RenderBlock::Html` is always emitted via `set:html`.
- `RenderBlock::Code` should be **absent** after `highlightBlocks` when Shiki is active.
- Slot content must remain HTML-safe; `set:html` is preferred unless nested components exist.
- Fragment slot handling remains unchanged (slot content rendered directly).

#### Implementation Plan (Minimal Changes)

1. **Add `highlightBlocks` transform**
   - New JS utility that maps `RenderBlock[] → RenderBlock[]`.
2. **Wire into pipeline**
   - Insert before `blocksToJsx()`, skip if ExpressiveCode enabled.
3. **Disable parse5 path**
   - Gate `highlightHtmlBlocks` / `highlightJsxCodeBlocks` behind a feature flag, then remove.
4. **Tests**
   - Unit tests for recursive `Code → Html` replacement.
   - Ensure nested slot code blocks are highlighted.

### Packages Responsibility Split (Target)

This is a **design split** to keep packages small and responsibilities clear.

**packages/markflow**
- User-facing API (Node/WASM) for `compile()` and types.
- No Astro-specific transforms or loader logic.

**packages/astro-loader**
- Content Layer loader only: file discovery + frontmatter extraction + store population.
- No Markdown compilation or rendering.

**packages/astro-markflow-core (proposed)**
- Vite integration (compile + minimal blocks→JSX).
- Calls native bindings and returns JSX module code.
- No Shiki/ExpressiveCode/registry presets bundled.

**packages/astro-markflow-pipeline (proposed)**
- Heavy transforms only (Shiki, ExpressiveCode, registry presets, component injection).
- Optional add-on; loaded only when enabled.

**packages/astro-markflow (full)**
- Convenience wrapper that composes `core + pipeline`.
- Provides the current “batteries-included” experience.

**Boundary rules**
- `core` must not import from `pipeline`.
- `pipeline` must be optional and safe to omit.
- `astro-loader` must not depend on Vite or transforms.

### Pipeline API (Draft)

Vite-like, but minimal and composable. Designed for **function-first** transforms.

```ts
// packages/astro-markflow-core
export interface MarkflowCoreOptions {
  pipeline?: MarkflowPipeline;
  binding?: MarkflowBinding;
  include?: (id: string) => boolean;
  mdx?: MdxImportHandlingOptions;
  compiler?: CompilerOptions;
}

export default function markflowCore(options?: MarkflowCoreOptions): AstroIntegration;

// packages/astro-markflow-pipeline
export type TransformContext = {
  code: string;
  source: string;
  filename: string;
  frontmatter: Record<string, unknown>;
  headings: HeadingEntry[];
  registry?: Registry;
  config: {
    expressiveCode: ExpressiveCodeConfig | null;
    shiki: ShikiHighlighter | null;
    starlightComponents: boolean;
  };
};

export type Transform = (ctx: TransformContext) => TransformContext | Promise<TransformContext>;

export interface MarkflowTransform {
  name: string;
  enforce?: 'pre' | 'post';
  apply?: 'build' | 'serve' | 'both';
  transform: Transform;
}

export interface MarkflowPipeline {
  use(transform: Transform | MarkflowTransform): MarkflowPipeline;
  run(ctx: TransformContext): Promise<TransformContext>;
}

export function createPipeline(): MarkflowPipeline;

// Built-ins (pipeline package)
export function shiki(options?: ShikiOptions): MarkflowTransform;
export function expressiveCode(options?: ExpressiveCodeOptions): MarkflowTransform;
export function registryInjection(options?: RegistryOptions): MarkflowTransform;
```

**Usage sketch**
```ts
import markflowCore from 'astro-markflow-core';
import { createPipeline, shiki, expressiveCode } from 'astro-markflow-pipeline';

const pipeline = createPipeline()
  .use(shiki({ mode: 'blocks' }))
  .use(expressiveCode({ enabled: false }));

export default markflowCore({ pipeline });
```

### /docs Route Generation: Concrete Plan

1. **Single scan**
   - Eliminate double globbing; derive file list from store entries.

2. **Bounded concurrency**
   - Cap concurrent reads/parses to prevent IO/memory spikes on large `/docs`.

3. **Optional Rust helper**
   - Provide a small Rust scanner:
     - Inputs: root, pattern
     - Outputs: ordered file list + frontmatter payloads
   - Keep Astro schema parsing in JS (no caching).
```

## Design Principles

- **Streaming-first**: Process content without full AST materialization
- **Memory efficiency**: Direct streaming without intermediate buffers
- **Performance**: Target sub-100ms processing for 10MB files
- **Extensibility**: Plugin-based rewriter hooks

## Astro Loader Contract

- The custom `@markflow/astro-loader` keeps raw Markdown bodies in the store so the Vite plugin can compile JSX/MDX, while slug IDs follow the normalized `dir/name` pattern (forward slashes, `index` fallback) to stay stable across OSes.
- Pass `throwOnFrontmatterError: true` when defining a collection if frontmatter validation should fail the build; by default the loader logs the Rust parser errors but continues so authors can see partially processed content.

## References

- Implementation roadmap was consolidated into the decision log; see `docs/decisions/0001-lean-architecture.md` for current status and history.
- Development guidelines: [guidelines.md](../development/guidelines.md)
