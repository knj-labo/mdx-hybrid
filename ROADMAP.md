# Markflow Development Roadmap

## Summary
Objective: dramatically improve build performance in Astro, Next.js, and Nuxt while removing legacy JavaScript-only dependencies.
Current status: Phase 1 (Core Engine Refactoring) remains in progress.
Contributor quickstart: follow the build/test steps in [AGENTS.md](./AGENTS.md) before tackling any milestones.
Backlog relationship: day-to-day ownership now lives in [Backlog.md](./Backlog.md); this roadmap stays focused on long-horizon planning.

## In Progress
### Phase 1: Core Engine Refactoring
Goal: transition from the prototype parser to `markdown-rs` for spec compliance and streaming performance.
- [x] Core Parser Implementation
    - [x] Remove `pulldown-cmark` dependency (HTML renderer implemented internally)
    - [x] Introduce [`markdown-rs`](https://github.com/wooorm/markdown-rs) as the default parser
    - [x] Verify basic Markdown parsing functionality with the new parser
    - [x] Define internal `Event` enum (rendering bridge still converts to `pulldown-cmark` events)
    - [x] Stream `markdown-rs` events directly into the renderer (drop the `Vec<Event>` buffer)
- [x] WASM Bindings (Enhancement)
    - [x] Expand `crates/wasm` to support streaming APIs (currently minimal wrapper)

## Done
### Phase 0: Initialization
- [x] Repository Setup
    - [x] Set up repository and Cargo workspace (`core`, `napi`, `wasm`)
    - [x] Configure CI/CD pipeline (GitHub Actions)

### Phase 1: Foundation Prototypes
- [x] The Glue (PipeAdapter)
    - [x] Implement adapter connecting `Iterator<Item=Event>` to `io::Write` (`crates/core/src/adapter.rs`)
    - [x] Benchmark stream connection (`benchmarks/stream_bench.rs`)
- [x] Streaming Rewriter
    - [x] Embed `lol_html`
    - [x] Implement basic tag rewriting (lazy images) (`crates/core/src/streaming_rewriter.rs`)
- [x] NAPI Bindings
    - [x] Set up `napi-rs`
    - [x] Expose basic functions (`parse`, `parse_with_options`)
    - [x] Verify Node.js interoperability

## Backlog
### Phase 2: Astro + Starlight MVP (Hybrid Loader + Vite)
Goal: ship Markflow as the MDX compiler for `withastro/docs` by combining the Content Layer loader with a pre-transform Vite plugin (Option 1 compiler path).
- [ ] Content Loader Glue
    - [ ] Mirror `src/content/config.ts` loader contract (glob discovery, slug generation, digest tracking)
    - [ ] Extract and store frontmatter for Starlight navigation/hero metadata in the DataStore
    - [ ] Keep non-Markflow entries compatible by reusing the default loader fallbacks
- [ ] Vite Plugin Interception
    - [ ] Publish `vite-plugin-markflow` with `enforce: 'pre'` to intercept `.mdx` before `@astrojs/mdx`
    - [ ] Call the NAPI `compile()` binding and emit JSX modules (source maps optional for MVP)
    - [ ] Verify coexistence with `@astrojs/starlight` auto-imported MDX integration
- [ ] Rust Compiler Option 1
    - [ ] YAML frontmatter extraction that emits `export const frontmatter`
    - [ ] Code-fence-aware import hoisting (state machine + multi-line support)
    - [ ] Markdown → JSX renderer with raw JSX passthrough to preserve components
    - [ ] `:::directive` → `<Aside>` transclusion plus auto-import injection
    - [ ] Heading slug generation (rehype-slug parity, including i18n) and `export const headings`
- [ ] Hydration + Validation Strategy
    - [ ] Smoke-test Tabs, FileTree, Steps, CardGrid to ensure hydration survives Option 1
    - [ ] Document known failure modes (e.g., malformed JSX) since compile-time validation is skipped
    - [ ] Provide fallback guidance for downgraded directive rendering if blockers appear
- [ ] E2E with withastro/docs
    - [ ] Wire repo fixture, run `npm run dev`, and load sample locales
    - [ ] Capture build and dev-server metrics vs. `@astrojs/mdx`
    - [ ] Log regressions and feed them back into backlog issues

### Phase 3: Universal Deployment (Nuxt & Next.js)
Goal: abstract framework-specific differences.
- [ ] Nuxt (Content v3) Support
    - [ ] Transformer implementation (MDC syntax support)
    - [ ] Serialize to Minimal Tree (JSON AST)
    - [ ] Metadata extraction (SQL storage structure)
- [ ] Next.js (App Router / Turbopack) Support
    - [ ] Implement `markflow-loader` (Webpack)
    - [ ] RSC support (JSX generation for Server Components)
    - [ ] Unify build config via `unplugin`

### Phase 4: Ecosystem & Optimization
- [ ] WASM Plugin System: load user-defined logic via WASM
- [ ] Rust-native Syntax Highlighting: integrate `syntect` or `tree-sitter`
- [ ] Turbopack Native Plugin: support the Next.js Rust plugin system

## Astro MVP Work Breakdown
Phase 1 – Environment plus N-API bridge (workspace bootstrap, napi-rs boilerplate, hello-world Vite plugin): 3 days.
Phase 2 – Core compiler pieces (frontmatter extraction, code-fence-aware hoisting, baseline Markdown-to-JSX): 5 days.
Phase 3 – Starlight-specific features (directive → Aside, slug generation, component passthrough validation): 6 days.
Phase 4 – Integration and QA (withastro/docs wiring, hydration debugging, performance measurements): 4 days.
Total – Prototype ready for docs site: approximately 18 person-days (about 3.6 weeks).

## References
Architecture reference: [docs/architecture/overview.md](./docs/architecture/overview.md)
Specification tracker: [Backlog.md](./Backlog.md)
