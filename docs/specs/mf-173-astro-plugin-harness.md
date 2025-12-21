# MF-173: Astro plugin integration & harness rollout

> Note: The legacy harness scripts (`run-astro-harness.mjs`, `compare-astro-harness.mjs`) have been removed. This spec keeps the rollout ideas; implement ad-hoc harness steps if/when needed.

Status: Draft

## Goal
Plan how to wire the updated NAPI/WASM JSX renderer into the Astro plugin, and validate via the harness / withastro/docs.

## Scope
- Plugin options surface (`renderToJsx` toggle, runtime imports, layout handling).
- Harness execution plan and success criteria (build/dev, directive parity, perf smoke).
- Dependency pinning and fallback strategy (monorepo path vs published package).

## Open Questions
- Should the plugin auto-detect MDX vs MD and select JSX vs HTML output?
- How to stage rollout in withastro/docs (subset of routes vs full swap)?
- Versioning/feature-flagging strategy to avoid breaking existing users.

## Plugin Options (proposal)
- `renderToJsx` (boolean, default: false) — route Markdown through `renderToJsx` instead of HTML render.
- `runtimeImport` (string, optional) — forwarded to `render_to_jsx` options.
- `wrapLayout` (boolean, default: true) — whether to wrap with layout when frontmatter has `layout`.
- `escapeHtml` (boolean, default: true) — text escaping flag passed through.
- `useWasm` (boolean, default: false) — switch between NAPI and WASM backend.
- `trace` (boolean, default: false) — emit timing/logs for harness comparisons.

## Harness Validation Plan
- Commands: `node scripts/run-astro-harness.mjs markflow`, `node scripts/run-astro-harness.mjs baseline`, compare outputs; `node scripts/compare-astro-harness.mjs --runs=3`.
- Targets: build + dev. Pages to spot-check: home, a Tabs page, a Steps page, a code-fence-heavy page.
- Checks: build success, console errors=0, rendered diffs (HTML snapshot or `diff -u`), perf numbers (build time, dev cold start).
- Regression points: directive hydration (Tabs/FileTree/Steps), frontmatter-driven layout, auto-imported components, code fences with imports.

## withastro/docs Rollout Plan (staged)
1) **Preview subset**: apply plugin to a small folder (e.g., `/en/getting-started`), feature-flag via `renderToJsx` option.
2) **Partial rollout**: expand to one locale or section; keep baseline fallback path.
3) **Full rollout**: enable across docs; remove fallback once CI + manual checks are clean.
Fallback: flip `renderToJsx=false` and/or `useWasm=false` to revert to existing pipeline without code churn.

## Next Steps
- Implement plugin options and thread them into NAPI/WASM calls.
- Add harness scripts/docs to CI (optional) or manual checklist for contributors.
- Prepare fixture pages list and expected outputs for quick diffing.
## Next Steps
- Define plugin option defaults and expected outputs.
- Write harness checklist (commands, pages to verify, metrics to capture).
- Map rollout milestones (internal preview → partial docs → full docs).
