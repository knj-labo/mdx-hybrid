# MF-172: mdast + lol-html pipeline (hybrid)

Status: Draft

## Goal
Introduce a new rendering pipeline that builds a Markdown AST (mdast) using `markdown-rs` and then rewrites the generated HTML using `lol-html`. This pipeline should coexist with the current multipass + event renderer so we can migrate page-by-page without breaking output parity.

## Pipeline Overview
1. **Parse MD/MDX into mdast**
   - Use `markdown-rs` to parse input to mdast.
   - Preserve MDX ESM and JSX nodes.
2. **Generate HTML**
   - Serialize mdast to HTML using `markdown-rs` facilities.
   - Keep raw JSX/MDX nodes in a placeholder-safe form.
3. **Rewrite with lol-html**
   - Apply deterministic element rewrites:
     - `<Steps>`: enforce single `<ol>` root, ensure non-list fragments land inside last `<li>`.
     - `<FileTree>`: enforce single `<ul>` root; discard non-list siblings.
     - `<Aside>`: convert to `aside.starlight-aside` wrapper with title/content structure.
     - `<Tabs>`/`<AstroJSXTabs>`/`<PackageManagerTabs>`: Starlight互換の tablist + tabpanel を生成し、`Fragment slot` をタブラベルとして展開（`data-sync-key` は `syncKey` 属性から継承）。
     - `<Aside>`: ensure starlight-compatible markup when needed.
   - Allow pluggable rules for future components.
4. **Return JSX module**
   - Preserve ESM imports/exports.
   - Maintain existing `compile_ir` module generation contract.

## Coexistence Strategy
- Introduce a pipeline selector (`compiler.pipeline = "multipass" | "mdast"`).
- Default to existing multipass pipeline until mdast parity is verified.
- Provide an env toggle for docs testing (e.g. `MARKFLOW_PIPELINE=mdast`).
  - Env toggle takes precedence over config.
- When mdast parsing fails, automatically fall back to the multipass renderer to keep docs working.

## Non-Goals (Phase 1)
- Full MDX expression evaluation.
- Full parity for every Astro docs page.
- Removal of existing multipass pipeline.

## Phase 1 Implementation Note
- `render_to_html_mdast` wraps `markdown::to_html_with_options` using mdx-enabled parse options and GFM compile options.
- The generated HTML is rewritten with lol-html for `<Steps>` / `<FileTree>` normalization, then injected via `<Fragment set:html={...} />`.
- Input is normalized with the same MDX JSX indentation normalization used by the event pipeline to avoid fenced blocks being misparsed inside JSX.
- MDX expression parsing is disabled in the mdast pipeline during Phase 1 to avoid parser failures on unbalanced `{` in docs content.
- Markdown directive blocks (`:::tip`/`:::note`/`:::caution`/`:::danger`/`:::info`) are preprocessed into `<Aside>` tags before mdast parsing; code fences are excluded from this conversion.
- `<Checklist>` は mdast 変換時に `<check-list data-key="..."><div class="checklist">…</div></check-list>` に正規化する。
- Shiki 導入までは `.astro-code` のフォールバックCSSでコードブロックの可読性を確保する。
- `.astro-code` を含むページでも CSS 注入を行い、コードブロックの見た目が落ちないようにする。
- `<ReadMore>` は mdast 変換時に `<div class="read-more">…</div>` に正規化する。
- `<CardGrid>` は `<div class="card-grid">` に、`<LinkCard>` は `<a class="link-card" href="...">` に正規化する。
- mdast の CSS 注入は Starlight/Docs の公式コンポーネント CSS（Steps/FileTree/Tabs/CardGrid/LinkCard/Checklist/ReadMore）をそのまま利用する。

## Compatibility Notes
- New pipeline must not change NAPI signatures.
- Existing tests remain; new tests will be added once basic parity is reached.
