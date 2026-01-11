# Decision Log

## 2026-01-10: Spacing regression investigation kickoff
- Scope: withastro/docs why-astro page, compare dist-baseline vs dist-markflow DOM and CSS.
- Findings: DOM adjacency intact; rule present; markflow includes an extra CSS file; next step is in-browser computed style verification.
- Artifact: docs/notes/spacing-regression.md

## 2026-01-10: Static scan for heading-wrapper placement
- Scope: dist-markflow HTML scan for `<p>` that appear to contain `.sl-heading-wrapper` before an explicit `</p>`.
- Findings: 2668 routes matched the heuristic; results appended to notes for review.
- Artifact: docs/notes/spacing-regression.md

## 2026-01-10: Baseline vs markflow heuristic diff
- Scope: routes where markflow matches the heuristic but baseline does not.
- Findings: 2610 routes matched; results appended to notes for review.
- Artifact: docs/notes/spacing-regression.md

## 2026-01-10: Filtered candidates after excluding known wrappers
- Scope: remove matches containing expressive-code/tabs/file-tree/etc. from Step 3 list.
- Findings: 0 remaining candidates; heuristic likely over-matching valid HTML constructs.
- Artifact: docs/notes/spacing-regression.md

## 2026-01-10: Computed style check on dev server
- Scope: compare computed margin-top for `.sl-heading-wrapper` on /en/concepts/why-astro/ between baseline and markflow.
- Findings: margin-top computed to 52.5px in both modes; no regression reproduced on this page.
- Artifact: docs/notes/spacing-regression.md

## 2026-01-10: Computed style check on explicit URL request
- Scope: /en/concepts/why-astro/ with requested localhost URL (port 4321 requested, port 4322 used due to conflict).
- Findings: computed margin-top unchanged between baseline and markflow (52.5px).
- Artifact: docs/notes/spacing-regression.md

## 2026-01-10: CSS cascade inspection on live dev server
- Scope: run inspect-heading-margin on localhost:4321 to identify margin overrides on .sl-heading-wrapper.
- Findings: content-layer rules match but computed margin-top is 0px; reset-layer `* { margin: 0 }` also matches, suggesting layer ordering or cascade priority is wrong in this run.
- Artifact: docs/notes/spacing-regression.md

## 2026-01-10: Layer order inspection on live dev server
- Scope: inspect @layer statements/blocks on localhost:4321.
- Findings: explicit layer order statement exists, but first-seen layer order (from blocks) differs; need precise sheet index for statement/block ordering to confirm cascade precedence.
- Artifact: docs/notes/spacing-regression.md

## 2026-01-10: Layer order statement appears late
- Scope: inspect @layer statement/block ordering with sheet indexes.
- Findings: the explicit @layer order statement appears in sheet #21, after several layer blocks (#1-#9). This late declaration may be causing reset to override content in the live dev server.
- Artifact: docs/notes/spacing-regression.md

## 2026-01-10: Build CSS places @layer order first
- Scope: dist-markflow and dist-baseline CSS inspection.
- Findings: both build outputs have the @layer order statement at the top of the CSS bundle; the ordering issue appears limited to dev server injection.
- Artifact: docs/notes/spacing-regression.md

## 2026-01-10: Dev server style tag order shows late layer declaration
- Scope: inspect dev server head style/link ordering.
- Findings: layers.css (@layer order statement) is injected at #21, after multiple layer blocks; reset.css (#23) and markdown.css (#46) come later, with reset winning in computed styles.
- Artifact: docs/notes/spacing-regression.md

## 2026-01-10: Baseline vs markflow dev style injection order
- Scope: compare baseline dev server (port 4322) with markflow dev server (port 4321).
- Findings: baseline injects layers.css at #1 (before any layer blocks), while markflow injects layers.css at #21 after multiple layer blocks; this ordering difference aligns with reset overriding content margins only in markflow dev.
- Artifact: docs/notes/spacing-regression.md

## 2026-01-10: layers.css import lives in Starlight Page.astro
- Scope: locate layers.css import and review integration order.
- Findings: layers.css is imported in starlight Page.astro (after virtual:starlight/user-css) with a comment that it must be the first import. Baseline dev respects this; markflow dev does not, suggesting markflow alters module/style injection order.
- Artifact: docs/notes/spacing-regression.md

## 2026-01-10: markflow plugin alters module graph for Markdown
- Scope: inspect markflow Vite plugin for CSS order side effects.
- Findings: plugin is enforce: pre and replaces .md/.mdx with virtual modules, injecting component imports and compiling to JSX. This can reorder how Page.astro styles are injected in dev, even without explicit CSS imports.
- Artifact: docs/notes/spacing-regression.md

## 2026-01-10: Add dev-only @layer order injection for harness
- Scope: withastro-docs dev only, markflow enabled, harness env set.
- Decision: inject the starlight layer order at head-prepend via transformIndexHtml to prevent dev style order regression.
- Artifact: fixtures/integration/withastro-docs/repo/astro.config.ts

## 2026-01-10: Dev layer fix validated
- Scope: markflow dev server with layer-order injection enabled.
- Findings: layers.css now injected at #1 and computed heading margin-top returns to 52.5px on /en/concepts/why-astro/.
- Artifact: docs/notes/spacing-regression.md

## 2026-01-10: Move dev layer-order fix into markflow plugin (harness-only)
- Scope: markflow Vite plugin.
- Decision: inject the starlight layer order via transformIndexHtml when MARKFLOW_HARNESS_* is set and Vite is running in serve mode.
- Artifact: packages/vite-plugin-markflow/src/index.js
