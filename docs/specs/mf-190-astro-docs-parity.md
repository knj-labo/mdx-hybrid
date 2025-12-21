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

### Support matrix (Astro docs features vs markflow status)

| カテゴリ | 機能例 | 状態 | 備考 |
| --- | --- | --- | --- |
| CommonMark | 段落/リンク/強調/見出し | ✅ | markdown-rs core |
| GFM | 表・タスクリスト・取り消し線・自動リンク | ✅ | markdown-rs gfm extension |
| MDX 構文 | ESM import/export, JSX要素/式 | ⚠️ | mdxjs-rs/SWCでパース済み。スナップショット3ケース追加済み、さらなるカバレッジ余地 |
| Frontmatter | YAML frontmatter 抽出 | ✅ | serde_yaml 経由 |
| Slug生成 | rehype-slug互換ID | ⚠️ | 重複IDインクリメントのスナップショットを追加済み。さらなるエッジケースは未網羅 |
| Smartypants | カール引用符・en/em dash・… | ✅ | enable_smartypants デフォルトON、コード/HTML除外 |
| カスタム要素 | `<Aside>` `<Steps>` `<Tabs>` `<FileTree>` | ✅ | enable_components デフォルトON、Asideは class 正規化+title挿入 |
| 属性/クラス | Astro固有classの再現 | ⚠️ | 上記4要素以外のclass互換性未網羅 |
| Raw JSX保持 | JSX/HTMLの透過 | ✅ | 不変パスで保持 |
| 構造差分検証 | HTMLセマンティックdiff | ⚠️ | compareスクリプトに簡易正規化diffを追加（属性ソート/空白圧縮）。複雑ケースは未検証 |
| パフォーマンス計測 | build時間記録 | ✅ | compare-astro-harness.mjs が baseline/markflow を計測 |

### Astro固有クラス互換性チェックリスト

| 要素/コンポーネント | 期待クラス例 (Astro/Starlight) | 現状対応 | 優先度 |
| --- | --- | --- | --- |
| Aside (`<Aside type="note">`) | `aside aside--note`, `aside__title` | 対応済み（リライト付与） | Must (完了) |
| Steps / Step | `steps`, `steps__item` | 対応済み | Must (完了) |
| Tabs / Tab | `tabs`, `tab` (role付き) | 対応済み | Must (完了) |
| FileTree / File | `filetree`, `filetree__item` | 対応済み | Should (完了) |
| Code block wrapper | `astro-code`, `rehype-pretty-code` 系 | 未調査 | Must |
| Callout / Note / Tip variants | `note`, `tip`, `warning` 等 | 未調査 | Should |
| Table wrapper (MDX) | `table`, `table-container` 等 | 未調査 | Should |
| Blockquote modifiers | `blockquote`＋装飾class | 未調査 | Nice |
| Image / figure | `astro-image` 系や `figure` のclass | 未調査 | Nice |

次スプリント候補（実装優先案）:
- **Must:** Code block wrapper のクラス互換（syntax highlight出力を Starlight 相当に合わせる）。
- **Should:** Callout/Note系のクラス付与（既存Asideと棲み分け設計）。

### Short-term priorities (P1–P3)
- **P1-1: セマンティックdiff実装**（進行中 → v1 完了）  
  - 実装済み: `--mode=semantic` で属性ソート/空白圧縮/コメント除去の簡易diffを実装。  
  - 次ステップ: 複雑なHTML/JSXでの精度検証と改良（parse5等の採用検討）。
- **P1-2: MDX構造差分テスト拡充**（完了・継続拡張余地）  
  - 実装済み: JSX・ネスト・式混在の3ケースを insta スナップショットで追加。  
  - 次ステップ: 追加の実例（レイアウト付き/ヘッドタグ/コンポーネント属性の多いケース）でカバレッジ拡大を検討。
- **P2-1: Slug重複IDテスト**（⚠️）  
  - 成果物: 見出し重複ケースのテストを追加し、`heading-1`, `heading-2`… のインクリメントが rehype-slug 互換になることを確認。  
  - 完了条件: テストが green で、期待IDが生成される。
- **P2-2: Astro固有class互換性カバレッジ**（⚠️）  
  - 成果物: Tabs/Steps/FileTree/Aside 以外の class 付与が必要な要素を洗い出し、対応の要否を判断するチェックリストを作成。  
  - 完了条件: チェックリスト公開、必要なら次スプリントに実装タスク化。
- **P3-1: 構造差分のCI常時化検討**（依存: P1-1完了後）  
  - 成果物: compare スクリプトの semantic mode を CI opt-in ジョブへ組み込み可否を評価し、所要時間と安定性を記録。  
  - 完了条件: decision log に採否を記録（実装または見送り理由）。

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
