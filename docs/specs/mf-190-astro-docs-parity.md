# MF-190: Astro Docs Parity (Rust Renderer)

## Goal & Scope
- Reproduce Astro official docs output (core pages) with the Rust-based renderer (crates/core + N-API/WASM) within ~2.5–4 weeks.
- Target: Markdown/MDX fidelity (structure, semantics, classes/attrs), performance uplift vs JS toolchain, and CI-based regression detection.

## Phases (high-level)
1) **Prep (0.5–1d)**: Align harness with Astro docs env; ensure fixtures/integration/astro-harness builds with current deps; enumerate Astro docs MDX features (imports/JSX, custom components, frontmatter, smartypants).
2) **Diff & Harness (1–2d)**: Extend `scripts/compare-astro-harness.mjs` to produce semantic HTML diffs (normalized attrs/whitespace) for baseline (JS pipeline) vs markflow. Optional: visual check via Playwright.
3) **Impl Sprints (~3w)**:
   - Week 1: CommonMark+GFM parity; slug/heading IDs; table-driven tests.
   - Week 2: MDX/JSX + custom components (<Aside>, <Tabs>, etc.); N-API/WASM surface parity; decide N-API primary for perf, WASM as portability option.
   - Week 3: Performance + streaming rewrite (lol-html) for component remapping; run harness comparisons (≥3 runs) aiming 5–10× JS speedup.
4) **Final QA (2–3d)**: cargo test --workspace, pnpm test (napi/wasm), `node scripts/run-astro-harness.mjs markflow`, CI wiring for automatic diff on PRs.

## Requirements & Inputs
- Core: markdown-rs + GFM; MDX handling (mdxjs-rs/SWC), frontmatter; slug generation matching rehype-slug/github-slugger.
- Content features to match: Aside/FileTree/Steps/Tabs, smartypants-equivalent, autolinks, tables, task lists, raw JSX preservation.
- Fixtures: `fixtures/integration/astro-harness` as baseline playground; reuse `fixtures/core/markdown/*`, `fixtures/core/mdx/*`.
- Scripts: `scripts/compare-astro-harness.mjs` (perf/diff), `scripts/run-astro-harness.mjs` (integration).

## CI / Regression Strategy
- Keep harness compare step gated to main/develop or PR label `perf` (decision #103).
- Add semantic diff output; fail on structural mismatches; optional visual snapshots.
- N-API build requires `pnpm install` in crates/napi; smoke uses `fixtures/core/markdown/hello.md` (decision #108).

## Planned semantic diff (Step #110 spec)
- Normalization: DOM-parse both outputs; sort attributes; collapse whitespace in text nodes; drop comments.
- Diffing: traverse synchronized trees; report tag/name, attribute-set, or text mismatches; emit JSON report (empty = pass).
- CLI: add `--mode=semantic` (default keeps string diff) and `--output=<file>` to save the report; baseline/markflow generation stays unchanged.

## Open Decisions / Risks
- Smartypants equivalent in Rust pipeline (own pass vs port of remark-smartypants).
- Exact component remapping rules (class names, wrapping structure) for Starlight/withastro docs.
- Registry/network constraints for native build in CI mirrors.

## Smartypants plan (Step #112)
- Add `RewriteOptions.enable_smartypants` (default true) and a streaming pass that:
  - Converts straight quotes to curly, `--`→en dash, `---`→em dash, `...`→ellipsis.
  - Skips code blocks/inline code and raw HTML tags.
- Expose as optional flag in N-API/WASM (`enableSmartypants?: boolean`, default true).

## Component rewrite plan (Step #114 draft)
- Target components (initial set):
  - `Steps`: `<Steps><Step>…</Step></Steps>` → `<ol class="steps"><li class="steps__item">…</li>…</ol>`.
  - `Tabs`: `<Tabs><Tab title="...">…</Tab>…</Tabs>` → `div.tablist` + `button[role=tab]` + `div[role=tabpanel]` (static indexing).
  - `FileTree`: `<FileTree><File>…</File></FileTree>` → `<ul class="filetree"><li class="filetree__item">…</li>…</ul>`.
- Passthrough/skip rules:
  - Child content is preserved; unknown props kept (initially as-is; later may map to `data-*`).
  - Existing HTML/MDX structure is not altered beyond wrapper/attrs above.
- Option flag: `enable_components` (RewriteOptions default true) surfaced in N-API/WASM.
- Tests: snapshot fixtures in core + minimal N-API/WASM cases for each component type.

## Component rewrite implementation plan (Step #115)
- Core: add `transform/components.rs` implementing rewrites for Steps/Tabs/FileTree using lol-html pass after directive/hoist; gate by `enable_components`. (Aside rewrite is deferred to avoid DirectiveAdapter clash.)
- Options: add `enable_components: bool` (default true) to RewriteOptions; expose `enableComponents?: boolean` in N-API/WASM (unwrap_or(true)).
- Tests: 4 core snapshot cases + 1 on/off case each for N-API/WASM to confirm passthrough when disabled.
