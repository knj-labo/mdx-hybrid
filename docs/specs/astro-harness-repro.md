# Astro Harness Reproducibility

## Scope
- Reduce snapshot/visual diff flakiness.
- Reduce baseline vs markflow output diffs for the top page.

## Decisions

### Step 1 (2026-01-10)
- Design doc path: docs/specs/astro-harness-repro.md
- Decision log path: docs/decisions/2026-01-10-harness-repro.md
- Each agreed step appends: step number, summary, touched paths.

### Step 2 (2026-01-10)
- Add MARKFLOW_HARNESS_SKIP_FRONTMATTER flag handling in fixtures/integration/astro-harness/plugins/markflow-content-plugin.mjs.
- When enabled and in markflow mode, strip <pre class="frontmatter">...</pre> from generated HTML.
- Baseline output remains unchanged.

### Step 3 (2026-01-10)
- In scripts/compare-astro-harness.mjs, set MARKFLOW_HARNESS_SKIP_FRONTMATTER=1 before running harness builds.
- Scope limited to compare script only; regular harness builds unchanged.
- Default is forced on (no CLI override yet).

### Step 4 (2026-01-10)
- In fixtures/integration/astro-harness/plugins/markflow-content-plugin.mjs, remove multiline ESM skipping in preprocessBaseline().
- Only skip lines that match isMdxEsmLine().
- Add a brief comment noting line-based normalization for harness stability.

### Step 5 (2026-01-10)
- Add MARKFLOW_HARNESS_DISABLE_SMARTYPANTS flag in fixtures/integration/astro-harness/plugins/markflow-content-plugin.mjs.
- In markflow mode, when enabled, use parseWithOptions with enableSmartypants: false and enforceImgLoadingLazy: true.
- Otherwise keep markflowParse(raw).

### Step 6 (2026-01-10)
- In scripts/compare-astro-harness.mjs, set MARKFLOW_HARNESS_DISABLE_SMARTYPANTS=1 before running harness builds.
- Scope limited to compare script only; normal harness builds unchanged.
- Default is forced on (no CLI override yet).

### Step 7 (2026-01-10)
- Update fixtures/integration/astro-harness/README.md with a "Repro flags" section.
- Document MARKFLOW_HARNESS_SKIP_FRONTMATTER and MARKFLOW_HARNESS_DISABLE_SMARTYPANTS.
- Note that scripts/compare-astro-harness.mjs forces both flags on.

### Step 8 (2026-01-10)
- Add cleanBuildArtifacts() to scripts/compare-astro-harness.mjs to remove dist/.astro/.vite-cache and node_modules caches under fixtures/integration/astro-harness.
- Run cleaning before baseline build and before markflow build.
- Keep existing dist-baseline/dist-markflow cleanup.

### Step 9 (2026-01-10)
- Update scripts/README.md to document compare-astro-harness reproducibility flags and cache cleanup.
- Note the behavior is compare-only and does not affect normal builds.
- Add reference to fixtures/integration/astro-harness/README.md for details.

### Step 10 (2026-01-10)
- Add scripts/visual-diff-astro-harness.mjs for full-page visual diffs via setContent and tiled capture.
- Save baseline/markflow/diff PNGs to fixtures/integration/astro-harness/visual-diff.
- Command: node scripts/visual-diff-astro-harness.mjs.
