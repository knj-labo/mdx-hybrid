# 0002 Draft PR Submission for mdast Renderer

## Status
**Draft** - Submitted 2026-01-05

## Context

We have implemented a new mdast (Markdown AST) renderer with significant features for Starlight documentation support:

### Features Implemented
- **mdast renderer** (`crates/core/src/renderer/mdast_renderer.rs`, 1,592 lines)
  - HTML post-processing pipeline using lol_html
  - Starlight component auto-injection (Aside, Tabs, Steps, FileTree, CardGrid, LinkCard)
  - Directive preprocessing (`:::tip`, `:::note`, `:::caution`, etc.)
  - Component CSS styling (`starlight_components.css`)

- **Visual diff tooling** for validation
  - `scripts/compare-withastro-docs.mjs` - Semantic HTML diffing
  - `scripts/visual-diff-withastro-docs.mjs` - Screenshot regression testing with Playwright
  - `fixtures/integration/withastro-docs/` - Test harness

- **Vite plugin integration** (`packages/vite-plugin-markflow/src/index.js`, 437 lines changed)
  - Pipeline selection (multipass vs mdast)
  - Component auto-injection system
  - Fallback handling

- **CI/CD updates** (`.github/workflows/ci.yml`)
  - Optional harness testing with 'perf' label

### Critical Issue Discovered

**Visual diff testing reveals a rendering bug**: The mdast renderer outputs raw markdown syntax wrapped in `<pre><code>` tags instead of properly rendered HTML elements.

**Failure rate**: 28/40 routes (70%) failing in withastro/docs semantic diff

**Examples**:
```html
<!-- Expected (baseline) -->
<p>Explore <a href="https://astro.build/themes/">Astro starter themes</a> for blogs...</p>

<!-- Actual (markflow) -->
<pre><code>Explore [Astro starter themes](https://astro.build/themes/) for blogs...</code></pre>
```

**Affected elements**:
- Paragraphs with inline links → `[text](url)` in `<pre><code>`
- Unordered lists → `- item` in `<pre><code>`
- Emphasis/strong text → `**text**` in `<pre><code>`
- Code blocks with expressive-code components

### Root Cause (Under Investigation)

The issue appears to be in the HTML rewriting or Fragment serialization pipeline:

**Potential locations**:
- `crates/core/src/renderer/mdast_renderer.rs`
  - `rewrite_mdast_html()` function (lines 47-49)
  - `MdxProcessor::process()` pipeline (lines 156-361)
  - Component normalization functions (e.g., `normalize_tabs_html()`, `normalize_aside_html()`)
  - Fragment serialization with `serde_json::to_string()` (line 43)

- `packages/vite-plugin-markflow/src/index.js`
  - Component injection logic may interfere with markdown rendering
  - Pipeline routing between mdast and multipass

**Investigation needed**:
1. Trace markdown → mdast → HTML → rewrite → Fragment pipeline
2. Check if HTML is being double-escaped before JSON serialization
3. Verify lol_html RewriteStrSettings configuration
4. Review how Fragment component receives HTML content

## Decision

**Submit as draft PR with known issues documented** for the following reasons:

1. **Early architectural feedback**: The mdast renderer introduces significant architectural changes that benefit from early review
2. **Visual diff tooling is valuable**: Even with rendering issues, the tooling infrastructure is complete and useful
3. **Incremental development**: Draft PR allows continued work while gathering feedback
4. **Known issue isolation**: The rendering bug is isolated and documented, not blocking review of overall architecture

## Consequences

### Positive
- ✅ Early feedback on mdast renderer architecture and component auto-injection strategy
- ✅ Visual diff tooling available for future testing
- ✅ CI/CD infrastructure in place for optional performance testing
- ✅ Clear documentation of known issues for future debugging

### Negative
- ❌ Cannot merge to main until rendering bug is fixed
- ❌ 70% of withastro/docs routes fail visual diff (28/40)
- ❌ Additional iteration needed to resolve `<pre><code>` wrapping issue

### Next Steps (Post-PR Submission)
1. Debug mdast_renderer.rs to identify where markdown becomes `<pre><code>`
2. Fix export hoisting test failures (2 failing NAPI tests - pre-existing issue)
   - `tests::compile_document_hoists_export_edge_cases` - "export *" not hoisted
   - `tests::compile_document_hoists_exports_variants` - default export not hoisted
3. Add unit tests for component rendering in `crates/core/tests`
4. Verify snapshot test expectations with insta
5. Re-run visual diff after fixes: `pnpm compare:withastro-docs -- --mode=semantic`

### Additional Known Issues

**NAPI Test Failures** (Pre-existing, 2/11 tests failing):
- `tests::compile_document_hoists_export_edge_cases`: Export * from statement not being hoisted before JSX content
- `tests::compile_document_hoists_exports_variants`: Default async export not being hoisted correctly

These failures existed before the current changes and are unrelated to the mdast renderer work.

---

## PR Preparation Checklist

### Pre-Submission Requirements

#### Code Quality
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] No new clippy warnings introduced

#### Tests
- [ ] `cargo test --workspace` passes (all Rust tests)
- [ ] `pnpm --dir crates/napi test` passes (N-API smoke tests)
- [ ] No snapshot test regressions (check insta snapshots in `crates/core/tests`)

#### Build Success
- [ ] `cargo build` succeeds
- [ ] `pnpm --dir crates/napi build` succeeds
- [ ] All binaries compile without errors

#### Repository Cleanup
- [ ] Debug files removed or gitignored:
  - `raw_baseline.html` (2,256 lines)
  - `raw_markflow.html` (2,225 lines)
  - `diff_baseline.html`
  - `diff_markflow.html`
- [ ] `.gitignore` updated with patterns: `raw_*.html`, `diff_*.html`
- [ ] Optional: Keep `scripts/debug-diff.mjs` for future investigations

#### Commit Structure
- [ ] All WIP commits squashed into conventional commits
- [ ] Target structure (3-4 clean commits):
  1. `feat: Implement mdast renderer with Starlight component support`
  2. `feat: Add visual diff tooling for withastro/docs validation`
  3. `chore: Update CI workflows and documentation`
  4. `chore: apply cargo fmt formatting` (already clean)
- [ ] No "wip" in commit messages
- [ ] Commit messages follow conventional format (from `AGENTS.md`)

#### Documentation
- [ ] This ADR (0002-draft-pr-submission.md) created
- [ ] `AGENTS.md` updated with new scripts:
  - `pnpm compare:withastro-docs -- --mode=semantic`
  - `pnpm visual:withastro-docs -- --build`
- [ ] `docs/specs/mf-172-mdast-lolhtml-pipeline.md` reflects current implementation

#### Security
- [ ] `git diff main --check` shows no whitespace errors
- [ ] No .env files or secrets accidentally committed
- [ ] `fixtures/integration/withastro-docs/.gitignore` properly configured (repo/ excluded)

### Harness Validation (Recommended)

#### Semantic Diff
```bash
pnpm compare:withastro-docs -- --mode=semantic --skip-install
```
Expected: 28/40 routes fail (documented)

#### Visual Diff (Small Subset)
```bash
pnpm visual:withastro-docs -- --max 5
```
Screenshots saved to `fixtures/integration/withastro-docs/visual-diff/`

#### Smoke Test
```bash
pnpm --dir crates/napi smoke:napi
```
Validates N-API binary works with basic fixtures

### PR Description Template

**Title**: `[WIP] feat: Add mdast renderer with Starlight component support`

**Summary**:
This PR introduces a new mdast (markdown AST) rendering pipeline optimized for Astro Starlight documentation sites, alongside visual diff tooling for validating output against the official withastro/docs repository.

Key additions:
- **New mdast renderer** with lol-html-based rewriting and Starlight component auto-injection
- **Component auto-discovery** for Aside, Tabs, Steps, FileTree, CardGrid, LinkCard
- **Directive preprocessing** (`:::tip`, `:::note`, `:::warning`) with CSS fallback styling
- **Visual diff harness** for end-to-end validation
- **Optional CI workflows** for performance and semantic regression testing

**Known Issues**: See `docs/decisions/0002-draft-pr-submission.md`

**CRITICAL**: Rendering bug - mdast renderer outputs raw markdown in `<pre><code>` instead of HTML. Impacts 28/40 routes (70%).

**Test Plan**:
```bash
# Unit tests
cargo test --workspace
pnpm --dir crates/napi test

# Code quality
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

# Harness (optional)
pnpm compare:withastro-docs -- --mode=semantic
pnpm visual:withastro-docs -- --build --max 10
```

**Files Changed**: 36 files, 9,033 insertions(+), 199 deletions(-)

**Discussion Points**:
1. Should rendering bug be fixed before merge?
2. Component auto-injection strategy: opt-in vs always-on?
3. Performance characteristics vs JSX renderer path?

**Labels**: `wip`, `feat`, `mdast-renderer`

### Post-Submission

- [ ] Monitor CI/CD pipeline
- [ ] Enable 'perf' label for optional harness testing if needed
- [ ] Document rendering bug investigation progress in PR comments
- [ ] Link related issues if applicable

---

## Files Involved

### Core Renderer
- `crates/core/src/renderer/mdast_renderer.rs` - NEW (1,592 lines)
- `crates/core/src/renderer/starlight_components.css` - NEW (419 lines)
- `crates/core/src/renderer/mod.rs` - Updated exports
- `crates/core/src/renderer/multipass.rs` - Updated (148 lines)

### Vite Plugin
- `packages/vite-plugin-markflow/src/index.js` - Major update (437 lines)

### NAPI Bindings
- `crates/napi/src/compiler.rs` - Updated with renderer routing (94 lines)
- `crates/napi/src/headings.rs` - Updated (19 lines)
- `crates/napi/src/types.rs` - Updated (2 lines)

### Testing & Harness
- `scripts/compare-withastro-docs.mjs` - NEW (495 lines)
- `scripts/visual-diff-withastro-docs.mjs` - NEW (597 lines)
- `scripts/debug-diff.mjs` - NEW (28 lines, optional)
- `fixtures/integration/withastro-docs/` - NEW fixture directory

### Documentation
- `docs/astro-docs-catalog.md` - NEW element catalog (66 lines)
- `docs/specs/mf-172-mdast-lolhtml-pipeline.md` - NEW spec (54 lines)
- `docs/decisions/0001-lean-architecture.md` - NEW (39 lines)
- `docs/decisions/0002-draft-pr-submission.md` - THIS FILE
- `AGENTS.md` - Updated with new scripts (41 lines)

### CI/CD
- `.github/workflows/ci.yml` - Added optional harness jobs (53 lines)

**Total: 36 files changed, 9,033 insertions(+), 199 deletions(-)**

---

## References

- Visual diff results: `fixtures/integration/withastro-docs/visual-diff/summary.json`
- Raw HTML comparison: `raw_baseline.html` vs `raw_markflow.html` (to be removed before merge)
- Semantic routes tested: `fixtures/integration/withastro-docs/semantic-routes.txt`
