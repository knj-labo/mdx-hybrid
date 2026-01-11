# Decisions: Astro Harness Reproducibility (2026-01-10)

## Step 1
- Create design doc at docs/specs/astro-harness-repro.md
- Create decision log at docs/decisions/2026-01-10-harness-repro.md
- Append each step's summary + target paths to both files.

## Step 2
- Add MARKFLOW_HARNESS_SKIP_FRONTMATTER in fixtures/integration/astro-harness/plugins/markflow-content-plugin.mjs.
- Only strip frontmatter HTML in markflow mode when the flag is enabled.
- Baseline output remains unchanged.

## Step 3
- Compare script (scripts/compare-astro-harness.mjs) forces MARKFLOW_HARNESS_SKIP_FRONTMATTER=1.
- Only affects compare runs; normal harness builds unchanged.
- No CLI override in this step.

## Step 4
- Remove multiline ESM skipping from preprocessBaseline() in fixtures/integration/astro-harness/plugins/markflow-content-plugin.mjs.
- Only skip lines that match isMdxEsmLine().
- Add a brief comment about line-based normalization.

## Step 5
- Add MARKFLOW_HARNESS_DISABLE_SMARTYPANTS in fixtures/integration/astro-harness/plugins/markflow-content-plugin.mjs.
- In markflow mode with the flag, use parseWithOptions({ enableSmartypants: false, enforceImgLoadingLazy: true }).
- Otherwise keep markflowParse(raw).

## Step 6
- Compare script (scripts/compare-astro-harness.mjs) forces MARKFLOW_HARNESS_DISABLE_SMARTYPANTS=1.
- Only affects compare runs; normal harness builds unchanged.
- No CLI override in this step.

## Step 7
- Add a "Repro flags" section to fixtures/integration/astro-harness/README.md.
- Document MARKFLOW_HARNESS_SKIP_FRONTMATTER and MARKFLOW_HARNESS_DISABLE_SMARTYPANTS.
- Note that scripts/compare-astro-harness.mjs forces both flags on.

## Step 8
- Add cleanBuildArtifacts() to scripts/compare-astro-harness.mjs for harness cache cleanup.
- Run cleanup before baseline and markflow builds.
- Keep existing dist-baseline/dist-markflow removal.

## Step 9
- Document compare-astro-harness reproducibility flags and cache cleanup in scripts/README.md.
- Clarify compare-only scope.
- Reference the harness README for details.

## Step 10
- Add scripts/visual-diff-astro-harness.mjs for full-page visual diffs via setContent + tiled capture.
- Save baseline/markflow/diff PNGs to fixtures/integration/astro-harness/visual-diff.
- Command: node scripts/visual-diff-astro-harness.mjs.
