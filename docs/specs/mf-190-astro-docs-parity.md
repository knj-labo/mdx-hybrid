# MF-190: Astro Docs Parity (Rust Renderer)

> Note: Harness scripts (`scripts/run-astro-harness.mjs`, `scripts/compare-astro-harness.mjs`) are back in a minimal form (build + time only, no HTML diff). CI では perf ラベルか workflow_dispatch で opt-in 実行。

## Goal & Scope
- Reproduce Astro official docs output (core pages) with the Rust-based renderer (crates/core + N-API/WASM) within ~2.5–4 weeks.
- Target: Markdown/MDX fidelity (structure, semantics, classes/attrs), performance uplift vs JS toolchain, and CI-based regression detection.

## Phases (high-level)
1) **Prep (0.5–1d)**: fixtures/integration/astro-harness を使用し、依存を合わせる。公式ハーネス比較は opt-in 運用。
2) **Diff & Harness (1–2d)**: 現行 compare スクリプトはビルド時間のみ記録。セマンティック diff が必要な場合はカスタムツールを追加する。
3) **Impl Sprints (~3w)**:
   - Week 1: CommonMark+GFM parity; slug/heading IDs; table-driven tests.
   - Week 2: MDX/JSX + custom components (<Aside>, <Tabs>, etc.); N-API/WASM surface parity; decide N-API primary for perf, WASM as portability option.
   - Week 3: Performance + streaming rewrite (lol-html) for component remapping; run harness comparisons (≥3 runs) aiming 5–10× JS speedup.
4) **Final QA (2–3d)**: cargo test --workspace, pnpm test (napi/wasm); harness自動化は撤去済みのため、必要に応じて手動チェック。

## Requirements & Inputs
- Core: markdown-rs + GFM; MDX handling (mdxjs-rs/SWC), frontmatter; slug generation matching rehype-slug/github-slugger.
- Content features to match: Aside/FileTree/Steps/Tabs, smartypants-equivalent, autolinks, tables, task lists, raw JSX preservation.
- Fixtures: `fixtures/integration/astro-harness` はプレイグラウンド。`fixtures/core/markdown/*`, `fixtures/core/mdx/*` を再利用。
- Scripts: compare/run スクリプトは build 時間のみ対応。ベンチや構造 diff が必要なら拡張する。

## CI / Regression Strategy
- ハーネス比較は opt-in（perf ラベルの PR または workflow_dispatch 入力 `harness=true`）。常時は走らない。
- セマンティック diff が必要なら compare スクリプトへ拡張し、構造差分のみで落とす方針にする。
- N-API build requires `pnpm install` in crates/napi; smoke uses `fixtures/core/markdown/hello.md` (decision #108).

## Planned semantic diff (not yet re-implemented)
- Normalization: DOM-parse both outputs; sort attributes; collapse whitespace in text nodes; drop comments.
- Diffing: traverse synchronized trees; report tag/name, attribute-set, or text mismatches; emit JSON report (empty = pass).
- 現行 compare スクリプトはビルド時間のみ。セマンティック diff を再導入する場合はここを実装ガイドとする。

## Open Decisions / Risks
- Smartypants equivalent in Rust pipeline (own pass vs port of remark-smartypants).
- Exact component remapping rules (class names, wrapping structure) for Starlight/withastro docs.
- Registry/network constraints for native build in CI mirrors.

## Smartypants plan (Step #112)
- Add `RewriteOptions.enable_smartypants` (default true) and a streaming pass that:
  - Converts straight quotes to curly, `--`→en dash, `---`→em dash, `...`→ellipsis.
  - Skips code blocks/inline code and raw HTML tags.
- Expose as optional flag in N-API/WASM (`enableSmartypants?: boolean`, default true).

## Component rewrite (implemented)
- Target components (current set):
  - `Aside`: normalized to `<aside class="aside aside--{type}">…</aside>`; when `title` exists, prepend `<div class="aside__title">{title}</div>`; remove `type/title/data-mf-source`, merge existing `class` into the class list. DirectiveAdapter emits `data-mf-source="directive"` for provenance before this pass.
  - `Steps`: `<Steps><Step>…</Step></Steps>` → `<ol class="steps"><li class="steps__item">…</li>…</ol>`.
  - `Tabs`: `<Tabs><Tab title="...">…</Tab>…</Tabs>` → `<div class="tabs" role="tablist"><div class="tab" role="tabpanel">…</div>…</div>` with inline title heading.
  - `FileTree`: `<FileTree><File>…</File></FileTree>` → `<ul class="filetree"><li class="filetree__item">…</li>…</ul>`.
- Passthrough/skip rules:
  - Child content is preserved; unknown props stay (Aside drops type/title/data-mf-source after consuming; others keep unknown attrs).
  - Existing HTML/MDX structure is untouched aside from wrapper/attrs above.
- Option flag: `enable_components` (RewriteOptions default true) surfaced in N-API/WASM; when false, all component rewrites are skipped.
- Tests: core snapshot fixtures for Tabs/Steps/FileTree/Aside with enable_components on/off; insta directive snapshot updated to normalized Aside output.
